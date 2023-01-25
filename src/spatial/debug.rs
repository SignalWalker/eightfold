use std::fmt::Display;

use eightfold_common::ArrayIndex;
use num_traits::AsPrimitive;

use super::{Float, VoxelOctree};

impl<Real: Float, T: std::fmt::Debug, Idx: ArrayIndex> Display for VoxelOctree<T, Real, Idx>
where
    u8: AsPrimitive<Idx>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.base)
    }
}
