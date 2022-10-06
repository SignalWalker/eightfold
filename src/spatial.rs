//! [Octrees](crate::Octree) with a defined relation to a 3D space.

mod bounding_cube;
pub use bounding_cube::*;
mod error;
mod octant;

use nalgebra::{Point3, Projective3, Vector3};

pub use parry3d::math::Real;

use crate::{Octree, TreeIndex};

/// A point within an octree volume
pub type WorldPoint = Point3<Real>;

/// A vector within an octree volume
pub type WorldVector = Vector3<Real>;

/// An [Octree] with an associated injective transformation such that each node represents a volume in 3D space unique among nodes at the same depth.
pub struct ProjectiveOctree<T, Idx: TreeIndex> {
    base: Octree<T, Idx>,
    trans_into: Projective3<Real>,
    trans_from: Projective3<Real>,
}

impl<T, Idx: TreeIndex> ProjectiveOctree<T, Idx> {
    pub fn new() -> Self {
        todo!()
    }
}
