use gltf::{material::AlphaMode, mesh::Mode};
use wgpu::{BufferUsages, PrimitiveTopology};

use crate::{
    buffer::{BufferCache, GpuBufferAccessor, GpuBufferCache, Material, VertexBuffer},
    preview::PipelineData,
};

fn gltf_mode_to_wgpu_topology(m: gltf::mesh::Mode) -> PrimitiveTopology {
    match m {
        Mode::Points => PrimitiveTopology::PointList,
        Mode::Lines => PrimitiveTopology::LineList,
        Mode::LineLoop => todo!("unsupported topology: line loop"),
        Mode::LineStrip => PrimitiveTopology::LineStrip,
        Mode::Triangles => PrimitiveTopology::TriangleList,
        Mode::TriangleStrip => PrimitiveTopology::TriangleStrip,
        Mode::TriangleFan => todo!("unsupported topology: triangle fan"),
    }
}

#[derive(Debug)]
pub struct GpuPrimitive {
    pub topology: PrimitiveTopology,
    pub indices: Option<GpuBufferAccessor>,
    pub material: Material,
    pub vertex_buffers: Vec<VertexBuffer>,
}

impl GpuPrimitive {
    pub fn create_cube(device: &wgpu::Device, label: Option<&str>, color: [f32; 3]) -> Self {
        let (vbuffer, indices) = VertexBuffer::new_cube(
            device,
            label.map(|l| format!("{l} buffer")).as_deref(),
            color,
        );
        Self {
            topology: PrimitiveTopology::TriangleList,
            material: Material {
                alpha_cutoff: None,
                alpha_mode: AlphaMode::Opaque,
                double_sided: false,
                pbr_metallic_roughness: Default::default(),
            },
            indices: Some(indices),
            vertex_buffers: vec![vbuffer],
        }
    }

    pub fn from_gltf(
        buf_cache: &GpuBufferCache,
        doc_cache: &BufferCache<'_>,
        primitive: gltf::Primitive<'_>,
        material_bind_group_layout: &wgpu::BindGroupLayout,
        label: Option<&str>,
    ) -> Self {
        Self {
            topology: gltf_mode_to_wgpu_topology(primitive.mode()),
            indices: primitive
                .indices()
                .map(|i| buf_cache.load_attribute(doc_cache, &i, BufferUsages::INDEX)),
            vertex_buffers: VertexBuffer::generate_from_gltf(
                buf_cache,
                doc_cache,
                primitive.attributes(),
            ),
            material: Material::from_gltf(
                buf_cache,
                doc_cache,
                primitive.material(),
                material_bind_group_layout,
                label.map(|l| format!("{l} material")).as_deref(),
            ),
        }
    }

    pub fn pipeline_data(&self, winding_order: wgpu::FrontFace) -> PipelineData {
        PipelineData {
            primitive_state: wgpu::PrimitiveState {
                topology: self.topology,
                // This only has an effect on strip topologies
                strip_index_format: match self.topology {
                    PrimitiveTopology::LineStrip | PrimitiveTopology::TriangleStrip => {
                        self.indices.as_ref().map(|i| i.ty.index_format())
                    }
                    _ => None,
                },
                front_face: winding_order,
                cull_mode: None,
                //cull_mode: match self.material.double_sided {
                //    true => None,
                //    false => Some(wgpu::Face::Back),
                //},
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            vertex_buffers: self
                .vertex_buffers
                .iter()
                .map(VertexBuffer::layout)
                .collect(),
        }
    }

    /// Submit this primitive to be drawn in the given render pass.
    pub fn draw(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        #[cfg(debug_assertions)]
        if self.vertex_buffers.is_empty() || self.vertex_buffers[0].attributes.is_empty() {
            tracing::error!("tried to render primitive without vertices");
            return;
        }
        if let Some(ref tex) = self.material.pbr_metallic_roughness.base_color_texture {
            render_pass.set_bind_group(2, &tex.bind_group, &[]);
        }
        for (i, buffer) in self.vertex_buffers.iter().enumerate() {
            #[cfg(debug_assertions)]
            if !buffer.view.buffer.usage().contains(BufferUsages::VERTEX) {
                tracing::error!("using non-vertex buffer as vertex buffer");
            }
            for (sem, attr) in &buffer.attributes {
                render_pass.set_vertex_buffer(i as u32, attr.as_slice());
            }
        }
        if let Some(ref indices) = self.indices {
            // TODO :: do i need to care about base vertex here?
            indices.set_index_buffer(render_pass);
            render_pass.draw_indexed(0..indices.count as u32, 0, 0..1);
        } else {
            render_pass.draw(
                0..self.vertex_buffers[0]
                    .attributes
                    .first_key_value()
                    .unwrap()
                    .1
                    .count as u32,
                0..1,
            );
        }
    }
}
