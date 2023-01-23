use eightfold_common::ArrayIndex;
use num_traits::AsPrimitive;

use crate::{Error, Octree, ProxyData};

/// Define how to merge two leaves at the same depth into a single instance, to allow collapsing [Octree] branches.
pub trait LeafMerge: Sized {
    /// Merge two leafs into a single leaf.
    fn leaf_merge(a: Self, b: Self) -> Self;
}

impl<T: LeafMerge, Idx: ArrayIndex> Octree<T, Idx> {
    #[allow(unsafe_code)]
    fn internal_merge_branch(&mut self, children_idx: Idx) -> Option<T> {
        debug_assert!(self.branch_data.is_init(children_idx.as_()));
        let mut res: Option<T> = None;
        for c in unsafe { self.branch_data.remove_unchecked(children_idx.as_()) }.unwrap() {
            match unsafe { self.proxies.remove_unchecked(c.as_()) }
                .unwrap()
                .data
            {
                ProxyData::Void => {}
                ProxyData::Leaf(l_idx) => match res {
                    Some(r) => {
                        res = Some(T::leaf_merge(
                            r,
                            unsafe { self.leaf_data.remove_unchecked(l_idx.as_()) }.unwrap(),
                        ))
                    }
                    None => {
                        res = Some(unsafe { self.leaf_data.remove_unchecked(l_idx.as_()) }.unwrap())
                    }
                },
                ProxyData::Branch(b_idx) => {
                    let data = match self.internal_merge_branch(b_idx) {
                        Some(d) => d,
                        None => continue,
                    };
                    match res {
                        Some(r) => res = Some(T::leaf_merge(r, data)),
                        None => res = Some(data),
                    }
                }
            }
        }
        res
    }

    /// Collapse a branch into a leaf by merging all descendant leaf data.
    ///
    /// All merges occur between leaf data at the same depth.
    pub fn merge_branch(&mut self, branch_idx: Idx) -> Result<&T, Error<Idx>>
    where
        usize: AsPrimitive<Idx>,
    {
        match self
            .proxies
            .get(branch_idx.as_())
            .ok_or(Error::InvalidIndex(branch_idx))?
            .data
        {
            crate::ProxyData::Branch(ch_idx) => match self.internal_merge_branch(ch_idx) {
                Some(r) => {
                    let l_idx = self.leaf_data.push(r);
                    self.proxies[branch_idx.as_()].data = ProxyData::Leaf(l_idx.as_());
                    Ok(&self.leaf_data[l_idx])
                }
                None => {
                    self.proxies[branch_idx.as_()].data = ProxyData::Void;
                    Err(Error::NoLeafs(branch_idx))
                }
            },
            _ => Err(Error::NotABranch(branch_idx)),
        }
    }
}

impl<T> LeafMerge for Vec<T> {
    fn leaf_merge(mut a: Self, mut b: Self) -> Self {
        a.append(&mut b);
        a
    }
}
