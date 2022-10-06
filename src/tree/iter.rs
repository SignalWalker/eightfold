use std::iter::FusedIterator;

use num_traits::AsPrimitive;

use crate::{NodePoint, Octant, Octree, Proxy, ProxyData, TreeIndex};

/// A depth-first iterator over leafs in an [Octree].
pub struct LeafIter<'tree, T, Idx: TreeIndex> {
    pub(crate) tree: &'tree Octree<T, Idx>,
    pub(crate) node_stack: Vec<(&'tree Proxy<Idx>, Octant, NodePoint<Idx>)>,
    pub(crate) curr_node: Option<(&'tree Proxy<Idx>, Octant, NodePoint<Idx>)>,
}

impl<'tree, T, Idx: TreeIndex> FusedIterator for LeafIter<'tree, T, Idx> where u8: AsPrimitive<Idx> {}

impl<'tree, T, Idx: TreeIndex> Iterator for LeafIter<'tree, T, Idx>
where
    u8: AsPrimitive<Idx>,
{
    type Item = (&'tree T, NodePoint<Idx>);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((prox, oct, np)) = self.curr_node {
            match prox.data {
                ProxyData::Void => self.curr_node = self.node_stack.pop(), // not a leaf & not a
                // branch; skip
                ProxyData::Leaf(leaf_idx) => {
                    // move the cursor back to this node's parent (for the next iteration),
                    // then output this node's data
                    self.curr_node = self.node_stack.pop();
                    return Some((&self.tree.leaf_data[leaf_idx.as_()], np));
                }
                ProxyData::Branch(ch_idx) => {
                    let children: &[Idx; 8] = &self.tree.branch_data[ch_idx.as_()];
                    // move the cursor to the next child node (ordered by `oct`)
                    self.curr_node = Some((
                        &self.tree.proxies[children[oct.0 as usize].as_()],
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
