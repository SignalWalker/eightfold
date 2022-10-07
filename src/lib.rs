#![doc = include_str!("../README.md")]
#![cfg_attr(not(debug_assertions), warn(missing_docs))]
#![warn(unreachable_pub)]

mod geom;
#[cfg(feature = "mesh")]
pub mod mesh;
#[cfg(feature = "render")]
pub mod render;
#[cfg(feature = "spatial")]
pub mod spatial;
mod tree;

pub use stablevec as vec;

use nalgebra::{Point3, Point4};
// pub mod slice;
// pub mod view;
pub use geom::*;
pub use tree::*;

/// The coordinates of a voxel within an octree's voxel grid.
pub type VoxelPoint<Idx> = Point3<Idx>;

/// The coordinates of a node within an octree, including its depth. { X, Y, Z, D }
///
/// In voxel terms, a NodePoint is a point `XYZ` within a voxel grid of size `2ᴰ`.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct NodePoint<Idx: TreeIndex>(pub Point4<Idx>);

impl<Idx: TreeIndex> NodePoint<Idx> {
    pub const fn new(x: Idx, y: Idx, z: Idx, d: Idx) -> Self {
        Self(nalgebra::point![x, y, z, d])
    }
}
