//! Deserialization of geometry from various storage formats.

#[cfg(feature = "gltf")]
pub mod gltf;
#[cfg(feature = "obj")]
pub mod obj;
