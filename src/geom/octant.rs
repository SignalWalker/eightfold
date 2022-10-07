use std::ops::Add;

use nalgebra::{ClosedAdd, Vector3};
use num_traits::AsPrimitive;

use crate::{NodePoint, TreeIndex};

/// A way to refer to octants in a 3D volume.
///
/// # Diagram
/// `IJK>A`, where `IJK` are the octant coords, and `A` is the corresponding child array index.
/// <pre>
/// Lower           Upper
/// -------------   -------------     2 - 6     J
/// |000>0|100>4|   |010>2|110>6|   3 - 7 |     |
/// |-----|-----|   |-----|-----|   |   | 4     ___ I
/// |001>1|101>5|   |011>3|111>7|   1 - 5      /
/// -------------   -------------             K
/// </pre>
///
/// So, you can think of it as being a right-handed coordinate system.
///
/// Note: Face 0-1-2 winds clockwise, on that cube.
#[repr(transparent)]
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct Octant(pub u8);

impl Octant {
    /// Array of all possible Octants.
    pub const fn all() -> [Self; 8] {
        [
            Octant(0),
            Octant(1),
            Octant(2),
            Octant(3),
            Octant(4),
            Octant(5),
            Octant(6),
            Octant(7),
        ]
    }

    /// Construct an Octant from coordinates.
    pub const fn new(i: bool, j: bool, k: bool) -> Self {
        Self((i as u8 * 0b100) | (j as u8 * 0b010) | (k as u8))
    }

    /// The `i` component of self.
    #[inline]
    pub const fn i(self) -> u8 {
        self.0 & 0b100
    }
    /// The `j` component of self.
    #[inline]
    pub const fn j(self) -> u8 {
        self.0 & 0b010
    }
    /// The `k` component of self.
    #[inline]
    pub const fn k(self) -> u8 {
        self.0 & 0b001
    }

    /// Get a [Vector3\<u8\>](Vector3) from Octant `0` to self.
    pub const fn vector(self) -> Vector3<u8> {
        nalgebra::vector![self.i(), self.j(), self.k()]
    }
}

impl From<Octant> for usize {
    fn from(oct: Octant) -> Self {
        oct.0 as usize
    }
}

impl From<Octant> for u8 {
    fn from(oct: Octant) -> Self {
        oct.0
    }
}

impl<Idx: TreeIndex + From<u8> + ClosedAdd> Add<Octant> for &NodePoint<Idx> {
    type Output = NodePoint<Idx>;

    /// Get the [NodePoint] of an [Octant] of self
    fn add(self, o: Octant) -> Self::Output {
        NodePoint::new(
            self.0.x + o.i().into(),
            self.0.y + o.j().into(),
            self.0.z + o.k().into(),
            self.0.w + 1u8.into(),
        )
    }
}

impl<Idx: TreeIndex> Add<Octant> for NodePoint<Idx>
where
    u8: AsPrimitive<Idx>,
{
    type Output = NodePoint<Idx>;

    /// Get the [NodePoint] of an [Octant] of self
    fn add(self, o: Octant) -> Self::Output {
        NodePoint::new(
            self.0.x + o.i().as_(),
            self.0.y + o.j().as_(),
            self.0.z + o.k().as_(),
            self.0.w + Idx::one(),
        )
    }
}

// impl From<Octant> for GridVector {
//     fn from(o: Octant) -> Self {
//         Self::new(
//             (o.0 & 0b100) as u32,
//             (o.0 & 0b010) as u32,
//             (o.0 & 0b001) as u32,
//         )
//     }
// }
