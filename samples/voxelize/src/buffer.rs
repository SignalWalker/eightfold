use core::slice;
use crossbeam::sync::ShardedLock;
use gltf::Gltf;
use memmap2::Mmap;
use std::{
    collections::HashMap,
    fs::{self, File},
    io,
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};
use url::Url;

mod accessor;
pub use accessor::*;

mod gpu;
pub use gpu::*;

/// Errors related to [`BufferCaches`](BufferCache).
#[derive(Debug, thiserror::Error)]
pub enum BufferError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("expected file, found directory: {0:?}")]
    IsADirectory(PathBuf), // TODO :: use `std::io::IsADirectory` once that's stabilized
    #[error(transparent)]
    Url(#[from] url::ParseError),
    #[error("expected local file path; instead, found: {0:?}")]
    UnsupportedUriScheme(String),
    #[error("attempted to access binary blob in document without one")]
    DocumentDoesNotIncludeBinaryBlob,
    #[error(transparent)]
    Accessor(#[from] AccessorError),
    #[error(transparent)]
    View(#[from] ViewError),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum BufferCacheId {
    Blob,
    Url(Url),
}

impl From<Url> for BufferCacheId {
    fn from(value: Url) -> Self {
        Self::Url(value)
    }
}

#[derive(Debug)]
pub struct BufferFile {
    _file: File,
    data: Option<Mmap>,
}

impl BufferFile {
    #[allow(unsafe_code)]
    pub fn new(path: impl AsRef<Path>) -> Result<Self, io::Error> {
        tracing::trace!(
            path = path.as_ref().as_os_str().to_str(),
            "memory-mapping glTF buffer"
        );
        let file = File::options().read(true).write(false).open(path)?;
        Ok(Self {
            data: Some(unsafe { Mmap::map(&file)? }),
            _file: file,
        })
    }
}

impl Drop for BufferFile {
    fn drop(&mut self) {
        self.data.take(); // the mmap must be dropped before we close its associated file
    }
}

impl Deref for BufferFile {
    type Target = [u8];
    #[allow(unsafe_code)]
    fn deref(&self) -> &Self::Target {
        unsafe { self.data.as_ref().unwrap_unchecked() }
    }
}

#[derive(Debug)]
pub enum BufferCacheData<'doc> {
    Blob(&'doc [u8]),
    File(BufferFile),
    Owned(Vec<u8>),
}

impl<'doc> From<&'doc [u8]> for BufferCacheData<'doc> {
    fn from(value: &'doc [u8]) -> Self {
        Self::Blob(value)
    }
}

impl<'doc> BufferCacheData<'doc> {
    pub fn map_path(path: impl AsRef<Path>) -> Result<Self, io::Error> {
        BufferFile::new(path).map(Self::File)
    }

    pub fn read_path(path: impl AsRef<Path>) -> Result<Self, io::Error> {
        fs::read(path).map(Self::Owned)
    }
}

impl<'doc> Deref for BufferCacheData<'doc> {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Blob(data) => data,
            Self::File(file) => file,
            Self::Owned(data) => data,
        }
    }
}

/// Cache `glTF` buffer data.
#[derive(Debug)]
pub struct BufferCache<'doc> {
    doc: &'doc Gltf,
    /// Path to the glTF document. Used when resolving relative URLs.
    src_path: Url,
    data: ShardedLock<HashMap<BufferCacheId, Arc<BufferCacheData<'doc>>>>,
}

impl<'doc> BufferCache<'doc> {
    pub fn new(doc: &'doc Gltf, src_path: impl AsRef<Path>) -> Result<Self, BufferError> {
        let src_path: PathBuf = src_path.as_ref().canonicalize()?;
        if src_path.is_dir() {
            return Err(BufferError::IsADirectory(src_path));
        }
        Ok(Self {
            doc,
            data: match doc.blob {
                Some(ref b) => ShardedLock::new(HashMap::from([(
                    BufferCacheId::Blob,
                    Arc::new(BufferCacheData::from(b.as_slice())),
                )])),
                None => ShardedLock::default(),
            },
            src_path: Url::from_file_path(src_path).unwrap(), // this function only fails if the
                                                              // input path is not absolute -- we
                                                              // already canonicalized it, so it's
                                                              // fine
        })
    }

    /// Parse a [`gltf::buffer::Source`] relative to the path to the glTF document.
    pub fn parse_source(
        &self,
        src: gltf::buffer::Source,
    ) -> Result<BufferCacheId, url::ParseError> {
        match src {
            gltf::buffer::Source::Bin => Ok(BufferCacheId::Blob),
            gltf::buffer::Source::Uri(u) => url::Url::options()
                .base_url(Some(&self.src_path))
                .parse(u)
                .map(BufferCacheId::Url),
        }
    }

    pub fn source_url(&self, src: gltf::buffer::Source) -> Result<url::Url, url::ParseError> {
        match self.parse_source(src)? {
            BufferCacheId::Blob => Ok(self.src_path.clone()),
            BufferCacheId::Url(url) => Ok(url),
        }
    }

    pub fn image_source_url(&self, src: &gltf::image::Source) -> Result<url::Url, url::ParseError> {
        match src {
            gltf::image::Source::View { view, .. } => self.source_url(view.buffer().source()),
            gltf::image::Source::Uri { uri, .. } => url::Url::options()
                .base_url(Some(&self.src_path))
                .parse(uri),
        }
    }

    #[tracing::instrument(skip(self), fields(src_url = self.src_path.as_str(), uri = uri.as_ref()))]
    pub fn load(&self, uri: impl AsRef<str>) -> Result<Arc<BufferCacheData<'doc>>, BufferError> {
        let uri = url::Url::options()
            .base_url(Some(&self.src_path))
            .parse(uri.as_ref())?;
        if uri.scheme() != "file" {
            return Err(BufferError::UnsupportedUriScheme(uri.scheme().to_owned()));
        }
        let data_key = BufferCacheId::from(uri.clone());

        if let Some(data) = self.data.read().unwrap().get(&data_key).cloned() {
            tracing::trace!(url = uri.as_str(), "already loaded glTF buffer");
            return Ok(data);
        }

        tracing::debug!(url = uri.as_str(), "loading glTF buffer");
        let path = uri.to_file_path().unwrap();
        let data = Arc::new(BufferCacheData::map_path(path)?);
        self.data.write().unwrap().insert(data_key, data.clone());
        Ok(data)
    }

    pub fn load_gltf_source(
        &self,
        source: &gltf::buffer::Source<'_>,
    ) -> Result<Arc<BufferCacheData<'doc>>, BufferError> {
        match source {
            gltf::buffer::Source::Bin => self
                .data
                .read()
                .unwrap()
                .get(&BufferCacheId::Blob)
                .cloned()
                .ok_or(BufferError::DocumentDoesNotIncludeBinaryBlob),
            gltf::buffer::Source::Uri(uri) => self.load(uri),
        }
    }

    pub fn load_gltf_image(
        &self,
        image: &gltf::Image<'_>,
    ) -> Result<Arc<BufferCacheData<'doc>>, BufferError> {
        match image.source() {
            gltf::image::Source::View { view, .. } => self.load_gltf_buffer(&view.buffer()),
            gltf::image::Source::Uri { uri, .. } => self.load(uri),
        }
    }

    pub fn load_gltf_buffer(
        &self,
        buffer: &gltf::Buffer<'_>,
    ) -> Result<Arc<BufferCacheData<'doc>>, BufferError> {
        self.load_gltf_source(&buffer.source())
    }

    pub fn access(&self, acc: &gltf::Accessor<'_>) -> Result<BufferAccessor<'_>, BufferError> {
        tracing::trace!(
            acc_index = acc.index(),
            acc_data_type = format!("{:?}", acc.data_type(),),
            acc_dimensions = format!("{:?}", acc.dimensions(),),
            acc_count = acc.count(),
            acc_name = acc.name(),
            "accessing data buffer"
        );
        BufferAccessor::new(
            self.load_gltf_buffer(
                &acc.view()
                    .expect("expected accessor to reference data buffer")
                    .buffer(),
            )?,
            acc,
        )
        .map_err(BufferError::from)
    }

    pub fn view(&self, view: &gltf::buffer::View<'_>) -> Result<BufferView<'_>, BufferError> {
        BufferView::new(self.load_gltf_buffer(&view.buffer())?, view).map_err(BufferError::from)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ViewError {
    #[error("views with non-zero stride are unsupported")]
    Stride,
}

pub struct BufferView<'buf> {
    pub(crate) buffer: Arc<BufferCacheData<'buf>>,
    pub(crate) length: usize,
    pub(crate) offset: usize,
    pub(crate) stride: Option<usize>,
}

impl<'buf> BufferView<'buf> {
    pub(crate) fn new(
        buffer: Arc<BufferCacheData<'buf>>,
        view: &gltf::buffer::View<'_>,
    ) -> Result<Self, ViewError> {
        if view.stride().is_some() {
            return Err(ViewError::Stride);
        }
        Ok(Self {
            buffer,
            length: view.length(),
            offset: view.offset(),
            stride: view.stride(),
        })
    }

    pub fn as_bytes(&self) -> &'buf [u8] {
        unsafe {
            slice::from_raw_parts(
                self.buffer.as_ptr().add(self.offset).cast::<u8>(),
                self.length,
            )
        }
    }
}
