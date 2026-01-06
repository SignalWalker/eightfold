use std::sync::Arc;

use wgpu::{BufferAddress, BufferSlice};

use crate::buffer::{GpuBufferAccessor, GpuBufferType};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GpuBufferView {
    pub buffer: Arc<wgpu::Buffer>,
    pub length: BufferAddress,
    pub offset: BufferAddress,
    pub stride: Option<BufferAddress>,
}

impl GpuBufferView {
    pub const fn new(
        buffer: Arc<wgpu::Buffer>,
        length: BufferAddress,
        offset: BufferAddress,
        stride: Option<BufferAddress>,
    ) -> Self {
        Self {
            buffer,
            length,
            offset,
            stride,
        }
    }

    pub fn from_gltf(buffer: Arc<wgpu::Buffer>, view: &gltf::buffer::View<'_>) -> Self {
        // FIX :: don't use `as`
        Self::new(
            buffer,
            view.length() as BufferAddress,
            view.offset() as BufferAddress,
            view.stride().map(|s| s as BufferAddress),
        )
    }

    pub const fn access(
        self,
        ty: GpuBufferType,
        offset: BufferAddress,
        count: BufferAddress,
        normalized: bool,
    ) -> GpuBufferAccessor {
        GpuBufferAccessor {
            view: self,
            ty,
            offset,
            count,
            normalized,
        }
    }

    pub fn access_gltf(self, acc: &gltf::Accessor<'_>) -> GpuBufferAccessor {
        self.access(
            GpuBufferType {
                ty: acc.data_type(),
                dimensions: acc.dimensions(),
            },
            acc.offset() as BufferAddress,
            acc.count() as BufferAddress,
            acc.normalized(),
        )
    }

    /// Return the [`BufferSlice`] referenced by this view.
    pub fn as_slice(&self) -> BufferSlice<'_> {
        self.buffer.slice(self.offset..(self.offset + self.length))
    }
}
