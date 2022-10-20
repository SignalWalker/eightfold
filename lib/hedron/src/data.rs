//! # See Also
//!

mod mesh;
use std::collections::HashMap;

pub use mesh::*;

use eightfold_common::ArrayIndex;
use nalgebra::Matrix4;
use stablevec::StableVec;

/// A set of scenes and associated data, which may be shared between scenes.
#[derive(Debug)]
pub struct DataSet<Real, Idx: ArrayIndex = u32> {
    /// If extant, the index of the default scene
    pub default_scene: Option<Idx>,
    pub(crate) names: HashMap<String, Idx>,
    pub(crate) scenes: StableVec<Scene<Idx>>,
    pub(crate) nodes: StableVec<Node<Real, Idx>>,
    pub(crate) transforms: StableVec<Matrix4<Real>>,
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

impl<Real, Idx: ArrayIndex> Default for DataSet<Real, Idx> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<Real, Idx: ArrayIndex> DataSet<Real, Idx> {
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

#[derive(Debug, Clone)]
pub struct Node<Real, Idx: ArrayIndex> {
    parent: NodeParent<Idx>,
    children: Vec<Idx>,
    // node-local data
    transform: Option<Idx>,
    camera: Option<Idx>,
    skin: Option<Idx>,
    light: Option<Idx>,
    // mesh-specific
    mesh: Option<Idx>,
    weights: Vec<Real>,
}
