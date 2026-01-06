use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use gltf::accessor::{DataType, Dimensions};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BufferAddress, BufferUsages, VertexAttribute,
};

use crate::{
    buffer::{BufferCache, GpuBufferAccessor, GpuBufferCache, GpuBufferType, GpuBufferView},
    preview::{PVertexBufferLayout, Semantic},
};

#[derive(Debug)]
pub struct VertexBuffer {
    pub view: GpuBufferView,
    pub attributes: BTreeMap<Semantic, GpuBufferAccessor>,
}

impl VertexBuffer {
    fn new(view: GpuBufferView) -> Self {
        Self {
            view,
            attributes: Default::default(),
        }
    }

    pub(super) fn new_cube(
        device: &wgpu::Device,
        label: Option<&str>,
        color: [f32; 3],
    ) -> (Self, GpuBufferAccessor) {
        use std::mem::size_of_val;
        const CUBE_POSITIONS: [[f32; 3]; 8] = [
            // back
            [0.5, -0.5, -0.5],
            [-0.5, -0.5, -0.5],
            [-0.5, 0.5, -0.5],
            [0.5, 0.5, -0.5],
            // front
            [-0.5, -0.5, 0.5],
            [0.5, -0.5, 0.5],
            [0.5, 0.5, 0.5],
            [-0.5, 0.5, 0.5],
        ];
        const CUBE_INDICES: [[u16; 3]; 12] = [
            // back
            [0, 1, 2],
            [2, 3, 0],
            // front
            [4, 5, 6],
            [6, 7, 4],
            // bottom
            [5, 4, 1],
            [1, 0, 5],
            // top
            [3, 2, 7],
            [7, 6, 3],
            // left
            [1, 4, 7],
            [7, 2, 1],
            // right
            [5, 0, 3],
            [3, 6, 5],
        ];
        let cube_colors = [color; 8];
        let mut cube_buffer = Vec::<u8>::with_capacity(
            size_of_val(&CUBE_POSITIONS) + size_of_val(&cube_colors) + size_of_val(&CUBE_INDICES),
        );
        cube_buffer.extend_from_slice(bytemuck::cast_slice(&CUBE_POSITIONS));
        cube_buffer.extend_from_slice(bytemuck::cast_slice(&cube_colors));
        cube_buffer.extend_from_slice(bytemuck::cast_slice(&CUBE_INDICES));
        let buffer = Arc::new(device.create_buffer_init(&BufferInitDescriptor {
            label,
            contents: &cube_buffer,
            usage: BufferUsages::VERTEX | BufferUsages::INDEX,
        }));
        let view = GpuBufferView::new(buffer, cube_buffer.len() as BufferAddress, 0, None);

        (
            Self {
                attributes: {
                    let mut res = BTreeMap::new();
                    res.insert(
                        Semantic::Position,
                        GpuBufferAccessor::new(
                            view.clone(),
                            GpuBufferType {
                                ty: DataType::F32,
                                dimensions: Dimensions::Vec3,
                            },
                            0,
                            CUBE_POSITIONS.len() as BufferAddress,
                            false,
                        ),
                    );
                    res.insert(
                        Semantic::Color(0),
                        GpuBufferAccessor::new(
                            view.clone(),
                            GpuBufferType {
                                ty: DataType::F32,
                                dimensions: Dimensions::Vec3,
                            },
                            CUBE_POSITIONS.len() as BufferAddress,
                            cube_colors.len() as BufferAddress,
                            false,
                        ),
                    );
                    res
                },
                view: view.clone(),
            },
            GpuBufferAccessor::new(
                view,
                GpuBufferType {
                    ty: DataType::U16,
                    dimensions: Dimensions::Scalar,
                },
                (CUBE_POSITIONS.len() + cube_colors.len()) as BufferAddress,
                CUBE_INDICES.len() as BufferAddress,
                false,
            ),
        )
    }

    pub(super) fn generate_from_gltf<'attr>(
        buf_cache: &GpuBufferCache,
        doc_cache: &BufferCache<'_>,
        attributes: impl Iterator<Item = (gltf::Semantic, gltf::Accessor<'attr>)>,
    ) -> Vec<Self> {
        fn cmp(a: &VertexBuffer, b: &VertexBuffer) -> Ordering {
            for (a, b) in a.attributes.keys().zip(b.attributes.keys()) {
                let c = a.cmp(b);
                if c != Ordering::Equal {
                    return c;
                }
            }
            Ordering::Equal
        }
        let mut buffers = HashMap::<GpuBufferView, Self>::new();
        for (sem, accessor) in attributes {
            let accessor = buf_cache.load_attribute(doc_cache, &accessor, BufferUsages::VERTEX);
            let view = &accessor.view;
            let vbuf = buffers
                .entry(view.clone())
                .or_insert_with(|| VertexBuffer::new(view.clone()));
            vbuf.attributes.insert(sem.into(), accessor);
        }
        let mut res = buffers.into_values().collect::<Vec<_>>();
        // sort buffers so that pipeline data is consistent & therefore there will be more cache
        // hits
        res.sort_unstable_by(cmp);
        res
    }

    pub(super) fn layout(&self) -> PVertexBufferLayout {
        let stride;
        let attributes = {
            let mut res = Vec::with_capacity(self.attributes.len());
            let mut offset = 0;
            for (sem, attr) in &self.attributes {
                res.push(VertexAttribute {
                    format: attr.ty.vertex_format(),
                    offset,
                    shader_location: sem.location(),
                });
                offset += attr.ty.bytes() as u64;
            }
            stride = offset;
            res
        };
        PVertexBufferLayout {
            array_stride: self.view.stride.unwrap_or(stride),
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes,
        }
    }
}
