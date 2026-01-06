use gltf::accessor::{DataType, Dimensions};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct GpuBufferType {
    pub ty: DataType,
    pub dimensions: Dimensions,
}

impl GpuBufferType {
    #[inline]
    pub fn bytes(&self) -> usize {
        self.ty.size() * self.dimensions.multiplicity()
    }

    pub fn vertex_format(&self) -> wgpu::VertexFormat {
        match (self.ty, self.dimensions) {
            (DataType::U8, Dimensions::Vec2) => wgpu::VertexFormat::Uint8x2,
            (DataType::U8, Dimensions::Vec4) => wgpu::VertexFormat::Uint8x4,
            (DataType::I8, Dimensions::Vec2) => wgpu::VertexFormat::Sint8x2,
            (DataType::I8, Dimensions::Vec4) => wgpu::VertexFormat::Sint8x2,
            (DataType::U16, Dimensions::Vec2) => wgpu::VertexFormat::Uint16x2,
            (DataType::U16, Dimensions::Vec4) => wgpu::VertexFormat::Uint16x2,
            (DataType::I16, Dimensions::Vec2) => wgpu::VertexFormat::Sint16x2,
            (DataType::I16, Dimensions::Vec4) => wgpu::VertexFormat::Sint16x2,
            (DataType::U32, Dimensions::Scalar) => wgpu::VertexFormat::Uint32,
            (DataType::U32, Dimensions::Vec2) => wgpu::VertexFormat::Uint32x2,
            (DataType::U32, Dimensions::Vec3) => wgpu::VertexFormat::Uint32x3,
            (DataType::U32, Dimensions::Vec4) => wgpu::VertexFormat::Uint32x4,
            (DataType::F32, Dimensions::Scalar) => wgpu::VertexFormat::Float32,
            (DataType::F32, Dimensions::Vec2) => wgpu::VertexFormat::Float32x2,
            (DataType::F32, Dimensions::Vec3) => wgpu::VertexFormat::Float32x3,
            (DataType::F32, Dimensions::Vec4) => wgpu::VertexFormat::Float32x4,
            _ => todo!("unsupported vertex format"),
        }
    }

    pub fn index_format(&self) -> wgpu::IndexFormat {
        match (self.ty, self.dimensions) {
            (DataType::U16, Dimensions::Scalar) => wgpu::IndexFormat::Uint16,
            (DataType::U32, Dimensions::Scalar) => wgpu::IndexFormat::Uint32,
            _ => todo!("unsupported index format"),
        }
    }
}
