use std::collections::HashMap;

use nalgebra::{Affine3, Matrix3, Matrix4, Rotation3};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupEntry, BufferUsages, RenderPass,
};

use crate::{
    buffer::{BufferCache, GpuBufferCache, GpuPrimitive},
    preview::{PipelineData, PipelineStore},
};

#[derive(Debug)]
pub struct GpuMesh {
    // TODO :: sort primitives by pipeline?
    pub primitives: HashMap<PipelineData, Vec<GpuPrimitive>>,
}

impl GpuMesh {
    pub fn create_cube(device: &wgpu::Device, label: Option<&str>, color: [f32; 3]) -> Self {
        Self {
            primitives: {
                let prim = GpuPrimitive::create_cube(
                    device,
                    label.map(|l| format!("{l} primitive")).as_deref(),
                    color,
                );
                let mut res = HashMap::<PipelineData, Vec<GpuPrimitive>>::new();
                res.insert(prim.pipeline_data(winding_order), vec![prim]);
                res
            },
        }
    }

    pub fn from_gltf(
        buf_cache: &GpuBufferCache,
        doc_cache: &BufferCache<'_>,
        mesh: gltf::Mesh<'_>,
    ) -> Self {
        let device = buf_cache.device();
        let label = mesh.name();
        Self {
            primitives: {
                let primitives = mesh
                    .primitives()
                    .enumerate()
                    .map(|(i, p)| {
                        let res = GpuPrimitive::from_gltf(
                            buf_cache,
                            doc_cache,
                            p,
                            label.map(|l| format!("{l} primitive {i}")).as_deref(),
                        );
                        let p_data = res.pipeline_data(winding_order);
                        (p_data, res)
                    })
                    .collect::<Vec<_>>();
                let mut res = HashMap::<PipelineData, Vec<GpuPrimitive>>::new();
                for (pipe, prim) in primitives {
                    res.entry(pipe).or_default().push(prim);
                }
                for pipe in res.keys() {
                    // FIX :: doesn't seem like the best place to do this, honestly
                    // (ensure pipeline existence)
                    pipe_cache.get_or_create(buf_cache.device(), pipe);
                }
                res
            },
        }
    }

    pub fn draw(&self, p_data: &PipelineData, render_pass: &mut RenderPass) {
        let primitives = self.primitives.get(p_data);
        #[cfg(debug_assertions)]
        if primitives.is_none() {
            tracing::error!("tried to render mesh primitives for incorrect pipeline");
            return;
        }
        render_pass.set_bind_group(1, &self.data_bind_group, &[]);
        for primitive in primitives.unwrap() {
            primitive.draw(render_pass);
        }
    }
}
