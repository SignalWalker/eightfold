//! An octree

mod bounding_cube;
pub use bounding_cube::*;
mod error;
mod octant;

use nalgebra::{Point3, Vector3};

pub use parry3d::math::Real;

/// A point within an octree volume
pub type WorldPoint = Point3<Real>;

/// A vector within an octree volume
pub type WorldVector = Vector3<Real>;
