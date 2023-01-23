#![doc = include_str!("../assets/doc/header.html")] // style metadata
#![doc = include_str!("../README.md")] // actual documentation
#![doc(
    html_favicon_url = "https://github.com/signalwalker/eightfold/raw/main/assets/doc/logo.svg",
    html_logo_url = "https://github.com/signalwalker/eightfold/raw/main/assets/doc/logo.svg"
)]
// release build lints
#![cfg_attr(not(debug_assertions), deny(unreachable_pub), warn(missing_docs))]

mod geom;
#[cfg(feature = "mesh")]
pub mod mesh;
#[cfg(feature = "render")]
pub mod render;
#[cfg(feature = "spatial")]
pub mod spatial;
mod tree;

#[cfg(feature = "mesh")]
pub use hedron;
pub use stablevec;

use eightfold_common::ArrayIndex;
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
pub struct NodePoint<Idx: ArrayIndex>(pub Point4<Idx>);

impl<Idx: ArrayIndex> NodePoint<Idx> {
    #[inline]
    pub const fn new(x: Idx, y: Idx, z: Idx, d: Idx) -> Self {
        Self(nalgebra::point![x, y, z, d])
    }
}

/// Quickly construct a [NodePoint]
#[macro_export]
macro_rules! nodepoint {
    [$x:expr, $y:expr, $z:expr, $w:expr] => {
        NodePoint(nalgebra::point![$x, $y, $z, $w])
    }
}
