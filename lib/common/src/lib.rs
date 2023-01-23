#![doc = include_str!("../README.md")]

use std::ops::{AddAssign, Shl, Shr, SubAssign};

use num_traits::{AsPrimitive, PrimInt};

pub mod macros;

/// Trait for types which can act as indices within an array (or an array-like structure).
///
/// # Safety
///
/// This is only intended to be implemented on unsigned integer types.
#[allow(unsafe_code)]
pub unsafe trait ArrayIndex:
    PrimInt
    + From<u8>
    + AsPrimitive<u8>
    + AsPrimitive<usize>
    + Shl<Self, Output = Self>
    + Shr<Self, Output = Self>
    + AddAssign<Self>
    + SubAssign<Self>
    + std::fmt::Debug
    + 'static
{
    const ZERO: Self;
    const ONE: Self;
}

#[cfg(target_pointer_width = "64")]
item_with! {Idx: u8, u16, u32, u64, usize => unsafe impl ArrayIndex for Idx {
    #![allow(unsafe_code)]
    const ZERO: Self = 0;
    const ONE: Self = 1;
}}

#[cfg(target_pointer_width = "32")]
item_with! {Idx: u8, u16, u32, usize => unsafe impl ArrayIndex for Idx {
    #![allow(unsafe_code)]
    const ZERO: Self = 0;
    const ONE: Self = 1;
}}

#[cfg(target_pointer_width = "16")]
item_with! {Idx: u8, u16, usize => unsafe impl ArrayIndex for Idx {
    #![allow(unsafe_code)]
    const ZERO: Self = 0;
    const ONE: Self = 1;
}}
