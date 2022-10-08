use std::ops::{Shl, ShlAssign, Shr, ShrAssign};

use eightfold_common::ArrayIndex;
use nalgebra::ClosedMul;

use crate::{Error, NodePoint, Octree};

/// Define a method for merging two leaf references into a single leaf, to allow sampling leaf data
/// over an entire branch.
pub trait LeafSample {
    /// Merge two leaf references into a single leaf.
    fn leaf_sample(a: &Self, b: &Self) -> Self;
}

impl<T: LeafSample + Clone, Idx: ArrayIndex> Octree<T, Idx> {
    fn internal_sample_branch(&self, children_idx: Idx) -> Option<T> {
        let mut res = None;
        for c in self.branch_data[children_idx.as_()] {
            match self.proxies[c.as_()].data {
                crate::ProxyData::Void => {}
                crate::ProxyData::Leaf(l_idx) => match res {
                    Some(r) => res = Some(T::leaf_sample(&r, &self.leaf_data[l_idx.as_()])),
                    None => res = Some(self.leaf_data[l_idx.as_()].clone()),
                },
                crate::ProxyData::Branch(b_idx) => {
                    let data = match self.internal_sample_branch(b_idx) {
                        Some(d) => d,
                        None => continue,
                    };
                    match res {
                        Some(r) => res = Some(T::leaf_sample(&r, &data)),
                        None => res = Some(data),
                    }
                }
            }
        }
        res
    }

    /// Return the merged leaf data of the descendants of a specific branch.
    pub fn sample_branch(&self, branch_idx: Idx) -> Result<T, Error<Idx>> {
        match self
            .proxies
            .get(branch_idx.as_())
            .ok_or(Error::InvalidIndex(branch_idx))?
            .data
        {
            crate::ProxyData::Branch(ch_idx) => self
                .internal_sample_branch(ch_idx)
                .ok_or(Error::NoLeafs(branch_idx)),
            _ => Err(Error::NotABranch(branch_idx)),
        }
    }

    /// Return the merged leaf data at a specific point.
    pub fn sample_at(&self, point: &NodePoint<Idx>) -> Option<T>
    where
        T: Clone,
        Idx: Shl<Idx, Output = Idx>
            + ShlAssign<Idx>
            + From<u8>
            + Shr<u8, Output = Idx>
            + ClosedMul
            + ShrAssign<u8>,
    {
        let mut node = self.node_at(point);
        let mut p = self.proxies[node.as_()];
        loop {
            match p.data {
                crate::ProxyData::Void => {}
                // leaf secured
                crate::ProxyData::Leaf(l_idx) => break Some(self.leaf_data[l_idx.as_()].clone()),
                // only break if we actually find something; otherwise, just go up the chain
                crate::ProxyData::Branch(b_idx) => {
                    if let Some(d) = self.internal_sample_branch(b_idx) {
                        break Some(d);
                    }
                }
            }
            if p.parent == node {
                break None;
            }
            node = p.parent;
            p = self.proxies[node.as_()];
        }
    }
}
