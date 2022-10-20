#![doc = include_str!("../README.md")]

#[cfg(feature = "deserialize")]
pub mod de;

#[cfg(feature = "serialize")]
pub mod ser;

mod data;
pub use data::*;
