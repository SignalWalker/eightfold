#[derive(Debug, thiserror::Error)]
pub enum Error<'data> {
    #[error("Attempted to access world point outside of range: {0:?}")]
    PointOutOfBounds(&'data super::WorldPoint),
}
