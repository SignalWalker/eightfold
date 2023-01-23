//! Type definitions for weight morph delta attribute data

use std::ops::{Add, AddAssign, Sub, SubAssign};

use eightfold_common::impl_add_sub;
use nalgebra::{Vector2, Vector3, Vector4};

pub type Position = Vector3<f32>;
pub type Normal = Vector3<f32>;
pub type Tangent = Vector3<f32>;

pub type Texcoord<C> = Vector2<C>; // gltf: u8 | u16 | i8 | i16 | f32
pub type Rgb<C> = Vector3<C>; // gltf: u8 | u16 | i8 | i16 | f32
pub type Rgba<C> = Vector4<C>; // gltf: u8 | u16 | i8 | i16 | f32

impl_add_sub!(self: super::Tangent, rhs: Tangent;
            (super::Tangent(self.0 + rhs, self.1); self.0 += rhs);
            (super::Tangent(self.0 - rhs, self.1); self.0 -= rhs));
