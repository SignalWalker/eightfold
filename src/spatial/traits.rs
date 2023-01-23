use eightfold_common::item_with;

/// Trait for floating-point types, so that other things can be generic over {f32, f64} without
/// having to use [weird macros](crate::item_with).
///
/// # Safety
///
/// * This is only intended to be implemented on floating-point types.
#[allow(unsafe_code)]
pub unsafe trait Float:
    num_traits::Float
    + nalgebra::Scalar
    + nalgebra::SimdPartialOrd
    + std::ops::AddAssign
    + std::ops::SubAssign
    + Copy
    + Send
    + Sync
{
    const ZERO: Self;
    const ONE: Self;
    const TWO: Self;

    const MIN: Self;
    const MAX: Self;
}

// this macro lets us impl Float for both f32 and f64 without having to copy/paste,
// but it feels very goofy
item_with! {Real: f32, f64 => unsafe impl Float for Real {
    #![allow(unsafe_code)]
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
    const TWO: Self = 2.0;

    const MIN: Self = Self::MIN;
    const MAX: Self = Self::MAX;
}}
