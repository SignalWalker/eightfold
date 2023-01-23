use eightfold_common::ArrayIndex;
use num_traits::AsPrimitive;
use stablevec::StableVec;

use crate::{Error, LeafIter, NodePoint, Octant, Octree, Proxy, ProxyData};

/// A slice representing a subset of an [Octree].
#[derive(Debug, Clone, Copy)]
pub struct TreeSlice<'tree, T, Idx: ArrayIndex> {
    tree: &'tree Octree<T, Idx>,
    root: Idx,
}

impl<T, Idx: ArrayIndex> Octree<T, Idx> {
    pub fn slice(&self, index: Idx) -> Result<TreeSlice<T, Idx>, Error<Idx>> {
        if !self.proxies.is_init(index.as_()) {
            Err(Error::InvalidIndex(index))
        } else {
            Ok(TreeSlice {
                tree: self,
                root: index,
            })
        }
    }

    pub fn as_slice(&self) -> TreeSlice<T, Idx> {
        TreeSlice {
            tree: self,
            root: self.root,
        }
    }
}

impl<'tree, T, Idx: ArrayIndex> TreeSlice<'tree, T, Idx> {
    pub fn base(&self) -> &'tree Octree<T, Idx> {
        self.tree
    }
}

/// Trait for [Octree] references.
pub trait OctreeSlice<T, Idx: ArrayIndex> {
    /// Index of the root node.
    fn root_idx(&self) -> Idx;
    fn proxies(&self) -> &StableVec<Proxy<Idx>>;
    fn branch_data(&self) -> &StableVec<[Idx; 8]>;
    fn leaf_data(&self) -> &StableVec<T>;
    /// Get the [Proxy] representing the node at a given index.
    #[inline]
    fn get(&self, i: Idx) -> Proxy<Idx> {
        self.proxies()[self.root_idx().as_()]
    }
    /// Get the [Proxy] representing the root node of `self`.
    #[inline]
    fn root_proxy(&self) -> Proxy<Idx> {
        self.get(self.root_idx())
    }
    /// The height of a subtree, originating at a specific node.
    fn height_from(&self, index: Idx) -> Idx;
    /// The height of the tree calculated from the root.
    #[inline]
    fn height(&self) -> Idx {
        self.height_from(self.root_idx())
    }
    /// The dimensions of the cubical voxel grid represented by this tree, as determined by the
    /// tree's height.
    #[inline]
    fn grid_size(&self) -> Idx {
        Idx::ONE << self.height()
    }
    /// Depth-first iterator through all leafs, from deepest to shallowest & nearest to farthest
    /// (by [Octant] ordering).
    fn leaf_dfi(&self) -> LeafIter<T, Idx>;
}

impl<T, Idx: ArrayIndex> OctreeSlice<T, Idx> for Octree<T, Idx> {
    #[inline]
    fn root_idx(&self) -> Idx {
        self.root
    }

    #[inline]
    fn get(&self, i: Idx) -> Proxy<Idx> {
        self.proxies[i.as_()]
    }

    #[inline]
    fn proxies(&self) -> &StableVec<Proxy<Idx>> {
        &self.proxies
    }

    #[inline]
    fn branch_data(&self) -> &StableVec<[Idx; 8]> {
        &self.branch_data
    }

    #[inline]
    fn leaf_data(&self) -> &StableVec<T> {
        &self.leaf_data
    }

    fn height_from(&self, index: Idx) -> Idx {
        let mut max_depth = Idx::ZERO;
        let mut node_stack = vec![(self.proxies[index.as_()], Idx::ZERO)];
        while let Some((p, depth)) = node_stack.pop() {
            if depth > max_depth {
                max_depth = depth;
            }
            if let ProxyData::Branch(b_idx) = p.data {
                node_stack.extend(
                    self.branch_data[b_idx.as_()]
                        .into_iter()
                        .map(|c| (self.proxies[c.as_()], depth + Idx::ONE)),
                );
            }
        }
        max_depth
    }

    fn leaf_dfi(&self) -> LeafIter<T, Idx> {
        LeafIter {
            tree: self,
            node_stack: Vec::default(),
            curr_node: Some((
                &self.proxies[self.root.as_()],
                Octant(0),
                NodePoint::new(Idx::ZERO, Idx::ZERO, Idx::ZERO, Idx::ZERO),
            )),
        }
    }
}

impl<'tree, T, Idx: ArrayIndex> OctreeSlice<T, Idx> for TreeSlice<'tree, T, Idx>
where
    u8: AsPrimitive<Idx>,
{
    #[inline]
    fn root_idx(&self) -> Idx {
        self.root
    }

    #[inline]
    fn proxies(&self) -> &StableVec<Proxy<Idx>> {
        &self.tree.proxies
    }

    #[inline]
    fn branch_data(&self) -> &StableVec<[Idx; 8]> {
        &self.tree.branch_data
    }

    #[inline]
    fn leaf_data(&self) -> &StableVec<T> {
        &self.tree.leaf_data
    }

    fn height_from(&self, index: Idx) -> Idx {
        self.tree.height_from(index)
    }
    fn leaf_dfi(&self) -> LeafIter<T, Idx> {
        LeafIter {
            tree: self.tree,
            node_stack: Vec::default(),
            curr_node: Some((
                &self.tree.proxies[self.root.as_()],
                Octant(0),
                self.tree.node_point_of_unchecked(self.root),
            )),
        }
    }
}
