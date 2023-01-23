use eightfold_common::ArrayIndex;
use nalgebra::Point3;

use super::{Aabb, Float};

#[derive(Debug, thiserror::Error)]
pub enum Error<Idx: ArrayIndex, Real: Float> {
    #[error(transparent)]
    Octree(#[from] crate::tree::Error<Idx>),
    #[error("volume {0:?} does not contain point {1:?}")]
    PointOutOfBounds(Aabb<Real>, Point3<Real>),
}
