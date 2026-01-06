use std::{marker::PhantomData, num::NonZero, sync::Arc};

use nalgebra::{Affine3, Matrix4};
use static_assertions::const_assert;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BufferUsages,
};

mod gpu_arena;
pub use gpu_arena::*;

mod gpu_image;
pub use gpu_image::*;

mod ty;
pub use ty::*;

mod view;
pub use view::*;

mod accessor;
pub use accessor::*;

mod material;
pub use material::*;

mod vertex;
pub use vertex::*;

mod primitive;
pub use primitive::*;

mod mesh;
pub use mesh::*;

use crate::{buffer::BufferCache, gltf_to_nalgebra, preview::PipelineStore};

fn winding_order(mat: &Matrix4<f32>) -> wgpu::FrontFace {
    if mat.determinant() > 0.0 {
        wgpu::FrontFace::Ccw
    } else {
        wgpu::FrontFace::Cw
    }
}

#[derive(Debug)]
pub struct UniformBuffer<T> {
    buffer: wgpu::Buffer,

    _value: PhantomData<T>,
}

impl<T> std::ops::Deref for UniformBuffer<T> {
    type Target = wgpu::Buffer;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl<T: bytemuck::NoUninit> UniformBuffer<T> {
    pub fn create_init(
        device: &wgpu::Device,
        label: Option<&str>,
        usage: BufferUsages,
        value: &T,
    ) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label,
            contents: bytemuck::bytes_of(value),
            usage: BufferUsages::UNIFORM | usage,
        });
        Self {
            buffer,
            _value: PhantomData,
        }
    }

    pub fn write(&self, queue: &wgpu::Queue, value: &T) {
        self.write_with(queue)
            .copy_from_slice(bytemuck::bytes_of(value));
    }

    pub fn write_with<'view>(
        &'view self,
        queue: &'view wgpu::Queue,
    ) -> wgpu::QueueWriteBufferView<'view> {
        queue
            .write_buffer_with(
                &self.buffer,
                0,
                NonZero::new(u64::try_from(std::mem::size_of::<T>()).unwrap()).unwrap(),
            )
            .unwrap()
    }
}

/// The root of a tree of [nodes](Node).
pub struct Scene {
    name: Option<String>,
    /// The root nodes of this scene.
    pub nodes: Vec<Node>,
}

impl Scene {
    pub fn from_gltf(
        buf_cache: &GpuBufferCache,
        doc_cache: &BufferCache<'_>,
        scene: gltf::Scene<'_>,
        node_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let identity: Affine3<f32> = Affine3::identity();
        let name = scene.name().map(ToOwned::to_owned);
        Self {
            name,
            nodes: scene
                .nodes()
                .map(|node| {
                    Node::from_gltf(
                        buf_cache,
                        doc_cache,
                        &identity,
                        node,
                        node_bind_group_layout,
                    )
                })
                .collect(),
        }
    }

    #[inline]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

/// A node within a scene, with a transform relative to its parent and, optionally, a
/// [mesh](GpuMesh).
pub struct Node {
    name: Option<String>,
    world_transform: Affine3<f32>,
    world_transform_buffer: UniformBuffer<Affine3<f32>>,
    bind_group: wgpu::BindGroup,
    pub mesh: Option<Arc<GpuMesh>>,
    pub children: Vec<Self>,
    // TODO :: node fields: camera, light, skin, weights
}

impl Node {
    pub fn from_gltf(
        buf_cache: &GpuBufferCache,
        doc_cache: &BufferCache<'_>,
        parent_world_transform: &Affine3<f32>,
        node: gltf::Node<'_>,
        node_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let device = buf_cache.device();
        let name = node.name().map(ToOwned::to_owned);
        let world_transform: Affine3<f32> =
            parent_world_transform * gltf_to_nalgebra(&node.transform());
        let world_transform_buffer = UniformBuffer::create_init(
            device,
            name.as_ref().map(|n| format!("{n} uniform")).as_deref(),
            BufferUsages::empty(),
            &world_transform,
        );
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: name.as_ref().map(|n| format!("{n} bind group")).as_deref(),
            layout: node_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: world_transform_buffer.as_entire_binding(),
            }],
        });
        Self {
            mesh: node
                .mesh()
                .map(|mesh| Arc::new(GpuMesh::from_gltf(buf_cache, doc_cache, mesh))),
            children: node
                .children()
                .map(|child| {
                    Node::from_gltf(
                        buf_cache,
                        doc_cache,
                        &world_transform,
                        child,
                        node_bind_group_layout,
                    )
                })
                .collect(),
            bind_group,
            world_transform_buffer,
            world_transform,
            name,
        }
    }

    #[inline]
    pub fn world_transform(&self) -> &Affine3<f32> {
        &self.world_transform
    }

    #[inline]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Determine the winding order of primitives in this node using the determinant of its transform.
    ///
    /// The winding order of a glTF primitive is encoded by the determinant of its
    /// parent node's **global** transform. Positive determinant => counterclockwise
    /// winding order.
    pub fn winding_order(&self) -> wgpu::FrontFace {
        if self.world_transform.matrix().determinant() > 0.0 {
            wgpu::FrontFace::Ccw
        } else {
            wgpu::FrontFace::Cw
        }
    }
}
