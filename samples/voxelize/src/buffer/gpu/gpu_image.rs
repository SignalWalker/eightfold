use std::{io::Cursor, num::NonZeroU32, sync::Arc};

use gltf::texture::{MagFilter, MinFilter, WrappingMode};
use image::{DynamicImage, GenericImageView, ImageFormat, ImageReader};
use wgpu::{
    AddressMode, BindGroup, BindGroupEntry, FilterMode, SamplerDescriptor, TextureDescriptor,
    TextureFormat, TextureUsages,
};

use crate::buffer::{BufferCache, BufferView, GpuBufferCache, GpuBufferView};

fn detect_format(url: Option<&str>, mime_type: Option<&str>) -> Option<ImageFormat> {
    match mime_type {
        Some("image/png") => Some(ImageFormat::Png),
        Some("image/jpeg") => Some(ImageFormat::Jpeg),
        // TODO :: others
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub struct GpuImage {
    pub buffer: Arc<wgpu::Texture>,
}

impl GpuImage {
    pub fn create_with_gltf(
        buf_cache: &GpuBufferCache,
        doc_cache: &BufferCache<'_>,
        image: &gltf::Image<'_>,
    ) -> Self {
        let buf = doc_cache.load_gltf_image(image).unwrap();
        let key = doc_cache.image_source_url(&image.source()).unwrap();
        let (fmt, data) = match image.source() {
            gltf::image::Source::View { mime_type, view } => (
                detect_format(None, Some(mime_type)),
                BufferView::new(buf, &view).unwrap().as_bytes(),
            ),
            gltf::image::Source::Uri { uri, mime_type } => {
                (detect_format(Some(uri), mime_type), &buf[..])
            }
        };
        let img = if let Some(fmt) = fmt {
            ImageReader::with_format(Cursor::new(data), fmt)
        } else {
            ImageReader::new(Cursor::new(data))
                .with_guessed_format()
                .unwrap()
        }
        .decode()
        .unwrap();
        let dims = img.dimensions();
        let img_bytes = img.to_rgba8();
        let buffer = buf_cache.create_texture_with_data(
            key.clone(),
            &TextureDescriptor {
                label: Some(key.as_str()),
                size: wgpu::Extent3d {
                    width: dims.0,
                    height: dims.1,
                    depth_or_array_layers: 1,
                },
                // TODO :: Mip levels
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                // FIX :: programmatically determine normalization
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &img_bytes,
        );
        Self { buffer }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct GpuTexture {
    pub image: Arc<wgpu::Texture>,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl GpuTexture {
    pub fn from_gltf(
        device: &wgpu::Device,
        image: Arc<wgpu::Texture>,
        tex: &gltf::Texture<'_>,
        label: Option<&str>,
    ) -> Self {
        fn wrap_gltf_to_wgpu(mode: WrappingMode) -> wgpu::AddressMode {
            match mode {
                WrappingMode::ClampToEdge => AddressMode::ClampToEdge,
                WrappingMode::MirroredRepeat => AddressMode::MirrorRepeat,
                WrappingMode::Repeat => AddressMode::Repeat,
            }
        }
        let samp = tex.sampler();
        let view = image.create_view(&wgpu::TextureViewDescriptor {
            label: label.map(|l| format!("{l} view")).as_deref(),
            ..Default::default()
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: label.map(|l| format!("{l} sampler")).as_deref(),
            address_mode_u: wrap_gltf_to_wgpu(samp.wrap_t()),
            address_mode_v: wrap_gltf_to_wgpu(samp.wrap_s()),
            mag_filter: match samp.mag_filter() {
                Some(MagFilter::Linear) => FilterMode::Linear,
                Some(MagFilter::Nearest) => FilterMode::Nearest,
                None => FilterMode::Nearest,
            },
            min_filter: match samp.min_filter() {
                Some(
                    MinFilter::Linear
                    | MinFilter::LinearMipmapLinear
                    | MinFilter::LinearMipmapNearest,
                ) => FilterMode::Linear,
                Some(
                    MinFilter::Nearest
                    | MinFilter::NearestMipmapLinear
                    | MinFilter::NearestMipmapNearest,
                ) => FilterMode::Nearest,
                None => FilterMode::Nearest,
            },
            mipmap_filter: match samp.min_filter() {
                Some(MinFilter::LinearMipmapLinear | MinFilter::NearestMipmapLinear) => {
                    FilterMode::Linear
                }
                Some(MinFilter::LinearMipmapNearest | MinFilter::NearestMipmapNearest) => {
                    FilterMode::Nearest
                }
                _ => FilterMode::Nearest,
            },
            anisotropy_clamp: 1,
            ..Default::default()
        });
        Self {
            image,
            view,
            sampler,
        }
    }

    pub fn new_depth_texture(device: &wgpu::Device, width: NonZeroU32, height: NonZeroU32) -> Self {
        let size = wgpu::Extent3d {
            width: width.into(),
            height: height.into(),
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // FIX :: don't hardcode depth tex format?
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let image = device.create_texture(&desc);
        let view = image.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Depth Texture Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        Self {
            image: Arc::new(image),
            view,
            sampler,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TextureSampler {
    pub mag_filter: Option<MagFilter>,
    pub min_filter: Option<MinFilter>,
    pub wrap_s: WrappingMode,
    pub wrap_t: WrappingMode,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct BoundGpuTexture {
    pub texture: GpuTexture,
    pub bind_group: BindGroup,
}

impl BoundGpuTexture {
    pub fn from_gltf(
        device: &wgpu::Device,
        image: Arc<wgpu::Texture>,
        tex: &gltf::Texture<'_>,
        bind_group_layout: &wgpu::BindGroupLayout,
        label: Option<&str>,
    ) -> Self {
        let texture = GpuTexture::from_gltf(device, image, tex, label);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: label.map(|l| format!("{l} bind group")).as_deref(),
            layout: bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        });

        Self {
            texture,
            bind_group,
        }
    }
}
