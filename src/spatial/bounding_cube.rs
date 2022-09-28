use std::ops::Add;

use nalgebra::{Point3, Scalar};
use parry3d::bounding_volume::AABB;

use super::{error::Error, Real};
use crate::Octant;

/// Axis-Aligned Bounding Cube
#[derive(Debug, Clone)]
pub struct AABC<R: Scalar> {
    pub origin: Point3<R>,
    pub length: R,
}

impl<R: Scalar> AABC<R> {
    pub fn new(origin: Point3<R>, length: R) -> Self {
        Self { origin, length }
    }

    pub fn maxs(&self) -> Point3<R>
    where
        R: Add<Output = R> + Copy,
    {
        Point3::new(
            self.origin.x + self.length,
            self.origin.y + self.length,
            self.origin.z + self.length,
        )
    }
}

impl AABC<Real> {
    pub fn new_invalid() -> Self {
        Self {
            origin: Point3::new(Real::MAX, Real::MAX, Real::MAX),
            length: Real::MIN,
        }
    }

    pub fn contains(&self, p: &Point3<Real>) -> bool {
        let Self {
            length: l,
            origin: o,
        } = &self;
        (p.x >= o.x && p.x < (o.x + l))
            && (p.y >= o.y && (p.y < (o.y + l)))
            && (p.z >= o.z && (p.z < (o.z + l)))
    }

    pub fn expand_to(&mut self, p: &Point3<Real>) {
        self.origin = self.origin.inf(p);
        self.length = (self.maxs().sup(p) - self.origin).max();
    }

    pub fn center(&self) -> Point3<Real> {
        let l2 = self.length / 2.0;
        Point3::new(self.origin.x + l2, self.origin.y + l2, self.origin.z + l2)
    }

    pub fn take_octant(&self, oct: Octant) -> Self {
        use Point3 as P;
        let l2 = self.length / 2.0;
        Self {
            origin: match oct.0 {
                0 => self.origin,
                1 => P::new(self.origin.x, self.origin.y, self.origin.z + l2),
                2 => P::new(self.origin.x, self.origin.y + l2, self.origin.z),
                3 => P::new(self.origin.x, self.origin.y + l2, self.origin.z + l2),
                4 => P::new(self.origin.x + l2, self.origin.y, self.origin.z),
                5 => P::new(self.origin.x + l2, self.origin.y, self.origin.z + l2),
                6 => P::new(self.origin.x + l2, self.origin.y + l2, self.origin.z),
                7 => P::new(self.origin.x + l2, self.origin.y + l2, self.origin.z + l2),
                _ => unreachable!(),
            },
            length: l2,
        }
    }

    pub fn octant_of_unchecked(&self, p: &Point3<Real>) -> Octant {
        let o = &self.origin;
        let l2 = self.length / 2.0;
        Octant::new(p.x > o.x + l2, p.y > o.y + l2, p.z > o.z + l2)
    }

    pub fn octant_of<'err>(&self, p: &'err Point3<Real>) -> Result<Octant, Error<'err>> {
        if !self.contains(p) {
            return Err(Error::PointOutOfBounds(p));
        }
        Ok(self.octant_of_unchecked(p))
    }

    pub fn take_octant_of<'err>(&self, p: &'err Point3<Real>) -> Result<Self, Error<'err>> {
        Ok(self.take_octant(self.octant_of(p)?))
    }

    /// Given an Octant `o`, construct an AABC `n` such that `self` is the `o`th octant of `n`
    pub fn parent(&self, oct: Octant) -> Self {
        use Point3 as P;
        let l2 = self.length * 2.0;
        Self {
            origin: match oct.0 {
                0 => self.origin,
                1 => P::new(self.origin.x, self.origin.y, self.origin.z - l2),
                2 => P::new(self.origin.x, self.origin.y - l2, self.origin.z),
                3 => P::new(self.origin.x, self.origin.y - l2, self.origin.z - l2),
                4 => P::new(self.origin.x - l2, self.origin.y, self.origin.z),
                5 => P::new(self.origin.x - l2, self.origin.y, self.origin.z - l2),
                6 => P::new(self.origin.x - l2, self.origin.y - l2, self.origin.z),
                7 => P::new(self.origin.x - l2, self.origin.y - l2, self.origin.z - l2),
                _ => unreachable!(),
            },
            length: l2,
        }
    }
}

impl AABC<parry3d::math::Real> {
    pub fn aabb(&self) -> AABB {
        AABB {
            mins: self.origin,
            maxs: self.maxs(),
        }
    }
}
