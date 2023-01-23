//! Additional [Octant] implementation for spatial trees

use nalgebra::{point, Point3};
use parry3d::bounding_volume::Aabb;

use parry3d::math::Real as ParryReal;

use crate::Octant;

use super::{error::Error, Float};

impl Octant {
    /// Find the Octant of a point `p` relative to another point `c`.
    #[inline]
    pub fn from_center<Real: Float>(c: &Point3<Real>, p: &Point3<Real>) -> Self {
        Self::new(p.x > c.x, p.y > c.y, p.z > c.z)
    }
}
