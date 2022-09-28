use crate::ProxyIndex;

#[derive(Debug, thiserror::Error)]
pub enum Error<'data> {
    #[error("Attempted to access the parent of the root node")]
    NoParent,
    #[error("Child index out of range: 0..8 ∌ {0}")]
    ChildOutOfRange(u8),
    #[error("Attempted to access child of terminal node: {0}")]
    NoChildren(ProxyIndex),
    #[error("Attempted to access grid point outside of tree: (0..{0}) ∌ {1:?}")]
    VoxelOutOfGrid(u32, &'data crate::VoxelPoint),
    #[error("Attempted to add a child to a position already occupied: {0:?}")]
    ChildCollision(ProxyIndex),
    #[error("Attempted to make a leaf into a branch")]
    BranchCollision,
}
