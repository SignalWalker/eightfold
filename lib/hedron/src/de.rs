//! Deserialization of geometry from various storage formats.

#[cfg(feature = "de_gltf")]
pub mod gltf;
#[cfg(feature = "de_obj")]
pub mod obj;
