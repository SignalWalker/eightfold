mod error;
mod iter;
mod merge;
// mod node;
mod proxy;
mod sample;
mod slice;

mod debug;

use std::{
    collections::HashMap,
    convert::TryInto,
    ops::{Index, Range, ShlAssign, ShrAssign},
};

use eightfold_common::ArrayIndex;
pub use error::*;
pub use iter::*;
pub use merge::*;
use nalgebra::ClosedMul;
// pub use node::*;
use num_traits::AsPrimitive;
pub use proxy::*;
pub use sample::*;
pub use slice::*;
use tracing::instrument;

use crate::{stablevec::StableVec, NodePoint, Octant, VoxelPoint};

use stablevec::stablevec;

/// A data structure for partitioning data in a 3D space.
#[derive(Debug)]
pub struct Octree<T, Idx: ArrayIndex> {
    proxies: StableVec<Proxy<Idx>>,
    /// Store for indices of the children of branches
    branch_data: StableVec<[Idx; 8]>,
    leaf_data: StableVec<T>,
    root: Idx,
}

impl<T, Idx: ArrayIndex> Index<Idx> for Octree<T, Idx> {
    type Output = Proxy<Idx>;
    fn index(&self, i: Idx) -> &Self::Output {
        &self.proxies[i.as_()]
    }
}

impl<T, Idx: ArrayIndex> Default for Octree<T, Idx> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, Idx: ArrayIndex> Octree<T, Idx> {
    /// Construct a new tree with a void root.
    pub fn new() -> Self {
        Self {
            proxies: stablevec![Proxy {
                parent: Idx::ZERO,
                data: ProxyData::Void,
            }],
            branch_data: StableVec::default(),
            leaf_data: StableVec::default(),
            root: Idx::ZERO,
        }
    }

    /// The depth of the node at a specified index.
    ///
    /// # Panics
    ///
    /// * `node` ∉ `self.proxies`
    pub fn depth_of_unchecked(&self, mut node: Idx) -> Idx {
        let mut depth = Idx::ZERO;
        let mut p = self.proxies[node.as_()];
        while p.parent != node {
            depth += Idx::ONE;
            node = p.parent;
            p = self.proxies[node.as_()];
        }
        depth
    }

    /// The depth of the node at a specified index.
    pub fn depth_of(&self, node: Idx) -> Result<Idx, Error<Idx>> {
        if !self.proxies.is_init(node.as_()) {
            return Err(Error::InvalidIndex(node));
        }
        Ok(self.depth_of_unchecked(node))
    }

    /// Divide a voxel into a branch, returning the indices of the new branch's children.
    ///
    /// If the target voxel is already a branch, no changes are made and the existing branch's
    /// children are returned.
    ///
    /// The children of a branch are always stored and given in [Octant] order.
    pub fn branch(&mut self, target: Idx) -> Result<(&[Idx; 8], Proxy<Idx>), Error<Idx>>
    where
        usize: AsPrimitive<Idx>,
        Range<Idx>: Iterator,
    {
        let prox = self.proxies[target.as_()];
        match prox.data {
            ProxyData::Branch(children) => Ok((&self.branch_data[children.as_()], prox)),
            ProxyData::Leaf(_) => Err(Error::BranchCollision),
            ProxyData::Void => {
                let children: [Idx; 8] = self
                    .proxies
                    .push_iter((0..8).map(|_| Proxy {
                        parent: target,
                        data: ProxyData::Void,
                    }))
                    .map(AsPrimitive::<Idx>::as_)
                    .collect::<Vec<_>>()
                    .as_slice()
                    .try_into()
                    .unwrap();
                let c_index = self.branch_data.push(children);

                let prox = &mut self.proxies[target.as_()];
                prox.data = ProxyData::Branch(c_index.as_());

                Ok((&self.branch_data[c_index], *prox))
            }
        }
    }

    #[allow(unsafe_code)]
    fn flatten_branch(
        &mut self,
        target: Idx,
        children_idx: Idx,
        new_data: ProxyData<Idx>,
    ) -> Vec<T> {
        self.proxies[target.as_()].data = new_data;

        let mut res = Vec::with_capacity(8);
        let mut to_remove = unsafe {
            self.branch_data
                .remove_unchecked(children_idx.as_())
                .unwrap()
                .to_vec()
        };
        while let Some(c_idx) = to_remove.pop() {
            match unsafe { self.proxies.remove_unchecked(c_idx.as_()).unwrap().data } {
                ProxyData::Void => {}
                ProxyData::Leaf(t) => {
                    res.push(unsafe { self.leaf_data.remove_unchecked(t.as_()).unwrap() })
                }
                ProxyData::Branch(c_idx) => {
                    to_remove.extend_from_slice(unsafe {
                        &self.branch_data.remove_unchecked(c_idx.as_()).unwrap()
                    });
                }
            }
        }

        res
    }

    /// Clear a voxel and clean up any data it represented. Leaf data is returned as a Vec, if any
    /// is removed.
    ///
    /// If the voxel is a branch, the branch's children are voided as well.
    pub fn void(&mut self, target: Idx) -> Vec<T> {
        match self.proxies[target.as_()].data {
            ProxyData::Void => Vec::with_capacity(0),
            ProxyData::Leaf(l) => {
                self.proxies[target.as_()].data = ProxyData::Void;
                vec![self.leaf_data.remove(l.as_()).unwrap()]
            }
            ProxyData::Branch(c) => self.flatten_branch(target, c, ProxyData::Void),
        }
    }

    /// Set the leaf data of a voxel and, if extant, return its previous leaf data.
    ///
    /// If the voxel is a branch, the branch is voided first.
    pub fn set_leaf(&mut self, target: Idx, data: T) -> Vec<T>
    where
        usize: AsPrimitive<Idx>,
    {
        match self.proxies[target.as_()].data {
            ProxyData::Leaf(l) => vec![self.leaf_data.set(l.as_(), data).unwrap()],
            ProxyData::Void => {
                self.proxies[target.as_()].data = ProxyData::Leaf(self.leaf_data.push(data).as_());
                Vec::with_capacity(0)
            }
            ProxyData::Branch(ch_idx) => {
                let res = self.flatten_branch(target, ch_idx, ProxyData::Void);
                self.proxies[target.as_()].data = ProxyData::Leaf(self.leaf_data.push(data).as_());
                res
            }
        }
    }

    /// Grow a tree by adding a parent branch to the old root, and return the index of the new root.
    /// The old root becomes the [Octant] `oct` of the new root.
    #[instrument(skip(self))]
    pub fn grow(&mut self, oct: Octant) -> Idx
    where
        usize: AsPrimitive<Idx>,
    {
        let old_root = self.root;

        self.proxies.reserve(8);

        let mut children: Vec<Idx> = self
            .proxies
            .push_iter(
                std::iter::repeat(Proxy {
                    parent: self.root,
                    data: ProxyData::Void,
                })
                .take(7),
            )
            .map(usize::as_)
            .collect::<Vec<Idx>>();
        children.insert(usize::from(oct), old_root);

        self.root = self
            .proxies
            .push(Proxy {
                parent: self.root,
                data: ProxyData::Branch(
                    self.branch_data
                        .push(children.as_slice().try_into().unwrap())
                        .as_(),
                ),
            })
            .as_();
        self.proxies[old_root.as_()].parent = self.root;

        self.root
    }

    /// Code shared by [`voxel_at`] and [`voxel_at_unchecked`]
    #[inline]
    fn internal_voxel_at(&self, p: &VoxelPoint<Idx>, size: Idx) -> Idx
    where
        Idx: ShrAssign<Idx> + PartialOrd,
    {
        let mut idx: Idx = Idx::ZERO;
        let mut vox = self.proxies[self.root.as_()];
        let mut s2 = size >> Idx::ONE; // size / 2 // half of the cube occupied by vox
        while let ProxyData::Branch(ch_idx) = vox.data {
            let children = &self.branch_data[ch_idx.as_()];
            let oct = Octant::new(p.x > s2, p.y > s2, p.z > s2);
            idx = children[oct.0 as usize];
            vox = self.proxies[idx.as_()];
            s2 >>= Idx::ONE; // s2 /= 2
        }
        idx
    }

    /// Get the index of the deepest voxel containing a specific [`VoxelPoint`].
    ///
    /// *Warning*: this requires knowing the height of the tree, which can be an expensive
    /// calculation.
    ///
    /// # Panics
    /// * `p` ∉ 0..`self.grid_size()`
    pub fn voxel_at_unchecked(&self, p: &VoxelPoint<Idx>) -> Idx
    where
        Idx: ShrAssign<Idx> + PartialOrd,
    {
        self.internal_voxel_at(p, self.grid_size())
    }

    /// Get the index of the deepest voxel containing a specific [`VoxelPoint`].
    ///
    /// *Warning*: this requires knowing the height of the tree, which can be an expensive
    /// calculation.
    ///
    /// # Errors
    /// * `p` ∉ 0..`self.grid_size()`
    pub fn voxel_at(&self, p: &VoxelPoint<Idx>) -> Result<Idx, Error<Idx>>
    where
        Idx: ShrAssign<Idx> + PartialOrd,
    {
        let size = self.grid_size();
        if p.x >= size || p.y >= size || p.z >= size {
            return Err(Error::VoxelOutOfGrid(size, *p));
        }
        Ok(self.internal_voxel_at(p, size))
    }

    /// Get the index of the deepest voxel encompassing a specific [`NodePoint`].
    pub fn node_at(&self, p: &NodePoint<Idx>) -> Idx
    where
        Idx: ShlAssign<Idx> + ClosedMul + ShrAssign<Idx>,
    {
        let mut idx = Idx::ZERO;
        let mut vox = &self.proxies[self.root.as_()];
        let ps = Idx::ONE << p.0.w; // 2ʷ // grid size at depth of `p`
        let mut s2 = ps >> Idx::ONE; // ps / 2 // 1/2 the size of the `p`-grid space occupied by vox
        let psp = VoxelPoint::new(p.0.x * ps, p.0.y * ps, p.0.z * ps); // voxelpoint of target in
                                                                       // grid sized to depth of `p`
        while let ProxyData::Branch(ch_idx) = &vox.data {
            if s2 == Idx::ZERO {
                break;
            };
            let children = &self.branch_data[ch_idx.as_()];
            let oct = Octant::new(psp.x > s2, psp.y > s2, psp.z > s2);
            idx = children[oct.0 as usize];
            vox = &self.proxies[idx.as_()];
            s2 >>= Idx::ONE; // s2 /= 2
        }

        idx
    }

    /// Convert this tree to one with a wider index type.
    pub fn upcast<NIdx: ArrayIndex>(mut self) -> Octree<T, NIdx>
    where
        usize: AsPrimitive<Idx>,
        Idx: AsPrimitive<NIdx>,
    {
        // TODO :: figure out how to enforce this at compile-time
        debug_assert!(std::mem::size_of::<Idx>() <= std::mem::size_of::<NIdx>());
        self.compress();
        Octree {
            proxies: self
                .proxies
                .iter()
                .map(|p| Proxy::<NIdx> {
                    parent: p.parent.as_(),
                    data: match p.data {
                        ProxyData::Void => ProxyData::Void,
                        ProxyData::Leaf(l_idx) => ProxyData::Leaf(l_idx.as_()),
                        ProxyData::Branch(b_idx) => ProxyData::Branch(b_idx.as_()),
                    },
                })
                .collect(),
            branch_data: self
                .branch_data
                .iter()
                .map(|b| b.map(|c| c.as_()))
                .collect(),
            leaf_data: self.leaf_data,
            root: self.root.as_(),
        }
    }

    /// Clean proxy & leaf data by removing empty entries and updating stored indices.
    pub fn defragment(&mut self)
    where
        usize: AsPrimitive<Idx>,
    {
        let p_swaps = HashMap::<usize, usize>::from_iter(self.proxies.defragment());
        let l_swaps = HashMap::<usize, usize>::from_iter(self.leaf_data.defragment());
        let b_swaps = HashMap::<usize, usize>::from_iter(self.branch_data.defragment());
        for p in self.proxies.as_slice_mut().unwrap().iter_mut() {
            if let Some(&parent) = p_swaps.get(&p.parent.as_()) {
                p.parent = parent.as_();
            }
            match &mut p.data {
                ProxyData::Void => {}
                ProxyData::Leaf(ref mut l_idx) => {
                    if let Some(&leaf) = l_swaps.get(&l_idx.as_()) {
                        *l_idx = leaf.as_();
                    }
                }
                ProxyData::Branch(ref mut b_idx) => {
                    if let Some(&children) = b_swaps.get(&b_idx.as_()) {
                        *b_idx = children.as_();
                    }
                    let children = &mut self.branch_data[b_idx.as_()];
                    for c in children {
                        if let Some(&child) = p_swaps.get(&c.as_()) {
                            *c = child.as_();
                        }
                    }
                }
            }
        }
    }

    /// [Defragment](Self::defragment) & reallocate `self` such that only enough memory to store
    /// `self` is allocated.
    pub fn compress(&mut self)
    where
        usize: AsPrimitive<Idx>,
    {
        self.defragment();
        self.proxies.compress();
        self.branch_data.compress();
        self.leaf_data.compress();
    }

    /// Iterate through all leaf data, from oldest to newest.
    pub fn leaf_unordered(&self) -> impl Iterator<Item = &T> {
        self.leaf_data.iter()
    }

    /// Calculate the [`NodePoint`] of a specific node.
    ///
    /// # Panics
    ///
    /// * `index` ∉ `self.proxies`
    pub fn node_point_of_unchecked(&self, mut index: Idx) -> NodePoint<Idx> {
        let mut x = Idx::ZERO;
        let mut y = Idx::ZERO;
        let mut z = Idx::ZERO;
        let mut d = Idx::ZERO;
        let mut p = self.proxies[index.as_()];
        while p.parent != index {
            d += Idx::ONE;
            match self.proxies[p.parent.as_()].data {
                ProxyData::Branch(b_idx) => {
                    let oct = Octant(
                        self.branch_data[b_idx.as_()]
                            .into_iter()
                            .find(|&c| c == index)
                            .unwrap()
                            .as_(),
                    );
                    x += oct.i().into();
                    y += oct.j().into();
                    z += oct.k().into();
                }
                _ => unreachable!(),
            }
            index = p.parent;
            p = self.proxies[index.as_()];
        }
        NodePoint::new(x, y, z, d)
    }

    /// Calculate the [`NodePoint`] of a specific node.
    pub fn node_point_of(&self, index: Idx) -> Result<NodePoint<Idx>, Error<Idx>> {
        if !self.proxies.is_init(index.as_()) {
            return Err(Error::InvalidIndex(index));
        }
        Ok(self.node_point_of_unchecked(index))
    }

    /// [`Self::graft`], without error checks.
    ///
    /// # Panics
    ///
    /// * `node` ∉ `self.proxies`
    #[allow(unsafe_code)]
    pub fn graft_unchecked(&mut self, mut other: Self, node: Idx)
    where
        usize: AsPrimitive<Idx>,
    {
        // remove other.root from other and replace `node` with it
        let o_root = unsafe { other.proxies.remove_unchecked(other.root.as_()) }.unwrap();
        self.proxies[node.as_()].data = o_root.data;

        // insert all the new data into self, collecting swap index information
        let l_swaps = self.leaf_data.extend_from_other(other.leaf_data);
        let b_swaps = self.branch_data.extend_from_other(other.branch_data);
        let mut p_swaps = self.proxies.extend_from_other(other.proxies);
        // ensure the old root is properly handled, because we took it from `other` earlier
        p_swaps.insert(other.root.as_(), node.as_());

        let mut node_stack = vec![(node.as_(), self.proxies[node.as_()])];

        while let Some((i, p)) = node_stack.pop() {
            // replace p.parent, unless p == `node` (which already has the correct parent)
            if i != AsPrimitive::<usize>::as_(node) {
                self.proxies[i].parent = p_swaps[&p.parent.as_()].as_();
            }
            match p.data {
                // no need to update void data
                ProxyData::Void => {}
                // replace old leaf index with new leaf index
                ProxyData::Leaf(l_idx) => {
                    self.proxies[i].data = ProxyData::Leaf(l_swaps[&l_idx.as_()].as_())
                }
                ProxyData::Branch(b_idx) => {
                    // update child indices and add them to the update queue
                    let c_idx = b_swaps[&b_idx.as_()];
                    for c in &mut self.branch_data[c_idx] {
                        let ci = c.as_();
                        // update child index in children
                        *c = p_swaps[&ci].as_();
                        // add child to the update queue
                        node_stack.push((ci, self.proxies[ci]));
                    }
                    // replace old children index with new children index
                    self.proxies[i].data = ProxyData::Branch(c_idx.as_());
                }
            }
        }
    }

    /// Merge another tree as a branch of this tree.
    ///
    /// # Errors
    ///
    /// * [`InvalidIndex`](Error::InvalidIndex) if `node` is not a valid index into `self.proxies`.
    /// * [`NotAVoid`](Error::NotAVoid) if `node` is not the index of a [`ProxyData::Void`] node.
    pub fn graft(&mut self, other: Self, node: Idx) -> Result<(), Error<Idx>>
    where
        usize: AsPrimitive<Idx>,
    {
        if !matches!(
            self.proxies
                .get(node.as_())
                .ok_or(Error::InvalidIndex(node))?
                .data,
            ProxyData::Branch(_)
        ) {
            return Err(Error::NotAVoid(node));
        }
        self.graft_unchecked(other, node);
        Ok(())
    }
}
