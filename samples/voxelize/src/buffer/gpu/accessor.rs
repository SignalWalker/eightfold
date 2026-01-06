use std::sync::Arc;

use wgpu::{BufferAddress, BufferSlice, BufferUsages};

use crate::buffer::{GpuBufferType, GpuBufferView};

#[derive(Debug, PartialEq, Eq)]
pub struct GpuBufferAccessor {
    pub view: GpuBufferView,
    pub ty: GpuBufferType,
    /// The offset of this accessor from the beginning of its [view](GpuBufferView).
    pub offset: BufferAddress,
    /// The amount of items in this buffer.
    pub count: BufferAddress,
    pub normalized: bool,
}

impl GpuBufferAccessor {
    pub const fn new(
        view: GpuBufferView,
        ty: GpuBufferType,
        offset: BufferAddress,
        count: BufferAddress,
        normalized: bool,
    ) -> Self {
        Self {
            view,
            ty,
            offset,
            count,
            normalized,
        }
    }
    pub fn from_gltf(buf: Arc<wgpu::Buffer>, acc: &gltf::Accessor<'_>) -> Self {
        GpuBufferView::from_gltf(buf, &acc.view().expect("non-sparse accessor")).access_gltf(acc)
    }

    pub fn bytes(&self) -> BufferAddress {
        // FIX :: handle conversion errors more gracefully
        self.count * BufferAddress::try_from(self.ty.bytes()).unwrap()
    }

    /// The offset from the beginning of the [`Buffer`](wgpu::Buffer).
    #[inline]
    pub fn buffer_offset(&self) -> BufferAddress {
        self.offset + self.view.offset
    }

    /// Return the [`BufferSlice`] referenced by this accessor.
    pub fn as_slice(&self) -> BufferSlice<'_> {
        let offset = self.buffer_offset();
        self.view.buffer.slice(offset..(offset + self.bytes()))
    }

    /// Set this as the active index buffer for the given render pass.
    pub fn set_index_buffer(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        #[cfg(debug_assertions)]
        if !self.view.buffer.usage().contains(BufferUsages::INDEX) {
            tracing::error!("using non-index buffer as index buffer");
        }
        render_pass.set_index_buffer(self.as_slice(), self.ty.index_format())
    }

    pub fn set_vertex_buffer(&self, render_pass: &mut wgpu::RenderPass<'_>, slot: u32) {
        #[cfg(debug_assertions)]
        if !self.view.buffer.usage().contains(BufferUsages::VERTEX) {
            tracing::error!("using non-vertex buffer as vertex buffer");
        }
        render_pass.set_vertex_buffer(slot, self.as_slice());
    }
}
