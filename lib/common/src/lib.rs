use std::ops::Shl;

use num_traits::{AsPrimitive, PrimInt};

pub mod macros;

/// Trait for types which can act as indices within an array (or an array-like structure).
pub trait ArrayIndex:
    PrimInt
    + AsPrimitive<usize>
    + AsPrimitive<u8>
    + Shl<Self, Output = Self>
    + std::fmt::Debug
    + 'static
{
}
impl<P> ArrayIndex for P where
    P: PrimInt
        + AsPrimitive<usize>
        + AsPrimitive<u8>
        + Shl<Self, Output = Self>
        + std::fmt::Debug
        + 'static
{
}
