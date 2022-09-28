//! Octree rewrite partially inspired by parry's QBVH type (that one partitions space into
//! multiples of 4 rather than 8, though)
#![cfg_attr(not(debug_assertions), warn(missing_docs))]

pub mod error;
mod octant;
#[cfg(feature = "spatial")]
pub mod spatial;
// pub mod slice;
// pub mod view;
pub use octant::*;

use parking_lot::RwLock;

use std::iter::FusedIterator;

use error::Error;
use nalgebra::{Point3, Point4};

/// Index type of nodes within an octree
pub type ProxyIndex = u32;

/// Index type of leaf data within an octree
pub type LeafIndex = u32;

/// The coordinates of a voxel within an octree's voxel grid.
pub type VoxelPoint = Point3<u32>;

/// The coordinates of a node within an octree, including its depth. { X, Y, Z, D }
///
/// In voxel terms, a NodePoint is a point `XYZ` within a voxel grid of size `2ᴰ`.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct NodePoint(Point4<u32>);

impl NodePoint {
    #[inline]
    pub fn new(x: u32, y: u32, z: u32, d: u32) -> Self {
        Self(Point4::new(x, y, z, d))
    }
}

#[derive(Debug, Clone)]
pub struct Proxy {
    parent: ProxyIndex,
    data: ProxyData,
    index: ProxyIndex,
}

#[derive(Debug, Clone)]
pub enum ProxyData {
    Void,
    Leaf(LeafIndex),
    Branch([ProxyIndex; 8]),
}

impl Proxy {
    pub fn parent_unchecked<'tree, T>(&self, tree: &'tree Octree<T>) -> &'tree Proxy {
        tree.proxies[self.parent as usize].as_ref().unwrap()
    }

    #[inline]
    pub fn has_parent(&self) -> bool {
        self.parent != self.index
    }

    /// Get a child index from octant coordinates.
    ///
    /// # Panics
    /// Will panic if `self.data` is not a ProxyData::Branch.
    #[inline]
    pub fn child_unchecked(&self, oct: Octant) -> ProxyIndex {
        match self.data {
            ProxyData::Branch(children) => children[oct.0 as usize],
            _ => panic!(),
        }
    }

    /// Get a child index from octant coordinates.
    pub fn child(&self, oct: Octant) -> Result<ProxyIndex, Error> {
        match self.data {
            ProxyData::Branch(children) => Ok(children[oct.0 as usize]),
            _ => Err(Error::NoChildren(self.index)),
        }
    }

    /// Get a child index from a raw octant specifier.
    ///
    /// # Panics
    /// Will panic if `self.data` is not a ProxyData::Branch, or if `index` ∉ 0..8
    #[inline]
    pub fn get_unchecked(&self, index: u8) -> ProxyIndex {
        match self.data {
            ProxyData::Branch(children) => children[index as usize],
            _ => panic!(),
        }
    }

    /// The depth of this node; i.e. the number of ancestors of this node.
    pub fn depth<T>(&self, tree: &Octree<T>) -> u32 {
        if self.parent == self.index {
            0
        } else {
            self.parent_unchecked(tree).depth(tree) + 1
        }
    }

    /// If this is the child of another node, get the octant it occupies.
    pub fn octant<T>(&self, tree: &Octree<T>) -> Option<Octant> {
        if self.parent == self.index {
            None
        } else {
            match tree.proxies[self.index as usize].as_ref().unwrap().data {
                ProxyData::Branch(children) => {
                    children.into_iter().enumerate().find_map(|(i, pi)| {
                        if pi == self.index {
                            Some(Octant(i as u8))
                        } else {
                            None
                        }
                    })
                }
                _ => unreachable!(),
            }
        }
    }
}

#[derive(Debug)]
pub struct Octree<T> {
    proxies: Vec<Option<Proxy>>,
    leaf_data: Vec<Option<T>>,
    root: ProxyIndex,
    height_cache: RwLock<Option<u32>>,
}

impl<T: Clone> Clone for Octree<T> {
    fn clone(&self) -> Self {
        Self {
            proxies: self.proxies.clone(),
            leaf_data: self.leaf_data.clone(),
            root: self.root,
            height_cache: RwLock::new(*self.height_cache.read()),
        }
    }
}

impl<T> Default for Octree<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Octree<T> {
    /// Construct a new tree with a void root.
    pub fn new() -> Self {
        Self {
            // aabc,
            proxies: vec![Some(Proxy {
                parent: 0,
                index: 0,
                data: ProxyData::Void,
            })],
            leaf_data: Vec::default(),
            root: 0,
            height_cache: Default::default(),
        }
    }

    /// The number of divisions within this octree.
    ///
    /// This can be used to determine the dimensions of the voxel grid represented by the tree.
    ///
    /// # Examples
    ///
    /// If `self` consists only of one node (the root), its height is 0.
    /// If the root is a branch, and each child of the root is a leaf (or void), then the tree's height is 1.
    /// If the root branch has a terminal child branch, the tree's height is 2.
    pub fn height(&self) -> u32 {
        match *self.height_cache.read() {
            Some(h) => h,
            None => {
                todo!()
            }
        }
    }

    /// The dimensions of the cubical voxel grid represented by this tree, as determined by the
    /// tree's [height].
    pub fn grid_size(&self) -> u32 {
        1 << self.height() // 2ʰ
    }

    /// Divide a voxel into a branch, returning the indices of the new branch's children.
    ///
    /// If the target voxel is already a branch, no changes are made and the existing branch's
    /// children are returned.
    ///
    /// The children of a branch are always stored and given in [Octant] order.
    pub fn branch(&mut self, target: ProxyIndex) -> Result<[ProxyIndex; 8], Error> {
        match self.proxies[target as usize].as_ref().unwrap().data {
            ProxyData::Branch(children) => Ok(children),
            ProxyData::Leaf(_) => Err(Error::BranchCollision),
            ProxyData::Void => {
                let start = self.proxies.len() as ProxyIndex;
                let children = [
                    start,
                    start + 1,
                    start + 2,
                    start + 3,
                    start + 4,
                    start + 5,
                    start + 6,
                    start + 7,
                ];
                self.proxies[target as usize].as_mut().unwrap().data = ProxyData::Branch(children);
                self.proxies.extend((start..start + 7).map(|idx| {
                    Some(Proxy {
                        parent: target,
                        data: ProxyData::Void,
                        index: idx,
                    })
                }));
                // TODO :: more efficient way to keep track of tree height
                if let Some(height) = *self.height_cache.read() {
                    let pdepth = self.proxies[target as usize].as_ref().unwrap().depth(self);
                    self.height_cache.write().replace(height.max(pdepth));
                }
                Ok(children)
            }
        }
    }

    /// Clear a voxel and clean up any data it represented.
    ///
    /// If the voxel is a branch, the branch's children are voided as well.
    pub fn void(&mut self, target: ProxyIndex) {
        match self.proxies[target as usize].as_ref().unwrap().data {
            ProxyData::Void => (),
            ProxyData::Leaf(l) => {
                self.leaf_data[l as usize].take();
                self.proxies[target as usize].as_mut().unwrap().data = ProxyData::Void;
            }
            ProxyData::Branch(children) => {
                // TODO :: more efficient way to keep track of tree height
                self.height_cache.write().take();
                for child in children {
                    self.void(child);
                    self.proxies[child as usize].take();
                }
                self.proxies[target as usize].as_mut().unwrap().data = ProxyData::Void;
            }
        }
    }

    /// Set the leaf data of a voxel and, if extant, return its previous leaf data.
    ///
    /// If the voxel is a branch, the branch is voided first.
    pub fn set_leaf(&mut self, target: ProxyIndex, data: T) -> Option<T> {
        match self.proxies[target as usize].as_ref().unwrap().data {
            ProxyData::Leaf(l) => self.leaf_data[l as usize].replace(data),
            ProxyData::Void => {
                let leaf_idx = self.leaf_data.len() as LeafIndex;
                self.leaf_data.push(Some(data));
                self.proxies[target as usize].as_mut().unwrap().data = ProxyData::Leaf(leaf_idx);
                None
            }
            ProxyData::Branch(children) => {
                for child in children {
                    self.void(child);
                    self.proxies[child as usize].take();
                }
                let leaf_idx = self.leaf_data.len() as LeafIndex;
                self.leaf_data.push(Some(data));
                self.proxies[target as usize].as_mut().unwrap().data = ProxyData::Leaf(leaf_idx);
                None
            }
        }
    }

    /// Grow a tree by adding a parent branch to the old root. The old root becomes the `oct`th
    /// child of the new root.
    ///
    /// # See Also
    /// * [Octant]
    pub fn grow(&mut self, oct: Octant) -> Result<ProxyIndex, Error> {
        let old_root = self.root;

        self.root = self.proxies.len() as ProxyIndex;
        self.height_cache.write().map(|h| h + 1);
        self.proxies[old_root as usize].as_mut().unwrap().parent = self.root;

        let mut children = [
            self.root + 1,
            self.root + 2,
            self.root + 3,
            self.root + 4,
            self.root + 5,
            self.root + 6,
            self.root + 7,
            self.root + 8,
        ];
        children[oct.0 as usize] = old_root;
        self.proxies.push(Some(Proxy {
            parent: self.root,
            data: ProxyData::Branch(children),
            index: self.root,
        }));

        for (i, child) in children.into_iter().enumerate() {
            if i == oct.0 as usize {
                continue;
            }
            self.proxies.push(Some(Proxy {
                parent: self.root,
                data: ProxyData::Void,
                index: child,
            }));
        }

        Ok(self.root)
    }

    /// Code shared by [at] and [at_unchecked]
    fn internal_voxel_at(&self, p: &VoxelPoint, size: u32) -> ProxyIndex {
        let mut vox = self.proxies[self.root as usize].as_ref().unwrap();
        let mut s2 = size >> 1; // size / 2 // half of the cube occupied by vox
        while let ProxyData::Branch(children) = vox.data {
            let oct = Octant::new(p.x > s2, p.y > s2, p.z > s2);
            vox = self.proxies[children[oct.0 as usize] as usize]
                .as_ref()
                .unwrap();
            s2 >>= 1; // s2 /= 2
        }
        vox.index
    }

    /// Get the index of the deepest voxel encompassing a specific [VoxelPoint].
    ///
    /// # Panics
    /// * `p` ∉ 0..`self.grid_size()`
    pub fn voxel_at_unchecked(&self, p: &VoxelPoint) -> ProxyIndex {
        self.internal_voxel_at(p, self.grid_size())
    }

    /// Get the index of the deepest voxel encompassing a specific [VoxelPoint].
    ///
    /// # Errors
    /// * `p` ∉ 0..`self.grid_size()`
    pub fn voxel_at<'data>(&self, p: &'data VoxelPoint) -> Result<ProxyIndex, Error<'data>> {
        let size = self.grid_size();
        if p.x >= size || p.y >= size || p.z >= size {
            return Err(Error::VoxelOutOfGrid(size, p));
        }
        Ok(self.internal_voxel_at(p, size))
    }

    /// Get the index of the deepest voxel encompassing a specific [NodePoint].
    pub fn node_at(&self, p: &NodePoint) -> ProxyIndex {
        let mut vox = self.proxies[self.root as usize].as_ref().unwrap();
        let ps = 1 << p.0.w; // 2ʷ // grid size at depth of `p`
        let mut s2 = ps >> 1; // ps / 2 // 1/2 the size of the `p`-grid space occupied by vox
        let psp = VoxelPoint::new(p.0.x * ps, p.0.y * ps, p.0.z * ps); // voxelpoint of target in
                                                                       // grid sized to depth of `p`
        while let ProxyData::Branch(children) = &vox.data {
            if s2 == 0 {
                break;
            };
            let oct = Octant::new(psp.x > s2, psp.y > s2, psp.z > s2);
            vox = self.proxies[children[oct.0 as usize] as usize]
                .as_ref()
                .unwrap();
            s2 >>= 1; // s2 /= 2
        }
        vox.index
    }

    /// Clean proxy & leaf data by removing empty entries and updating stored indices.
    pub fn defragment(&mut self) {
        todo!()
    }

    /// Iterate through all leaf data, from oldest to newest.
    pub fn leaf_unordered(&self) -> impl Iterator<Item = &T> {
        self.leaf_data.iter().filter_map(Option::as_ref)
    }

    /// Iterate through all leaf data, from deepest to shallowest & nearest to farthest (by [Octant] ordering).
    pub fn leaf_dfs(&self) -> LeafIter<T> {
        LeafIter {
            tree: self,
            node_stack: Vec::default(),
            curr_node: Some((
                self.proxies[self.root as usize].as_ref().unwrap(),
                Octant(0),
                NodePoint::new(0, 0, 0, 0),
            )),
        }
    }
}

pub struct LeafIter<'tree, T> {
    tree: &'tree Octree<T>,
    node_stack: Vec<(&'tree Proxy, Octant, NodePoint)>,
    curr_node: Option<(&'tree Proxy, Octant, NodePoint)>,
}

impl<'tree, T> FusedIterator for LeafIter<'tree, T> {}

impl<'tree, T> Iterator for LeafIter<'tree, T> {
    type Item = (&'tree T, NodePoint);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((prox, oct, np)) = self.curr_node {
            match prox.data {
                ProxyData::Void => self.curr_node = self.node_stack.pop(), // not a leaf & not a
                // branch; skip
                ProxyData::Leaf(leaf_idx) => {
                    // move the cursor back to this node's parent (for the next iteration),
                    // then output this node's data
                    self.curr_node = self.node_stack.pop();
                    return Some((self.tree.leaf_data[leaf_idx as usize].as_ref().unwrap(), np));
                }
                ProxyData::Branch(children) => {
                    // move the cursor to the next child node (ordered by `oct`)
                    self.curr_node = Some((
                        self.tree.proxies[children[oct.0 as usize] as usize]
                            .as_ref()
                            .unwrap(),
                        Octant(0),
                        np + oct,
                    ));
                    if oct.0 < 8 {
                        // if we haven't checked all children of this node,
                        // put it on the top of the node stack
                        self.node_stack.push((prox, Octant(oct.0 + 1), np));
                    }
                }
            }
        }
        None
    }
}
