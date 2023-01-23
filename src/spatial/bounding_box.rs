use nalgebra::Point3;

use crate::Octant;

use super::{Aabc, Float};

#[derive(Debug, thiserror::Error)]
pub enum AabbError<Real: Float> {
    #[error("volume {0:?} does not contain point {1:?}")]
    PointOutOfBounds(Aabb<Real>, Point3<Real>),
}

/// Axis-Aligned Bounding Box
///
/// Similar to [`parry3d::Aabb`], except generic over the Real type.
#[derive(Debug, Clone, Copy)]
pub struct Aabb<Real: Float> {
    pub mins: Point3<Real>,
    pub maxs: Point3<Real>,
}

impl<Real: Float> Aabb<Real> {
    #[inline]
    pub fn new(mins: Point3<Real>, maxs: Point3<Real>) -> Self {
        Self { mins, maxs }
    }

    #[inline]
    pub fn contains(&self, p: &Point3<Real>) -> bool {
        let Self { mins: i, maxs: a } = self;
        (p.x >= i.x && p.y >= i.y && p.z >= i.z) && (p.x <= a.x && p.y <= a.y && p.z <= a.z)
    }

    /// Determine the center of `self`.
    #[inline]
    pub fn center(&self) -> Point3<Real> {
        let Self { mins: i, maxs: a } = self;
        nalgebra::point![
            (i.x + a.x) / Real::TWO,
            (i.y + a.y) / Real::TWO,
            (i.z + a.z) / Real::TWO
        ]
    }

    /// Determine the [Octant] of `p`.
    ///
    /// This still works even if `p` ∉ `self`: the result is given as if taking the octant of `p`
    /// within an infinitely-large bounding box sharing a center with `self`.
    #[inline]
    pub fn octant_of(&self, p: &Point3<Real>) -> Octant {
        Octant::from_center(&self.center(), p)
    }

    /// Construct an [Aabb] such that `self` is an octant of the result.
    #[rustfmt::skip]
    pub fn parent(&self, oct: Octant) -> Self {
        use nalgebra::point;
        let Self { mins: i, maxs: a } = self;
        let v = a - i;
        match oct.0 {
            0 => Self { mins: *i, maxs: a + v, },
            1 => Self {
                mins: point![i.x      , i.y      , i.z - v.z],
                maxs: point![a.x + v.x, a.y + v.y, a.z      ],
            },
            2 => Self {
                mins: point![i.x      , i.y - v.y, i.z      ],
                maxs: point![a.x + v.x, a.y      , a.z + v.z],
            },
            3 => Self {
                mins: point![i.x      , i.y - v.y, i.z - v.z],
                maxs: point![a.x + v.x, a.y      , a.z      ],
            },
            4 => Self {
                mins: point![i.x - v.x, i.y      , i.z      ],
                maxs: point![a.x      , a.y + v.y, a.z + v.z],
            },
            5 => Self {
                mins: point![i.x - v.x, i.y      , i.z - v.z],
                maxs: point![a.x      , a.y + v.y, a.z      ],
            },
            6 => Self {
                mins: point![i.x - v.x, i.y - v.y, i.z      ],
                maxs: point![a.x      , a.y      , a.z + v.z],
            },
            7 => Self { mins: i - v, maxs: *a, },
            _ => unreachable!(),
        }
    }

    /// Construct an [Aabb] such that the result is an octant of `self`.
    pub fn child(&self, oct: Octant) -> Self {
        use nalgebra::point;
        let Self { mins: i, maxs: a } = self;
        let c = self.center();
        match oct.0 {
            0 => Self { mins: *i, maxs: c },
            1 => Self {
                mins: point![i.x, i.y, c.z],
                maxs: point![c.x, c.y, a.z],
            },
            2 => Self {
                mins: point![i.x, c.y, i.z],
                maxs: point![c.x, a.y, c.z],
            },
            3 => Self {
                mins: point![i.x, c.y, c.z],
                maxs: point![c.x, a.y, a.z],
            },
            4 => Self {
                mins: point![c.x, i.y, i.z],
                maxs: point![a.x, c.y, c.z],
            },
            5 => Self {
                mins: point![c.x, i.y, c.z],
                maxs: point![a.x, c.y, a.z],
            },
            6 => Self {
                mins: point![c.x, c.y, i.z],
                maxs: point![a.x, a.y, c.z],
            },
            7 => Self { mins: c, maxs: *a },
            _ => unreachable!(),
        }
    }

    /// Construct an [Aabb] `n` containing both `self` and a point `p`, where `self` is a
    /// descendant octant of `n`.
    ///
    /// If `p` lies within `self`, the returned volume is `self`.
    pub fn parent_containing(self, p: &Point3<Real>) -> Self {
        let mut res = self;
        while !res.contains(p) {
            res = res.parent(res.octant_of(p));
        }
        res
    }

    /// Construct an [Aabb] `n` containing a point `p`, where `n` is an [octant](Octant) of `self`.
    ///
    /// # Safety
    ///
    /// * If `p` ∉ `self`, the result is undefined.
    #[inline]
    #[allow(unsafe_code)]
    pub unsafe fn child_containing_unchecked(&self, p: &Point3<Real>) -> (Octant, Self) {
        let oct = self.octant_of(p);
        (oct, self.child(oct))
    }

    /// Construct an [Aabb] `n` containing a point `p`, where `n` is an [octant](Octant) of `self`.
    ///
    /// # Errors
    ///
    /// * [`PointOutOfBounds`](AabbError::PointOutOfBounds) if `p` ∉ `self`.
    #[inline]
    #[allow(unsafe_code)]
    pub fn child_containing(&self, p: &Point3<Real>) -> Result<(Octant, Self), AabbError<Real>> {
        if !self.contains(p) {
            return Err(AabbError::PointOutOfBounds(*self, *p));
        }
        Ok(unsafe { self.child_containing_unchecked(p) })
    }
}

impl From<parry3d::bounding_volume::Aabb> for Aabb<parry3d::math::Real> {
    fn from(p: parry3d::bounding_volume::Aabb) -> Self {
        Self {
            mins: p.mins,
            maxs: p.maxs,
        }
    }
}

impl From<Aabb<parry3d::math::Real>> for parry3d::bounding_volume::Aabb {
    fn from(e: Aabb<parry3d::math::Real>) -> Self {
        Self {
            mins: e.mins,
            maxs: e.maxs,
        }
    }
}

impl<Real: Float> From<Aabc<Real>> for Aabb<Real> {
    fn from(cube: Aabc<Real>) -> Self {
        Self {
            mins: cube.origin,
            maxs: cube.maxs(),
        }
    }
}
