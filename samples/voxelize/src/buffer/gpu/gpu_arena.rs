use dashmap::DashMap;
use std::{ops::Deref, sync::Arc};
use url::Url;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt, TextureDataOrder},
    BufferAddress, BufferDescriptor, BufferUsages, TextureDescriptor,
};

use crate::buffer::{
    BufferCache, BufferCacheData, BufferCacheId, GpuBufferAccessor, GpuBufferView, GpuImage,
};

pub struct GpuBufferCache {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    buffers: DashMap<Url, Arc<wgpu::Buffer>>,
    textures: DashMap<Url, Arc<wgpu::Texture>>,
}

impl GpuBufferCache {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        Self {
            device,
            queue,
            buffers: Default::default(),
            textures: Default::default(),
        }
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn get_buf<'buf>(
        &'buf self,
        key: &Url,
    ) -> Option<impl Deref<Target = Arc<wgpu::Buffer>> + 'buf> {
        self.buffers.get(key)
    }

    pub fn get_tex<'buf>(
        &'buf self,
        key: &Url,
    ) -> Option<impl Deref<Target = Arc<wgpu::Texture>> + 'buf> {
        self.textures.get(key)
    }

    /// Create & initialize a new buffer on the GPU and return a reference to it.
    pub fn create_init(&self, key: Url, desc: &BufferInitDescriptor) -> Arc<wgpu::Buffer> {
        let buf = Arc::new(self.device.create_buffer_init(desc));
        if self.buffers.insert(key.clone(), buf.clone()).is_some() {
            tracing::warn!(%key, "replaced existing gpu buffer in cache");
        }
        buf
    }

    /// Create a new buffer on the GPU and return a reference to it.
    pub fn create(&self, key: Url, desc: &BufferDescriptor) -> Arc<wgpu::Buffer> {
        let buf = Arc::new(self.device.create_buffer(desc));
        if self.buffers.insert(key.clone(), buf.clone()).is_some() {
            tracing::warn!(%key, "replaced existing gpu buffer in cache");
        }
        buf
    }

    pub fn create_texture(&self, key: Url, desc: &TextureDescriptor) -> Arc<wgpu::Texture> {
        let tex = Arc::new(self.device.create_texture(desc));
        if self.textures.insert(key.clone(), tex.clone()).is_some() {
            tracing::warn!(%key, "replaced existing gpu texture in cache");
        }
        tex
    }

    pub fn create_texture_with_data(
        &self,
        key: Url,
        desc: &TextureDescriptor,
        order: TextureDataOrder,
        data: &[u8],
    ) -> Arc<wgpu::Texture> {
        let tex = Arc::new(
            self.device
                .create_texture_with_data(&*self.queue, desc, order, data),
        );
        if self.textures.insert(key.clone(), tex.clone()).is_some() {
            tracing::warn!(%key, "replaced existing gpu texture in cache");
        }
        tex
    }

    pub fn load_attribute(
        &self,
        doc_cache: &BufferCache<'_>,
        acc: &gltf::Accessor,
        mut usage: BufferUsages,
    ) -> GpuBufferAccessor {
        let view = acc.view().expect("non-sparse accessor");
        let key = doc_cache.source_url(view.buffer().source()).unwrap();
        // FIX :: Avoid reloading GPU buffers
        if let Some(buf) = self.get_buf(&key) {
            let b_usage = buf.usage();
            if b_usage.contains(usage) {
                return GpuBufferAccessor::from_gltf(buf.clone(), acc);
            }
            tracing::warn!("reloading gpu buffer to fix usage");
            usage |= b_usage;
        }

        GpuBufferAccessor::from_gltf(
            self.create_init(
                key.clone(),
                &BufferInitDescriptor {
                    label: Some(key.as_str()),
                    contents: &doc_cache.access(acc).unwrap().buffer,
                    usage,
                },
            ),
            acc,
        )
    }

    pub fn load_texture(
        &self,
        doc_cache: &BufferCache<'_>,
        image: &gltf::Image<'_>,
    ) -> Arc<wgpu::Texture> {
        let key = doc_cache.image_source_url(&image.source()).unwrap();
        if let Some(tex) = self.get_tex(&key) {
            return tex.clone();
        }
        GpuImage::create_with_gltf(self, doc_cache, image).buffer
    }

    pub fn view_buf(
        &self,
        key: &Url,
        length: BufferAddress,
        offset: BufferAddress,
        stride: Option<BufferAddress>,
    ) -> Option<GpuBufferView> {
        self.get_buf(key)
            .map(|buffer| GpuBufferView::new(buffer.clone(), length, offset, stride))
    }
}
