//! Additional [Octant] implementation for spatial trees

use nalgebra::point;
use parry3d::bounding_volume::AABB;

use crate::Octant;

use super::{error::Error, WorldPoint};

impl Octant {
    /// Find the Octant of a point `p` relative to another point `c`.
    pub fn from_center(c: &WorldPoint, p: &WorldPoint) -> Self {
        Self::new(p.x > c.x, p.y > c.y, p.z > c.z)
    }

    /// Find the Octant of a point `p` within an AABB `bb`.
    pub fn from_aabb<'err>(bb: &AABB, p: &'err WorldPoint) -> Result<Self, Error<'err>> {
        if !bb.contains_local_point(p) {
            return Err(Error::PointOutOfBounds(p));
        }
        Ok(Self::from_center(&bb.center(), p))
    }

    /// Construct an [AABB] such that `bb` is an octant of the result
    pub fn sup_aabb(self, bb: &AABB) -> AABB {
        let n = &bb.mins;
        let x = &bb.maxs;
        let v = x - n;
        match self.0 {
            0 => AABB {
                mins: *n,
                maxs: x + v,
            },
            1 => AABB {
                mins: point![n.x, n.y, n.z - v.z],
                maxs: point![x.x + v.x, x.y + v.y, x.z],
            },
            2 => AABB {
                mins: point![n.x, n.y - v.y, n.z],
                maxs: point![x.x + v.x, x.y, x.z + v.z],
            },
            3 => AABB {
                mins: point![n.x, n.y - v.y, n.z - v.z],
                maxs: point![x.x + v.x, x.y, x.z],
            },
            4 => AABB {
                mins: point![n.x - v.x, n.y, n.z],
                maxs: point![x.x, x.y + v.y, x.z + v.z],
            },
            5 => AABB {
                mins: point![n.x - v.x, n.y, n.z - v.z],
                maxs: point![x.x, x.y + v.y, x.z],
            },
            6 => AABB {
                mins: point![n.x - v.x, n.y - v.y, n.z],
                maxs: point![x.x, x.y, x.z + v.z],
            },
            7 => AABB {
                mins: n - v,
                maxs: *x,
            },
            _ => unreachable!(),
        }
    }

    /// Construct an [AABB] by taking an octant from an existing [AABB].
    pub fn sub_aabb(self, bb: &AABB) -> AABB {
        let c = bb.center();
        let n = &bb.mins;
        let x = &bb.maxs;
        match self.0 {
            0 => AABB {
                mins: *n, // ---
                maxs: c,
            },
            1 => AABB {
                mins: point![n.x, n.y, c.z], // --z
                maxs: point![c.x, c.y, x.z],
            },
            2 => AABB {
                mins: point![n.x, c.y, n.z], // -y-
                maxs: point![c.x, x.y, c.z],
            },
            3 => AABB {
                mins: point![n.x, c.y, c.z], // -yz
                maxs: point![c.x, x.y, x.z],
            },
            4 => AABB {
                mins: point![c.x, n.y, n.z], // x--
                maxs: point![x.x, c.y, c.z],
            },
            5 => AABB {
                mins: point![c.x, n.y, c.z], // x-z
                maxs: point![x.x, c.y, x.z],
            },
            6 => AABB {
                mins: point![c.x, c.y, n.z], // xy-
                maxs: point![x.x, x.y, c.z],
            },
            7 => AABB {
                mins: c, // xyz
                maxs: *x,
            },
            _ => unreachable!(),
        }
    }
}
