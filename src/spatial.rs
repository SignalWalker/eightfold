//! [Octrees](crate::Octree) with a defined relation to a 3D space.

mod bounding_cube;
mod debug;
use std::ops::Range;

pub use bounding_cube::*;
mod bounding_box;
pub use bounding_box::*;
mod error;
pub use error::*;
pub(crate) mod macros;
mod octant;
mod traits;
use num_traits::AsPrimitive;
use tracing::instrument;
pub use traits::*;

use eightfold_common::ArrayIndex;
use nalgebra::{Point3, Vector3};

use crate::{Octant, Octree, OctreeSlice, Proxy, ProxyData};

/// An [Octree] indexing a defined voxel space.
#[derive(Debug)]
pub struct VoxelOctree<T, Real: Float, Idx: ArrayIndex> {
    base: Octree<T, Idx>,
    /// The number of branches between the root and the voxel grid.
    height: Idx,
    /// The dimensions of a single voxel.
    voxel_size: Vector3<Real>,
    /// The bounding volume of this Octree.
    aabb: Aabb<Real>,
}

impl<T, Real: Float, Idx: ArrayIndex> VoxelOctree<T, Real, Idx> {
    /// Construct a [`VoxelTree`] encompassing the voxel at [0,0,0].
    pub fn new(voxel_size: Vector3<Real>) -> Self {
        Self {
            base: Octree::new(),
            height: Idx::ZERO,
            voxel_size,
            aabb: Aabb {
                mins: nalgebra::point![Real::ZERO, Real::ZERO, Real::ZERO],
                maxs: voxel_size.into(),
            },
        }
    }

    #[inline]
    pub fn aabb(&self) -> &Aabb<Real> {
        &self.aabb
    }

    /// Whether the space bounded by `self` contains a point `p`.
    #[inline]
    pub fn contains(&self, p: &Point3<Real>) -> bool {
        self.aabb.contains(p)
    }

    /// Grow `self` such that the current root becomes the [Octant] `oct` of the new root, and
    /// return the index of the new root.
    pub fn grow(&mut self, oct: Octant) -> Idx
    where
        usize: AsPrimitive<Idx>,
    {
        self.height += Idx::ONE;
        self.aabb = self.aabb.parent(oct);
        self.base.grow(oct)
    }

    /// Grow `self` until it contains a point `p`, and return whether the size of `self` changed.
    ///
    /// Does nothing if `self` already contains `p`.
    #[instrument(skip(self))]
    pub fn grow_to_contain(&mut self, p: &Point3<Real>) -> bool
    where
        usize: AsPrimitive<Idx>,
    {
        let old_aabb = self.aabb;
        let mut grew = false;
        while !self.contains(p) {
            self.grow(!self.aabb.octant_of(p));
            grew = true;
        }
        if grew {
            tracing::trace!(?old_aabb, new_aabb = ?self.aabb, "grew tree");
        }
        grew
    }

    /// Grow `self` until it contains a bounding volume `vol`, and return whether the size of `self`
    /// changed.
    ///
    /// Does nothing if `self` already contains `vol`.
    #[inline]
    pub fn grow_to_contain_aabb(&mut self, vol: &Aabb<Real>) -> bool
    where
        usize: AsPrimitive<Idx>,
    {
        let g1 = self.grow_to_contain(&vol.mins);
        self.grow_to_contain(&vol.maxs) || g1
    }

    /// Get the index of the deepest node containing a given [point](Point3) `p`.
    ///
    /// # Errors
    ///
    /// * [`PointOutOfBounds`](Error::PointOutOfBounds) if `p` ∉ `self`.
    #[inline]
    #[allow(unsafe_code)]
    pub fn node_containing(
        &self,
        p: &Point3<Real>,
    ) -> Result<(Aabb<Real>, Idx, Proxy<Idx>, Idx), Error<Idx, Real>> {
        if !self.aabb.contains(p) {
            return Err(Error::PointOutOfBounds(self.aabb, *p));
        }
        Ok(unsafe { self.node_containing_unchecked(p) })
    }

    /// Get the index of the deepest node containing a given [point](Point3) `p`.
    ///
    /// # Safety
    ///
    /// * When `p` ∉ `self`, the result is undefined.
    #[inline]
    #[allow(unsafe_code)]
    pub unsafe fn node_containing_unchecked(
        &self,
        p: &Point3<Real>,
    ) -> (Aabb<Real>, Idx, Proxy<Idx>, Idx) {
        let branch_data = self.base.branch_data();
        let mut depth = Idx::ZERO;
        let mut oct;
        let mut idx: Idx = self.base.root_idx();
        let mut prox = self.base.get(idx);
        let mut aabb = self.aabb;
        while let ProxyData::Branch(ch_idx) = prox.data {
            (oct, aabb) = unsafe { aabb.child_containing_unchecked(p) };
            idx = branch_data[ch_idx.as_()][oct.0 as usize];
            prox = self.base.get(idx);
            depth += Idx::ONE;
        }
        (aabb, idx, prox, depth)
    }

    /// Insert data into a leaf node encompassing the voxel at point `p`.
    ///
    /// # Errors
    ///
    /// * [PointOutOfBounds](Error::PointOutOfBounds) if `p` ∉ `self`.
    #[allow(unsafe_code)]
    pub fn insert_voxel_at(
        &mut self,
        p: &Point3<Real>,
        data: T,
    ) -> Result<(Idx, Proxy<Idx>, Option<T>), Error<Idx, Real>>
    where
        usize: AsPrimitive<Idx>,
        Range<Idx>: Iterator,
    {
        let mut oct;
        let (mut aabb, mut idx, mut prox, mut depth) = self.node_containing(p)?;
        while depth != self.height {
            // safety: `node_containing` already confirmed that `p` lies within `aabb`
            (oct, aabb) = unsafe { aabb.child_containing_unchecked(p) };
            (idx, prox) = {
                let (children, prox) = self.base.branch(idx)?;
                (children[oct.0 as usize], prox)
            };
            depth += Idx::ONE;
        }
        Ok((idx, prox, self.base.set_leaf(idx, data).pop()))
    }
}

// pub struct SpatialOctree<T, Idx: ArrayIndex> {
//     base: Octree<T, Idx>,
// }

// /// An [Octree] with an associated injective transformation such that each node represents a volume in 3D space unique among nodes at the same depth.
// pub struct ProjectiveOctree<T, Idx: TreeIndex> {
//     base: Octree<T, Idx>,
//     trans_into: Projective3<Real>,
//     trans_from: Projective3<Real>,
// }

// impl<T, Idx: TreeIndex> ProjectiveOctree<T, Idx> {
//     pub fn new() -> Self {
//         todo!()
//     }
// }
