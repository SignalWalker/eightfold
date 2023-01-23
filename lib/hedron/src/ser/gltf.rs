#![doc = include_str!("./gltf.md")]

use eightfold_common::ArrayIndex;
use nalgebra::{Quaternion, Vector3};
use stablevec::StableVec;

/// The value of the `generator` field in output `glTF` assets.
///
/// See also: [`glTF` Asset Specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#asset)
pub const GENERATOR_ID: &str = concat!("hedron@", env!("CARGO_PKG_VERSION"));

/// Minimum `glTF` version required to load generated assets.
///
/// See also: [`glTF` Asset Specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#asset)
pub const MIN_VERSION: &str = "2.0";

pub struct NodeMesh<Idx: ArrayIndex> {
    pub idx: Idx,
    pub skin: Option<Idx>,
    pub weights: Vec<f32>,
}

pub struct Node<Idx: ArrayIndex> {
    pub name: Option<String>,
    pub children: Vec<Idx>,
    pub translation: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vector3<f32>,
    pub camera: Option<Idx>,
    pub mesh: Option<NodeMesh<Idx>>,
}

pub enum Camera {
    Orthographic {
        xmag: f32,
        ymag: f32,
        zfar: f32,
        znear: f32,
    },
    Perspective {
        aspect_ratio: Option<f32>,
        yfov: f32,
        zfar: Option<f32>,
        znear: f32,
    },
}

pub struct Scene<Idx: ArrayIndex> {
    name: Option<String>,
    nodes: Vec<Idx>,
}

pub struct AssetField {
    version: (u64, u64),
    minVersion: Option<(u64, u64)>,
    copyright: Option<String>,
    generator: Option<String>,
}

pub struct Asset<Idx: ArrayIndex> {
    asset: AssetField,
    extensions_used: Vec<String>,
    extensions_required: Vec<String>,
    /// default scene
    scene: Option<Idx>,
    scenes: StableVec<Scene<Idx>>,
    nodes: StableVec<Node<Idx>>,

    buffers: StableVec<()>,
    buffer_views: StableVec<()>,
    accessors: StableVec<()>,

    meshes: StableVec<()>,
    skins: StableVec<()>,

    cameras: StableVec<Camera>,

    animations: StableVec<()>,

    images: StableVec<()>,
    materials: StableVec<()>,
    samplers: StableVec<()>,
    textures: StableVec<()>,
}

/// `glTF` primitive topology type
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Mode {
    Points = 0,
    Lines = 1,
    LineLoop = 2,
    LineStrip = 3,
    // Triangles as default: https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#schema-reference-mesh-primitive
    #[default]
    Triangles = 4,
    TriangleStrip = 5,
    TriangleFan = 6,
}

/// `glTF` primitive attributes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Attribute {
    Position,
    Normal,
    Tangent,
    Texcoord(u32),
    Color(u32),
    Joints(u32),
    Weights(u32),
}

/// Type definitions for mesh primitive attribute data
pub mod attribute {
    
    
    
}
