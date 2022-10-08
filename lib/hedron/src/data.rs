//! # See Also
//!
//! * [glTF Reference Guide](https://www.khronos.org/files/gltf20-reference-guide.pdf)

mod mesh;
pub use mesh::*;

use eightfold_common::ArrayIndex;
use stablevec::StableVec;

/// A set of scenes and associated data, which may be shared between scenes.
#[derive(Debug)]
pub struct DataSet<Idx: ArrayIndex> {
    /// If extant, the index of the default scene
    pub default_scene: Option<Idx>,
    pub(crate) scenes: StableVec<Scene<Idx>>,
    pub(crate) nodes: StableVec<Node<Idx>>,
    pub(crate) lights: StableVec<()>,
    pub(crate) cameras: StableVec<()>,
    pub(crate) skins: StableVec<()>,
    pub(crate) animations: StableVec<()>,
    pub(crate) materials: StableVec<()>,
    pub(crate) textures: StableVec<()>,
    pub(crate) samplers: StableVec<()>,
    pub(crate) images: StableVec<()>,
    pub(crate) meshes: StableVec<()>,
    pub(crate) accessors: StableVec<()>,
    pub(crate) buffer_views: StableVec<()>,
    pub(crate) buffers: StableVec<()>,
}

impl<Idx: ArrayIndex> Default for DataSet<Idx> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<Idx: ArrayIndex> DataSet<Idx> {
    pub fn empty() -> Self {
        Self {
            default_scene: None,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Scene<Idx: ArrayIndex> {
    nodes: Vec<Idx>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum NodeParent<Idx: ArrayIndex> {
    Scene(Idx),
    Node(Idx),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum NodeData<Idx: ArrayIndex> {
    Camera(Idx),
    Mesh(Idx), // TODO :: https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-node
    Light(Idx),
}

#[derive(Debug, Clone)]
pub struct Node<Idx: ArrayIndex> {
    parent: NodeParent<Idx>,
    children: Vec<Idx>,
    data: (),
}
