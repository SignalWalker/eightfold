use std::{fmt::Display, ops::Add};

use eightfold_common::ArrayIndex;
use nalgebra::Vector3;
use num_traits::AsPrimitive;

use crate::NodePoint;

/// A way to refer to octants in a 3D volume.
///
/// # Diagram
/// `IJK>A`, where `IJK` are the octant coords, and `A` is the corresponding child array index.
/// <pre>
///     Left            Right
/// -XYZ---XYZ---   -XYZ---XYZ---          2 - 6     J
/// |000>0|010>2|   |100>4|110>6| Back   3 - 7 |     |
/// |-----|-----|   |-----|-----|        |   | 4     ___ I
/// |001>1|011>3|   |101>5|111>7| Front  1 - 5      /
/// -------------   -------------                  K
///  Lower Upper     Lower Upper
/// </pre>
///
/// So, you can think of it as being a right-handed coordinate system.
#[repr(transparent)]
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct Octant(pub u8);

impl Display for Octant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Octant").field(&self.0).finish()
    }
}

impl std::ops::Not for Octant {
    type Output = Self;
    /// Get the [Octant] opposite to `self`, as if `self` were mirrored on each axis across the
    /// center of its parent volume.
    ///
    /// ```rust
    /// assert_eq!(!Octant(7), Octant(0));
    /// assert_eq!(!Octant(6), Octant(1));
    /// assert_eq!(!Octant(5), Octant(2));
    /// assert_eq!(!Octant(4), Octant(3));
    /// assert_eq!(!Octant(3), Octant(4));
    /// assert_eq!(!Octant(2), Octant(5));
    /// assert_eq!(!Octant(1), Octant(6));
    /// assert_eq!(!Octant(0), Octant(7));
    /// ```
    #[inline]
    #[rustfmt::skip]
    fn not(self) -> Self::Output {
        Self(0b111 ^ self.0)
    }
}

impl Octant {
    pub const MIN: Self = Octant(0);
    pub const MAX: Self = Octant(7);

    /// Array of all possible [Octants](Octant).
    pub const ALL: [Self; 8] = [
        Octant(0),
        Octant(1),
        Octant(2),
        Octant(3),
        Octant(4),
        Octant(5),
        Octant(6),
        Octant(7),
    ];

    /// Construct an [Octant] from coordinates.
    #[inline]
    #[rustfmt::skip]
    pub const fn new(i: bool, j: bool, k: bool) -> Self {
        Self(
            ((i as u8) << 2)
          | ((j as u8) << 1)
          | ((k as u8) << 0)
        )
    }

    /// The `i` component of `self`.
    #[inline]
    pub const fn i(self) -> u8 {
        self.0 & 0b100
    }
    /// The `j` component of `self`.
    #[inline]
    pub const fn j(self) -> u8 {
        self.0 & 0b010
    }
    /// The `k` component of `self`.
    #[inline]
    pub const fn k(self) -> u8 {
        self.0 & 0b001
    }

    /// Get a [Vector3\<u8\>](Vector3) from [Octant] `0` to self.
    #[inline]
    pub const fn vector(self) -> Vector3<u8> {
        nalgebra::vector![self.i(), self.j(), self.k()]
    }
}

impl From<Octant> for usize {
    #[inline]
    fn from(oct: Octant) -> Self {
        oct.0 as usize
    }
}

impl From<Octant> for u8 {
    #[inline]
    fn from(oct: Octant) -> Self {
        oct.0
    }
}

macro_rules! np_add_impl {
    ($np:ident, $o:ident) => {
        $crate::nodepoint![
            $np.0.x + $o.i().as_(),
            $np.0.y + $o.j().as_(),
            $np.0.z + $o.k().as_(),
            $np.0.w + Idx::ONE
        ]
    };
}

impl<Idx: ArrayIndex> Add<Octant> for &NodePoint<Idx>
where
    u8: AsPrimitive<Idx>,
{
    type Output = NodePoint<Idx>;

    /// Get the [`NodePoint`] of an [Octant] of self
    #[inline]
    fn add(self, o: Octant) -> Self::Output {
        np_add_impl!(self, o)
    }
}

impl<Idx: ArrayIndex> Add<Octant> for NodePoint<Idx>
where
    u8: AsPrimitive<Idx>,
{
    type Output = NodePoint<Idx>;

    /// Get the [`NodePoint`] of an [Octant] of self
    #[inline]
    fn add(self, o: Octant) -> Self::Output {
        np_add_impl!(self, o)
    }
}

#[cfg(test)]
mod tests {
    use crate::Octant;

    #[test]
    #[rustfmt::skip]
    fn new() {
        assert_eq!(Octant::new(false, false, false), Octant(0));
        assert_eq!(Octant::new(false, false,  true), Octant(1));
        assert_eq!(Octant::new(false,  true, false), Octant(2));
        assert_eq!(Octant::new(false,  true,  true), Octant(3));
        assert_eq!(Octant::new( true, false, false), Octant(4));
        assert_eq!(Octant::new( true, false,  true), Octant(5));
        assert_eq!(Octant::new( true,  true, false), Octant(6));
        assert_eq!(Octant::new( true,  true,  true), Octant(7));
    }

    #[test]
    fn not() {
        assert_eq!(!Octant(7), Octant(0));
        assert_eq!(!Octant(6), Octant(1));
        assert_eq!(!Octant(5), Octant(2));
        assert_eq!(!Octant(4), Octant(3));
        assert_eq!(!Octant(3), Octant(4));
        assert_eq!(!Octant(2), Octant(5));
        assert_eq!(!Octant(1), Octant(6));
        assert_eq!(!Octant(0), Octant(7));
    }
}
