use gltf::material::AlphaMode;
use nalgebra::Vector4;

use crate::buffer::{BoundGpuTexture, BufferCache, GpuBufferCache};

#[derive(Debug)]
pub struct PbrMetallicRoughness {
    pub base_color_factor: Vector4<f32>,
    pub base_color_texture: Option<BoundGpuTexture>,
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub metallic_roughness_texture: Option<BoundGpuTexture>,
}

impl Default for PbrMetallicRoughness {
    fn default() -> Self {
        Self {
            base_color_factor: Vector4::new(1.0, 1.0, 1.0, 1.0),
            base_color_texture: None,
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            metallic_roughness_texture: None,
        }
    }
}

impl PbrMetallicRoughness {
    fn from_gltf(
        buf_cache: &GpuBufferCache,
        doc_cache: &BufferCache<'_>,
        pbr: gltf::material::PbrMetallicRoughness,
        bind_group_layout: &wgpu::BindGroupLayout,
        label: Option<&str>,
    ) -> Self {
        Self {
            base_color_factor: pbr.base_color_factor().into(),
            base_color_texture: pbr.base_color_texture().map(|i| {
                let tex = i.texture();
                let img = tex.source();
                BoundGpuTexture::from_gltf(
                    buf_cache.device(),
                    buf_cache.load_texture(doc_cache, &img),
                    &tex,
                    bind_group_layout,
                    label.map(|l| format!("{l} base color texture")).as_deref(),
                )
            }),
            metallic_factor: pbr.metallic_factor(),
            roughness_factor: pbr.roughness_factor(),
            metallic_roughness_texture: pbr.metallic_roughness_texture().map(|i| {
                let tex = i.texture();
                let img = tex.source();
                BoundGpuTexture::from_gltf(
                    buf_cache.device(),
                    buf_cache.load_texture(doc_cache, &img),
                    &tex,
                    bind_group_layout,
                    label
                        .map(|l| format!("{l} metallic roughness texture"))
                        .as_deref(),
                )
            }),
        }
    }
}

#[derive(Default, Debug)]
pub struct Material {
    pub alpha_cutoff: Option<f32>,
    pub alpha_mode: AlphaMode,
    pub double_sided: bool,
    pub pbr_metallic_roughness: PbrMetallicRoughness,
    // TODO :: material fields: pbr_specular_glossiness, transmission, index of refraction,
    // emissive strength, volume, specular, normal texture, occlusion texture, emissive texture,
    // emissive factor, unlit
}

impl Material {
    pub(super) fn from_gltf(
        buf_cache: &GpuBufferCache,
        doc_cache: &BufferCache<'_>,
        mat: gltf::Material<'_>,
        bind_group_layout: &wgpu::BindGroupLayout,
        label: Option<&str>,
    ) -> Self {
        Self {
            alpha_cutoff: mat.alpha_cutoff(),
            alpha_mode: mat.alpha_mode(),
            double_sided: mat.double_sided(),
            pbr_metallic_roughness: PbrMetallicRoughness::from_gltf(
                buf_cache,
                doc_cache,
                mat.pbr_metallic_roughness(),
                bind_group_layout,
                label.map(|l| format!("{l} PBRMR")).as_deref(),
            ),
        }
    }
}
