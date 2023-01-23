use std::marker::PhantomData;

use eightfold_common::ArrayIndex;

use crate::{Octree, Proxy};

#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error("not a void node")]
    NotAVoid,
    #[error("not a leaf node")]
    NotALeaf,
    #[error("not a branch node")]
    NotABranch,
}

/// A view into a single leaf within an [Octree].
// pub struct Leaf<'tree, T, Idx: ArrayIndex> {
//     tree: &'tree Octree<T, Idx>,
//     proxy: Proxy<Idx>,
//     index: Idx,
//     data: &'tree T,
// }

/// A view into a single branch within an [Octree].
// pub struct Branch<'tree, T, Idx: ArrayIndex> {
//     tree: &'tree Octree<T, Idx>,
//     proxy: Proxy<Idx>,
//     index: Idx,
//     children: &'tree [Idx; 8],
// }

// pub struct Void<'tree, T, Idx: ArrayIndex> {
//     tree: &'tree Octree<T, Idx>,
//     proxy: Proxy<Idx>,
//     index: Idx,
// }

pub enum NodeData<'tree, T, Idx: ArrayIndex> {
    Void,
    Branch(&'tree [Idx; 8]),
    Leaf(&'tree T),
}

pub struct Node<'tree, T, Idx: ArrayIndex, Data: 'tree = NodeData<'tree, T, Idx>> {
    tree: &'tree Octree<T, Idx>,
    proxy: Proxy<Idx>,
    index: Idx,
    data: Data,
}

pub type Void<'tree, T, Idx: ArrayIndex> = Node<'tree, T, Idx, ()>;
pub type Leaf<'tree, T, Idx: ArrayIndex> = Node<'tree, T, Idx, &'tree T>;
pub type Branch<'tree, T, Idx: ArrayIndex> = Node<'tree, T, Idx, &'tree [Idx; 8]>;

impl<'tree, T, Idx: ArrayIndex> Node<'tree, T, Idx, NodeData<'tree, T, Idx>> {
    pub fn is_leaf(&self) -> bool {}
    pub fn as_leaf() -> Result<Leaf<'tree, T, Idx>, NodeError> {}
}

// impl<T, Idx: ArrayIndex> Octree<T, Idx> {
//     pub fn node<'tree>(&'tree self, i: Idx) -> Node<'tree, T, Idx> {
//
//     }
// }
