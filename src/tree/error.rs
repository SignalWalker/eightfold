use eightfold_common::ArrayIndex;

/// Errors related to [Octrees](crate::Octree).
#[derive(Debug, thiserror::Error)]
pub enum Error<'data, Idx: ArrayIndex> {
    #[error("Attempted to access the parent of the root node")]
    NoParent,
    #[error("Child index out of range: 0..8 ∌ {0}")]
    ChildOutOfRange(u8),
    #[error("Attempted to access node at index {0}, which is unoccupied.")]
    InvalidIndex(Idx),
    #[error("Attempted to perform a branch-specific operation on a non-branch node at index {0}.")]
    NotABranch(Idx),
    #[error("Attempted to perform a leaf-specific operation on a non-leaf node at index {0}.")]
    NotALeaf(Idx),
    #[error("Attempted to perform a void-specific operation on a non-void node at index {0}.")]
    NotAVoid(Idx),
    #[error("Attempted to access child of terminal node: {0}")]
    NoChildren(Idx),
    #[error("No descendant of node {0} is a leaf.")]
    NoLeafs(Idx),
    #[error("Attempted to access grid point outside of tree: (0 -> {0}) ∌ {1:?}")]
    VoxelOutOfGrid(Idx, &'data crate::VoxelPoint<Idx>),
    #[error("Attempted to add a child to a position already occupied: {0:?}")]
    ChildCollision(Idx),
    #[error("Attempted to make a leaf into a branch")]
    BranchCollision,
}
