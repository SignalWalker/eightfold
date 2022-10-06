use num_traits::AsPrimitive;
use parking_lot::RwLock;

use crate::{Error, LeafIter, NodePoint, Octant, Octree, ProxyData, TreeIndex};

/// A slice representing a subset of an [Octree].
#[derive(Debug, Clone, Copy)]
pub struct TreeSlice<'tree, T, Idx: TreeIndex> {
    tree: &'tree Octree<T, Idx>,
    root: Idx,
    depth: Idx,
    height: Idx,
}

impl<T, Idx: TreeIndex> Octree<T, Idx> {
    pub fn slice(&self, index: Idx) -> Result<TreeSlice<T, Idx>, Error<Idx>> {
        if !self.proxies.is_init(index.as_()) {
            Err(Error::InvalidIndex(index))
        } else {
            Ok(TreeSlice {
                tree: self,
                root: index,
                depth: self.depth_of_unchecked(index),
                height: if index == self.root {
                    self.height()
                } else {
                    self.height_from(index)
                },
            })
        }
    }

    pub fn as_slice(&self) -> TreeSlice<T, Idx> {
        TreeSlice {
            tree: self,
            root: self.root,
            depth: Idx::zero(),
            height: self.height(),
        }
    }
}

impl<'tree, T, Idx: TreeIndex> TreeSlice<'tree, T, Idx> {
    pub fn base(&self) -> &'tree Octree<T, Idx> {
        self.tree
    }
}

/// Trait for [Octree] references.
pub trait OctreeSlice<T, Idx: TreeIndex> {
    /// Index of the root node.
    fn root_idx(&self) -> Idx;
    /// The height of a subtree, originating at a specific node.
    fn height_from(&self, index: Idx) -> Idx;
    /// The height of the tree calculated from the root.
    fn height(&self) -> Idx {
        self.height_from(self.root_idx())
    }
    /// The dimensions of the cubical voxel grid represented by this tree, as determined by the
    /// tree's height.
    fn grid_size(&self) -> Idx {
        Idx::one() << self.height()
    }
    /// Depth-first iterator through all leafs, from deepest to shallowest & nearest to farthest
    /// (by [Octant] ordering).
    fn leaf_dfi(&self) -> LeafIter<T, Idx>;
}

impl<T, Idx: TreeIndex> OctreeSlice<T, Idx> for Octree<T, Idx> {
    fn root_idx(&self) -> Idx {
        self.root
    }

    fn height_from(&self, index: Idx) -> Idx {
        let mut max_depth = Idx::zero();
        let mut node_stack = vec![(self.proxies[index.as_()], Idx::zero())];
        while let Some((p, depth)) = node_stack.pop() {
            if depth > max_depth {
                max_depth = depth;
            }
            if let ProxyData::Branch(b_idx) = p.data {
                node_stack.extend(
                    self.branch_data[b_idx.as_()]
                        .into_iter()
                        .map(|c| (self.proxies[c.as_()], depth + Idx::one())),
                );
            }
        }
        max_depth
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
    fn height(&self) -> Idx {
        match *self.height_cache.read() {
            Some(h) => h,
            None => {
                let mut height_cache = self.height_cache.write();
                if let Some(h) = *height_cache {
                    return h;
                } // another thread calculated this
                  // before this one could get a lock
                #[cfg(feature = "tracing")]
                tracing::trace!("Recalculating tree height...");
                let height = self.height_from(self.root);
                height_cache.replace(height);
                height
            }
        }
    }

    fn leaf_dfi(&self) -> LeafIter<T, Idx> {
        LeafIter {
            tree: self,
            node_stack: Vec::default(),
            curr_node: Some((
                &self.proxies[self.root.as_()],
                Octant(0),
                NodePoint::new(Idx::zero(), Idx::zero(), Idx::zero(), Idx::zero()),
            )),
        }
    }
}

impl<'tree, T, Idx: TreeIndex> OctreeSlice<T, Idx> for TreeSlice<'tree, T, Idx>
where
    u8: AsPrimitive<Idx>,
{
    fn root_idx(&self) -> Idx {
        self.root
    }
    fn height_from(&self, index: Idx) -> Idx {
        self.tree.height_from(index)
    }
    fn height(&self) -> Idx {
        self.height
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
