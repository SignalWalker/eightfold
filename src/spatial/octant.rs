//! Additional [Octant] implementation for spatial trees

use nalgebra::{Point3};




use crate::Octant;

use super::{Float};

impl Octant {
    /// Find the Octant of a point `p` relative to another point `c`.
    #[inline]
    pub fn from_center<Real: Float>(c: &Point3<Real>, p: &Point3<Real>) -> Self {
        Self::new(p.x > c.x, p.y > c.y, p.z > c.z)
    }
}
