use eightfold_common::ArrayIndex;

pub trait TreeIndex {}

/// Indices of data within an [Octree](crate::Octree).
#[derive(Debug, Clone, Copy)]
pub struct Proxy<Idx: ArrayIndex> {
    pub(crate) parent: Idx,
    pub(crate) data: ProxyData<Idx>,
}

/// The type of data pointed to by a [Proxy] and the index of that data.
#[derive(Debug, Clone, Copy)]
pub enum ProxyData<Idx: ArrayIndex> {
    /// Empty
    Void,
    /// Internal pointer to leaf data
    Leaf(Idx),
    /// Internal pointer to branch data
    Branch(Idx),
}
