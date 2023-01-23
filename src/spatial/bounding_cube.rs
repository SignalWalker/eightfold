use nalgebra::{point, Point3};
use parry3d::bounding_volume::Aabb;

use super::Float;
use crate::Octant;

#[derive(Debug, thiserror::Error)]
pub enum AabcError<Real: Float> {
    #[error("volume {0:?} does not contain point {1:?}")]
    PointOutOfBounds(Aabc<Real>, Point3<Real>),
}

/// Axis-Aligned Bounding Cube
#[derive(Debug, Clone, Copy)]
pub struct Aabc<R: Float> {
    pub origin: Point3<R>,
    pub length: R,
}

impl<Real: Float> Aabc<Real> {
    pub fn new(origin: Point3<Real>, length: Real) -> Self {
        Self { origin, length }
    }

    pub fn maxs(&self) -> Point3<Real> {
        nalgebra::point![
            self.origin.x + self.length,
            self.origin.y + self.length,
            self.origin.z + self.length
        ]
    }

    pub fn new_invalid() -> Self {
        Self {
            origin: point![Real::MAX, Real::MAX, Real::MAX],
            length: Real::MIN,
        }
    }

    /// Determine whether a point `p` lies within `self`.
    pub fn contains(&self, p: &Point3<Real>) -> bool {
        let Self {
            length: l,
            origin: o,
        } = &self;
        (p.x >= o.x && p.x < (o.x + *l))
            && (p.y >= o.y && (p.y < (o.y + *l)))
            && (p.z >= o.z && (p.z < (o.z + *l)))
    }

    /// Grow `self` such that it contains `p`.
    pub fn expand_to(&mut self, p: &Point3<Real>) {
        self.origin = self.origin.inf(p);
        self.length = (self.maxs().sup(p) - self.origin).max();
    }

    /// Given an [Octant] `oct`, construct an [Aabc] `n` such that `self` is the `o`th octant of `n`
    pub fn parent(&self, oct: Octant) -> Self {
        let l2 = self.length * Real::TWO;
        Self {
            origin: match oct.0 {
                0 => self.origin,
                1 => point![self.origin.x, self.origin.y, self.origin.z - l2],
                2 => point![self.origin.x, self.origin.y - l2, self.origin.z],
                3 => point![self.origin.x, self.origin.y - l2, self.origin.z - l2],
                4 => point![self.origin.x - l2, self.origin.y, self.origin.z],
                5 => point![self.origin.x - l2, self.origin.y, self.origin.z - l2],
                6 => point![self.origin.x - l2, self.origin.y - l2, self.origin.z],
                7 => point![self.origin.x - l2, self.origin.y - l2, self.origin.z - l2],
                _ => unreachable!(),
            },
            length: l2,
        }
    }

    /// Determine the center point of `self`.
    pub fn center(&self) -> Point3<Real> {
        let l2 = self.length / Real::TWO;
        Point3::new(self.origin.x + l2, self.origin.y + l2, self.origin.z + l2)
    }

    /// Given an [Octant] `oct`, construct an [Aabc] `n` such that `n` is the `o`th octant of `self`
    pub fn child(&self, oct: Octant) -> Self {
        let l2 = self.length / Real::TWO;
        Self {
            origin: match oct.0 {
                0 => self.origin,
                1 => point![self.origin.x, self.origin.y, self.origin.z + l2],
                2 => point![self.origin.x, self.origin.y + l2, self.origin.z],
                3 => point![self.origin.x, self.origin.y + l2, self.origin.z + l2],
                4 => point![self.origin.x + l2, self.origin.y, self.origin.z],
                5 => point![self.origin.x + l2, self.origin.y, self.origin.z + l2],
                6 => point![self.origin.x + l2, self.origin.y + l2, self.origin.z],
                7 => point![self.origin.x + l2, self.origin.y + l2, self.origin.z + l2],
                _ => unreachable!(),
            },
            length: l2,
        }
    }

    /// Determine the [Octant] of `p`.
    ///
    /// This still works even if `p` ∉ `self`: the result is given as if taking the octant of `p`
    /// within an infinitely-large bounding box sharing a center with `self`.
    pub fn octant_of<'err>(&self, p: &'err Point3<Real>) -> Octant {
        let o = &self.origin;
        let l2 = self.length / Real::TWO;
        Octant::new(p.x > o.x + l2, p.y > o.y + l2, p.z > o.z + l2)
    }

    /// Given a [point](Point3) `p`, construct an [Aabc] `n` such that `n` is an octant of `self`
    /// containing `p`.
    ///
    /// * [`PointOutOfBounds`](AabcError::PointOutOfBounds) if `p` ∉ `self`.
    pub fn child_containing(&self, p: &Point3<Real>) -> Result<Self, AabcError<Real>> {
        if !self.contains(p) {
            return Err(AabcError::PointOutOfBounds(*self, *p));
        }
        Ok(self.child(self.octant_of(p)))
    }
}

impl From<Aabc<parry3d::math::Real>> for parry3d::bounding_volume::Aabb {
    fn from(e: Aabc<parry3d::math::Real>) -> Self {
        Aabb {
            mins: e.origin,
            maxs: e.maxs(),
        }
    }
}
