mod error;
mod iter;
mod merge;
mod proxy;
mod sample;
mod slice;

use std::{
    collections::HashMap,
    convert::TryInto,
    ops::{AddAssign, Range, Shl, ShlAssign, Shr, ShrAssign},
};

pub use error::*;
pub use iter::*;
pub use merge::*;
use nalgebra::ClosedMul;
use num_traits::{AsPrimitive, PrimInt};
pub use proxy::*;
pub use sample::*;
pub use slice::*;

use crate::{stablevec, vec::StableVec, NodePoint, Octant, VoxelPoint};
use parking_lot::RwLock;

// TODO :: convert to trait alias once https://github.com/rust-lang/rfcs/pull/1733 is stabilized
/// Trait alias for types which can act as indices within an [Octree].
pub trait TreeIndex:
    PrimInt
    + AsPrimitive<usize>
    + AsPrimitive<u8>
    + Shl<Self, Output = Self>
    + std::fmt::Debug
    + 'static
{
}
impl<P> TreeIndex for P where
    P: PrimInt
        + AsPrimitive<usize>
        + AsPrimitive<u8>
        + Shl<Self, Output = Self>
        + std::fmt::Debug
        + 'static
{
}

/// A data structure for partitioning data in a 3D space.
#[derive(Debug)]
pub struct Octree<T, Idx: TreeIndex> {
    proxies: StableVec<Proxy<Idx>>,
    branch_data: StableVec<[Idx; 8]>,
    leaf_data: StableVec<T>,
    root: Idx,
    height_cache: RwLock<Option<Idx>>,
}

// impl<T: Clone, Idx: TreeIndex> Clone for Octree<T, Idx> {
//     fn clone(&self) -> Self {
//         Self {
//             proxies: self.proxies.clone(),
//             branch_data: self.branch_data.clone(),
//             leaf_data: self.leaf_data.clone(),
//             root: self.root,
//             height_cache: RwLock::new(*self.height_cache.read()),
//         }
//     }
// }

impl<T, Idx: TreeIndex> Default for Octree<T, Idx> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, Idx: TreeIndex> Octree<T, Idx> {
    /// The maximum grid size representable by this Octree
    pub fn max_grid_size() -> Idx {
        Idx::max_value()
    }

    /// Construct a new tree with a void root.
    pub fn new() -> Self {
        Self {
            proxies: stablevec![Proxy {
                parent: Idx::zero(),
                data: ProxyData::Void,
            }],
            branch_data: StableVec::default(),
            leaf_data: StableVec::default(),
            root: Idx::zero(),
            height_cache: Default::default(),
        }
    }

    /// The depth of the node at a specified index.
    ///
    /// # Panics
    ///
    /// * `node` ∉ `self.proxies`
    pub fn depth_of_unchecked(&self, mut node: Idx) -> Idx {
        let mut depth = Idx::zero();
        let mut p = self.proxies[node.as_()];
        while p.parent != node {
            depth = depth + Idx::one();
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
    pub fn branch(&mut self, target: Idx) -> Result<&[Idx; 8], Error<Idx>>
    where
        usize: AsPrimitive<Idx>,
        u8: AsPrimitive<Idx>,
        Range<Idx>: Iterator,
    {
        match self.proxies[target.as_()].data {
            ProxyData::Branch(children) => Ok(&self.branch_data[children.as_()]),
            ProxyData::Leaf(_) => Err(Error::BranchCollision),
            ProxyData::Void => {
                let children: [Idx; 8] = self
                    .proxies
                    .push_iter((0..8).map(|_| Proxy {
                        parent: target,
                        data: ProxyData::Void,
                    }))
                    .collect::<Vec<_>>()
                    .as_slice()
                    .try_into()
                    .unwrap();
                let c_index = self.branch_data.push(children);
                self.proxies[target.as_()].data = ProxyData::Branch(c_index.as_());
                // TODO :: more efficient way to keep track of tree height
                if let Some(height) = *self.height_cache.read() {
                    let pdepth = self.depth_of_unchecked(target);
                    self.height_cache.write().replace(height.max(pdepth));
                }
                Ok(&self.branch_data[c_index])
            }
        }
    }

    fn flatten_branch(
        &mut self,
        target: Idx,
        children_idx: Idx,
        new_data: ProxyData<Idx>,
    ) -> Vec<T> {
        self.height_cache.write().take();
        self.proxies[target.as_()].data = new_data;

        let mut res = Vec::with_capacity(8);
        let mut to_remove = unsafe {
            self.branch_data
                .remove_at_unchecked(children_idx.as_())
                .unwrap()
                .to_vec()
        };
        while let Some(c_idx) = to_remove.pop() {
            match unsafe { self.proxies.remove_at_unchecked(c_idx.as_()).unwrap().data } {
                ProxyData::Void => {}
                ProxyData::Leaf(t) => {
                    res.push(unsafe { self.leaf_data.remove_at_unchecked(t.as_()).unwrap() })
                }
                ProxyData::Branch(c_idx) => {
                    to_remove.extend_from_slice(unsafe {
                        &self.branch_data.remove_at_unchecked(c_idx.as_()).unwrap()
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
            ProxyData::Leaf(l) => vec![self.leaf_data.replace(l.as_(), data).unwrap()],
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

    /// Grow a tree by adding a parent branch to the old root. The old root becomes the `oct`th
    /// child of the new root.
    ///
    /// # See Also
    /// * [Octant]
    pub fn grow(&mut self, oct: Octant) -> Result<Idx, Error<Idx>>
    where
        u8: AsPrimitive<Idx>,
        usize: AsPrimitive<Idx>,
    {
        let old_root = self.root;
        if let Some(h) = self.height_cache.write().as_mut() {
            *h = *h + Idx::one();
        }

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
            .collect::<Vec<Idx>>();
        children.insert(oct.0 as usize, old_root);
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

        Ok(self.root)
    }

    /// Code shared by [at] and [at_unchecked]
    fn internal_voxel_at(&self, p: &VoxelPoint<Idx>, size: Idx) -> Idx
    where
        Idx: Shr<u8, Output = Idx> + ShrAssign<u8> + PartialOrd + From<u8>,
    {
        let mut idx: Idx = 0u8.into();
        let mut vox = self.proxies[self.root.as_()];
        let mut s2 = size >> 1u8; // size / 2 // half of the cube occupied by vox
        while let ProxyData::Branch(ch_idx) = vox.data {
            let children = &self.branch_data[ch_idx.as_()];
            let oct = Octant::new(p.x > s2, p.y > s2, p.z > s2);
            idx = children[oct.0 as usize];
            vox = self.proxies[idx.as_()];
            s2 >>= 1u8; // s2 /= 2
        }
        idx
    }

    /// Get the index of the deepest voxel encompassing a specific [VoxelPoint].
    ///
    /// # Panics
    /// * `p` ∉ 0..`self.grid_size()`
    pub fn voxel_at_unchecked(&self, p: &VoxelPoint<Idx>) -> Idx
    where
        Idx: Shr<u8, Output = Idx> + ShrAssign<u8> + PartialOrd + From<u8> + Shl<Idx, Output = Idx>,
    {
        self.internal_voxel_at(p, self.grid_size())
    }

    /// Get the index of the deepest voxel encompassing a specific [VoxelPoint].
    ///
    /// # Errors
    /// * `p` ∉ 0..`self.grid_size()`
    pub fn voxel_at<'data>(&self, p: &'data VoxelPoint<Idx>) -> Result<Idx, Error<'data, Idx>>
    where
        Idx: Shr<u8, Output = Idx> + ShrAssign<u8> + PartialOrd + From<u8> + Shl<Idx, Output = Idx>,
    {
        let size = self.grid_size();
        if p.x >= size || p.y >= size || p.z >= size {
            return Err(Error::VoxelOutOfGrid(size, p));
        }
        Ok(self.internal_voxel_at(p, size))
    }

    /// Get the index of the deepest voxel encompassing a specific [NodePoint].
    pub fn node_at(&self, p: &NodePoint<Idx>) -> Idx
    where
        Idx: Shl<Idx, Output = Idx>
            + ShlAssign<Idx>
            + From<u8>
            + Shr<u8, Output = Idx>
            + ClosedMul
            + ShrAssign<u8>,
    {
        let mut idx = Idx::zero();
        let mut vox = &self.proxies[self.root.as_()];
        let ps = Idx::one() << p.0.w; // 2ʷ // grid size at depth of `p`
        let mut s2 = ps >> 1u8; // ps / 2 // 1/2 the size of the `p`-grid space occupied by vox
        let psp = VoxelPoint::new(p.0.x * ps, p.0.y * ps, p.0.z * ps); // voxelpoint of target in
                                                                       // grid sized to depth of `p`
        while let ProxyData::Branch(ch_idx) = &vox.data {
            if s2.is_zero() {
                break;
            };
            let children = &self.branch_data[ch_idx.as_()];
            let oct = Octant::new(psp.x > s2, psp.y > s2, psp.z > s2);
            idx = children[oct.0 as usize];
            vox = &self.proxies[idx.as_()];
            s2 >>= 1; // s2 /= 2
        }

        idx
    }

    /// Convert this tree to one with a wider index type.
    pub fn upcast<NIdx: TreeIndex>(mut self) -> Octree<T, NIdx>
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
            height_cache: RwLock::new(self.height_cache.read().map(|h| h.as_())),
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

    /// [Defragment](Self::defragment) & reallocate self such that only enough memory to store
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

    pub fn node_point_of_unchecked(&self, mut index: Idx) -> NodePoint<Idx>
    where
        u8: AsPrimitive<Idx>,
    {
        let mut x = Idx::zero();
        let mut y = Idx::zero();
        let mut z = Idx::zero();
        let mut d = Idx::zero();
        let mut p = self.proxies[index.as_()];
        while p.parent != index {
            d = d + Idx::one();
            match self.proxies[p.parent.as_()].data {
                ProxyData::Branch(b_idx) => {
                    let oct = Octant(
                        self.branch_data[b_idx.as_()]
                            .into_iter()
                            .find(|&c| c == index)
                            .unwrap()
                            .as_(),
                    );
                    x = x + oct.i().as_();
                    y = y + oct.j().as_();
                    z = z + oct.k().as_();
                }
                _ => unreachable!(),
            }
            index = p.parent;
            p = self.proxies[index.as_()];
        }
        NodePoint::new(x, y, z, d)
    }

    /// Calculate the [NodePoint] of a specific node.
    pub fn node_point_of(&self, index: Idx) -> Result<NodePoint<Idx>, Error<Idx>>
    where
        u8: AsPrimitive<Idx>,
    {
        if !self.proxies.is_init(index.as_()) {
            return Err(Error::InvalidIndex(index));
        }
        Ok(self.node_point_of_unchecked(index))
    }

    /// [Self::graft], without error checks.
    ///
    /// # Panics
    ///
    /// * `node` ∉ `self.proxies`
    pub fn graft_unchecked(&mut self, mut other: Self, node: Idx)
    where
        usize: AsPrimitive<Idx>,
    {
        // remove other.root from other and replace `node` with it
        let o_root = unsafe { other.proxies.remove_at_unchecked(other.root.as_()) }.unwrap();
        self.proxies[node.as_()].data = o_root.data;

        // insert all the new data into self, collecting swap index information
        let l_swaps = self.leaf_data.extend_from_other(other.leaf_data);
        let b_swaps = self.branch_data.extend_from_other(other.branch_data);
        let mut p_swaps = self.proxies.extend_from_other(other.proxies);
        // ensure the old root is properly handled, because we took it from `other` earlier
        p_swaps.insert(other.root.as_(), node.as_());

        // might as well calculate depth here, too, since we're doing a depth recursion anyway
        let mut max_depth = self.depth_of_unchecked(node);
        let mut node_stack = vec![(node.as_(), self.proxies[node.as_()], max_depth)];

        while let Some((i, p, d)) = node_stack.pop() {
            // update max_depth
            if d > max_depth {
                max_depth = d;
            }
            // replace p.parent, unless p == `node` (which already has the correct parent)
            if i != node.as_() {
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
                        node_stack.push((ci, self.proxies[ci], d + Idx::one()));
                    }
                    // replace old children index with new children index
                    self.proxies[i].data = ProxyData::Branch(c_idx.as_());
                }
            }
        }

        // update the height cache
        let mut h_cache = self.height_cache.write();
        // we only know whether we've actually found the deepest node if height_cache already has a
        // value
        if let Some(h) = *h_cache {
            *h_cache = Some(h.max(max_depth))
        }
    }

    /// Merge another tree as a branch of this tree.
    ///
    /// `node` must be the index of a [ProxyData::Void] node.
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
