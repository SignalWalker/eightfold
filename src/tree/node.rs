use std::marker::PhantomData;

use eightfold_common::ArrayIndex;

use crate::{Octree, OctreeSlice, Proxy, ProxyData};

#[derive(Debug, Clone, Copy)]
pub enum NodeData<'tree, T, Idx: ArrayIndex> {
    Void,
    Branch(&'tree [Idx; 8]),
    Leaf(&'tree T),
}

impl<'tree, T, Idx: ArrayIndex> NodeData<'tree, T, Idx> {
    fn from_tree_proxy(tree: &'tree Octree<T, Idx>, prox: Proxy<Idx>) -> Self {
        match prox.data {
            crate::ProxyData::Void => NodeData::Void,
            crate::ProxyData::Leaf(idx) => NodeData::Leaf(&tree.leaf_data[idx.as_()]),
            crate::ProxyData::Branch(idx) => NodeData::Branch(&tree.branch_data[idx.as_()]),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Node<'tree, T, Idx: ArrayIndex, Data: 'tree = NodeData<'tree, T, Idx>> {
    tree: &'tree Octree<T, Idx>,
    proxy: Proxy<Idx>,
    index: Idx,
    data: Data,
}

pub type Void<'tree, T, Idx> = Node<'tree, T, Idx, ()>;
pub type Leaf<'tree, T, Idx> = Node<'tree, T, Idx, &'tree T>;
pub type Branch<'tree, T, Idx> = Node<'tree, T, Idx, &'tree [Idx; 8]>;

#[derive(Debug)]
pub enum NodeDataMut<'tree, T, Idx: ArrayIndex> {
    Void,
    Branch(&'tree mut [Idx; 8]),
    Leaf(&'tree mut T),
}

impl<'tree, T, Idx: ArrayIndex> NodeDataMut<'tree, T, Idx> {
    fn from_tree_proxy(tree: &'tree mut Octree<T, Idx>, prox: Proxy<Idx>) -> Self {
        match prox.data {
            crate::ProxyData::Void => Self::Void,
            crate::ProxyData::Leaf(idx) => Self::Leaf(&mut tree.leaf_data[idx.as_()]),
            crate::ProxyData::Branch(idx) => Self::Branch(&mut tree.branch_data[idx.as_()]),
        }
    }
}

impl<'tree, T, Idx: ArrayIndex, Data: 'tree> Node<'tree, T, Idx, Data> {
    pub fn parent(&self) -> Option<Branch<'tree, T, Idx>> {
        todo!()
    }
}

impl<'tree, T, Idx: ArrayIndex> Node<'tree, T, Idx, NodeData<'tree, T, Idx>> {}

#[derive(Debug)]
pub struct NodeMut<'tree, T, Idx: ArrayIndex, Data: 'tree = NodeDataMut<'tree, T, Idx>> {
    tree: &'tree mut Octree<T, Idx>,
    proxy: Proxy<Idx>,
    index: Idx,
    _data: PhantomData<Data>,
}

impl<'tree, T, Idx: ArrayIndex, Data: 'tree> NodeMut<'tree, T, Idx, Data> {
    pub fn parent(self) -> Option<BranchMut<'tree, T, Idx>> {
        todo!()
    }

    pub fn data(&'tree self) -> NodeData<'tree, T, Idx> {
        NodeData::from_tree_proxy(self.tree, self.proxy)
    }

    pub fn data_mut(&'tree mut self) -> NodeDataMut<'tree, T, Idx> {
        NodeDataMut::from_tree_proxy(self.tree, self.proxy)
    }
}

pub type VoidMut<'tree, T, Idx> = NodeMut<'tree, T, Idx, ()>;
pub type LeafMut<'tree, T, Idx> = NodeMut<'tree, T, Idx, &'tree mut T>;
pub type BranchMut<'tree, T, Idx> = NodeMut<'tree, T, Idx, &'tree mut [Idx; 8]>;

impl<'tree, T, Idx: ArrayIndex> LeafMut<'tree, T, Idx> {
    pub fn leaf_data(&'tree self) -> &'tree T {
        &self.tree.leaf_data[self.proxy.leaf().unwrap().as_()]
    }

    pub fn leaf_data_mut(&'tree mut self) -> &'tree mut T {
        &mut self.tree.leaf_data[self.proxy.leaf().unwrap().as_()]
    }
}

impl<'tree, T, Idx: ArrayIndex> BranchMut<'tree, T, Idx> {
    pub fn child_indices(&'tree self) -> &'tree [Idx; 8] {
        &self.tree.branch_data[self.proxy.branch().unwrap().as_()]
    }

    pub fn child_indices_mut(&'tree mut self) -> &'tree mut [Idx; 8] {
        &mut self.tree.branch_data[self.proxy.branch().unwrap().as_()]
    }
}

impl<T, Idx: ArrayIndex> Octree<T, Idx> {
    pub fn node<'tree>(&'tree self, index: Idx) -> Option<Node<'tree, T, Idx>> {
        self.proxies.get(index.as_()).copied().map(|proxy| Node {
            tree: self,
            proxy,
            index,
            data: NodeData::from_tree_proxy(self, proxy),
        })
    }

    pub fn node_mut<'tree>(&'tree mut self, index: Idx) -> Option<NodeMut<'tree, T, Idx>> {
        self.proxies.get(index.as_()).copied().map(|proxy| NodeMut {
            tree: self,
            proxy,
            index,
            _data: Default::default(),
        })
    }

    pub fn void<'tree>(&'tree self, index: Idx) -> Option<Void<'tree, T, Idx>> {
        self.proxies
            .get(index.as_())
            .copied()
            .and_then(|proxy| match proxy.data {
                ProxyData::Void => Some(Void {
                    tree: self,
                    proxy,
                    index,
                    data: (),
                }),
                _ => None,
            })
    }

    pub fn leaf<'tree>(&'tree self, index: Idx) -> Option<Leaf<'tree, T, Idx>> {
        self.proxies.get(index.as_()).copied().and_then(|proxy| {
            proxy.leaf().map(|idx| Leaf {
                tree: self,
                proxy,
                index,
                data: &self.leaf_data[idx.as_()],
            })
        })
    }

    pub fn leaf_mut<'tree>(&'tree mut self, index: Idx) -> Option<LeafMut<'tree, T, Idx>> {
        self.proxies.get(index.as_()).copied().and_then(|proxy| {
            proxy.leaf().map(|_| LeafMut {
                tree: self,
                proxy,
                index,
                _data: Default::default(),
            })
        })
    }

    pub fn branch<'tree>(&'tree self, index: Idx) -> Option<Branch<'tree, T, Idx>> {
        self.proxies.get(index.as_()).copied().and_then(|proxy| {
            proxy.branch().map(|idx| Branch {
                tree: self,
                proxy,
                index,
                data: &self.branch_data[idx.as_()],
            })
        })
    }

    pub fn branch_mut<'tree>(&'tree mut self, index: Idx) -> Option<BranchMut<'tree, T, Idx>> {
        self.proxies.get(index.as_()).copied().and_then(|proxy| {
            proxy.branch().map(|_| BranchMut {
                tree: self,
                proxy,
                index,
                _data: Default::default(),
            })
        })
    }
}
