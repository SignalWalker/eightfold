use std::{
    ops::Deref,
    sync::{atomic::AtomicUsize, Arc},
};

use dashmap::DashMap;
use wgpu::{naga, BindGroupLayout, BufferAddress, VertexAttribute, VertexStepMode};

mod shader;
pub use shader::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PVertexBufferLayout {
    pub array_stride: BufferAddress,
    pub step_mode: VertexStepMode,
    pub attributes: Vec<VertexAttribute>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PipelineData {
    pub primitive_state: wgpu::PrimitiveState,
    pub vertex_buffers: Vec<PVertexBufferLayout>,
}

impl std::fmt::Display for PipelineData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}

pub struct PipelineStore {
    shader_cache: ShaderCache,
    texture_format: wgpu::TextureFormat,
    pub pipelines: DashMap<PipelineData, wgpu::RenderPipeline>,
    bind_group_layouts: Vec<wgpu::BindGroupLayout>,

    p_index: AtomicUsize,
}

impl PipelineStore {
    pub fn new(
        device: Arc<wgpu::Device>,
        base_shader: naga::Module,
        texture_format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            texture_format,
            pipelines: DashMap::new(),

            p_index: AtomicUsize::new(0),

            bind_group_layouts: vec![
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Camera Bind Group Layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                }),
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Mesh Bind Group Layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                }),
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Material Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                }),
            ],

            shader_cache: ShaderCache::new(device, base_shader),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.pipelines.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pipelines.is_empty()
    }

    pub fn bind_group_layouts(&self) -> &[wgpu::BindGroupLayout] {
        self.bind_group_layouts.as_slice()
    }

    pub fn get<'pipeline>(
        &'pipeline self,
        data: &PipelineData,
    ) -> Option<impl Deref<Target = wgpu::RenderPipeline> + 'pipeline> {
        self.pipelines.get(data)
    }

    pub fn get_or_create<'pipeline>(
        &'pipeline self,
        device: &wgpu::Device,
        data: &PipelineData,
    ) -> impl Deref<Target = wgpu::RenderPipeline> + 'pipeline {
        if self.pipelines.contains_key(data) {
            return self.pipelines.get(data).unwrap();
        }

        tracing::info!(%data, "generating new pipeline");

        let index = self
            .p_index
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(format!("Render Pipeline Layout {index}")).as_deref(),
            bind_group_layouts: self
                .bind_group_layouts
                .iter()
                .collect::<Vec<_>>()
                .as_slice(),
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(format!("Render Pipeline {index}")).as_deref(),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &self.shader_cache.module,
                entry_point: Some("vs_main"),
                buffers: &data
                    .vertex_buffers
                    .iter()
                    .map(|buf| wgpu::VertexBufferLayout {
                        array_stride: buf.array_stride,
                        step_mode: buf.step_mode,
                        attributes: &buf.attributes,
                    })
                    .collect::<Vec<_>>(),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &self.shader_cache.module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.texture_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: data.primitive_state,
            depth_stencil: Some(wgpu::DepthStencilState {
                // FIX :: coordinate depth format with depth texture
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        self.pipelines.insert(data.clone(), pipeline);
        self.pipelines.get(data).unwrap()
    }
}
