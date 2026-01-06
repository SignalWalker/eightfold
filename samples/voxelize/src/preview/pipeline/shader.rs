use std::{collections::HashMap, sync::Arc};

use dashmap::DashMap;
use wgpu::naga::{self, ShaderStage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Semantic {
    Position,
    Normal,
    Tangent,
    TexCoord(u32),
    Color(u32),
    Joint(u32),
    Weight(u32),
}

impl From<gltf::Semantic> for Semantic {
    fn from(value: gltf::Semantic) -> Self {
        match value {
            gltf::Semantic::Positions => Self::Position,
            gltf::Semantic::Normals => Self::Normal,
            gltf::Semantic::Tangents => Self::Tangent,
            gltf::Semantic::Colors(i) => Self::Color(i),
            gltf::Semantic::TexCoords(i) => Self::TexCoord(i),
            gltf::Semantic::Joints(i) => Self::Joint(i),
            gltf::Semantic::Weights(i) => Self::Weight(i),
        }
    }
}

impl Semantic {
    pub fn location(self) -> u32 {
        match self {
            Self::Position => 0,
            Self::Normal => 1,
            Self::TexCoord(0) => 2,
            Self::TexCoord(1) => 3,
            Self::TexCoord(n) => todo!("semantic shader location: texcoords {n}"),
            Self::Tangent => 4,
            Self::Color(0) => 5,
            Self::Color(n) => todo!("semantic shader location: colors {n}"),
            Self::Joint(0) => 6,
            Self::Joint(n) => todo!("semantic shader location: joints {n}"),
            Self::Weight(0) => 7,
            Self::Weight(n) => todo!("semantic shader location: weights {n}"),
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
pub struct VertexAttr {
    semantic: Option<Semantic>,
    binding: naga::Binding,
    format: wgpu::VertexFormat,
}

pub struct VertexLayout {
    pub fields: Vec<VertexAttr>,
}

pub struct ShaderInput {
    pub vertex: VertexLayout,
}

pub struct EntryPoint {
    pub stage: ShaderStage,
}

pub struct BaseShader {
    entry_points: HashMap<String, ()>,
    shader: naga::Module,
}

impl BaseShader {
    fn new(mut shader: naga::Module) -> Self {
        naga::compact::compact(&mut shader);
        let mut entry_points = HashMap::new();
        for entry_point in &shader.entry_points {
            let name = entry_point.name.clone();
            let stage = entry_point.stage;
            entry_points.insert(name, ());
        }
        Self {
            entry_points,
            shader,
        }
    }
}

pub struct ShaderCache {
    pub device: Arc<wgpu::Device>,
    pub base_shader: BaseShader,
    pub module: wgpu::ShaderModule,
    pub shaders: DashMap<(), wgpu::ShaderModule>,
}

impl ShaderCache {
    pub(super) fn new(device: Arc<wgpu::Device>, base_shader: naga::Module) -> Self {
        let base_shader = BaseShader::new(base_shader);
        //let v_bindings = process(&mut base_shader);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Naga(std::borrow::Cow::Owned(base_shader.shader.clone())),
        });
        Self {
            device,
            base_shader,
            module,
            shaders: Default::default(),
        }
    }
}

fn process(shader: &mut naga::Module) {}
