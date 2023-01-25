use std::iter::FusedIterator;

use eightfold_common::ArrayIndex;
use num_traits::AsPrimitive;

use crate::{NodePoint, Octant, Octree, Proxy, ProxyData};

pub struct NodeIter<'tree, T, Idx: ArrayIndex> {
    pub(crate) tree: &'tree Octree<T, Idx>,
    pub(crate) node_stack: Vec<(&'tree Proxy<Idx>, Octant, NodePoint<Idx>)>,
    pub(crate) curr_node: Option<(&'tree Proxy<Idx>, Octant, NodePoint<Idx>)>,
}

impl<'tree, T, Idx: ArrayIndex> FusedIterator for NodeIter<'tree, T, Idx> where u8: AsPrimitive<Idx> {}

impl<'tree, T, Idx: ArrayIndex> Iterator for NodeIter<'tree, T, Idx>
where
    u8: AsPrimitive<Idx>,
{
    type Item = (Proxy<Idx>, NodePoint<Idx>);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((prox, oct, np)) = self.curr_node {
            match prox.data {
                ProxyData::Void | ProxyData::Leaf(_) => {
                    self.curr_node = self.node_stack.pop();
                    return Some((*prox, np));
                }
                ProxyData::Branch(ch_idx) => {
                    let children: &[Idx; 8] = &self.tree.branch_data[ch_idx.as_()];
                    // move the cursor to the next child node (ordered by `oct`)
                    self.curr_node = Some((
                        &self.tree.proxies[children[usize::from(oct)].as_()],
                        Octant(0),
                        np + oct,
                    ));
                    if oct < Octant::MAX {
                        // if we haven't checked all children of this node,
                        // put it on the top of the node stack
                        self.node_stack.push((prox, Octant(oct.0 + 1), np));
                    }
                    return Some((*prox, np));
                }
            }
        }
        None
    }
}

/// A depth-first iterator over leafs in an [Octree].
pub struct LeafIter<'tree, T, Idx: ArrayIndex> {
    pub(crate) tree: &'tree Octree<T, Idx>,
    pub(crate) node_stack: Vec<(&'tree Proxy<Idx>, Octant, NodePoint<Idx>)>,
    pub(crate) curr_node: Option<(&'tree Proxy<Idx>, Octant, NodePoint<Idx>)>,
}

impl<'tree, T, Idx: ArrayIndex> FusedIterator for LeafIter<'tree, T, Idx> where u8: AsPrimitive<Idx> {}

impl<'tree, T, Idx: ArrayIndex> Iterator for LeafIter<'tree, T, Idx>
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
                    if oct < Octant::MAX {
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
