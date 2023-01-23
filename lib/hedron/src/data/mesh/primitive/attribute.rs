use eightfold_common::impl_mul_div;
use nalgebra::{Point2, Point3, Point4, Vector3, Vector4};

use std::ops::{Div, DivAssign, Mul, MulAssign};

pub mod storage;

pub mod delta;

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttributeUsage {
    Position,
    Normal,
    Tangent,
    Texcoord(u32),
    Color(u32),
    Joints(u32),
    Weights(u32),
}

/// The inner components of an [AttributeType]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttributeComponent {
    U8,
    U16,
    U32,
    I8,
    I16,
    F32,
}

impl AttributeComponent {
    pub const fn alignment(self) -> usize {
        use std::mem::align_of;
        match self {
            AttributeComponent::U8 => align_of::<u8>(),
            AttributeComponent::U16 => align_of::<u16>(),
            AttributeComponent::U32 => align_of::<u32>(),
            AttributeComponent::I8 => align_of::<i8>(),
            AttributeComponent::I16 => align_of::<i16>(),
            AttributeComponent::F32 => align_of::<f32>(),
        }
    }

    pub const fn size(self) -> usize {
        use std::mem::size_of;
        match self {
            AttributeComponent::U8 => size_of::<u8>(),
            AttributeComponent::U16 => size_of::<u16>(),
            AttributeComponent::U32 => size_of::<u32>(),
            AttributeComponent::I8 => size_of::<i8>(),
            AttributeComponent::I16 => size_of::<i16>(),
            AttributeComponent::F32 => size_of::<f32>(),
        }
    }
}

/// The type of value stored in an [AttrStore].
///
/// Values taken from the [glTF specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#accessor-data-types).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttributeType {
    Scalar,
    Vec2,
    Vec3,
    Vec4,
    Mat2,
    Mat3,
    Mat4,
}

impl AttributeType {
    #[inline]
    pub const fn alignment(self, comp: AttributeComponent) -> usize {
        // from [rustref](https://doc.rust-lang.org/nightly/reference/type-layout.html#array-layout), we know that arrays have the same alignment as their component type, so:
        comp.alignment()
    }

    pub const fn size_elements(self) -> usize {
        match self {
            AttributeType::Scalar => 1,
            AttributeType::Vec2 => 2,
            AttributeType::Vec3 => 3,
            AttributeType::Vec4 => 4,
            AttributeType::Mat2 => 4,
            AttributeType::Mat3 => 9,
            AttributeType::Mat4 => 16,
        }
    }

    pub const fn size_bytes(self, comp: AttributeComponent) -> usize {
        comp.size() * self.size_elements()
    }
}

pub trait AttributeComponentType {
    const COMPONENT: AttributeComponent;
}

/// Trait for types which can be used as primitive attribute data.
///
/// # Safety
///
/// Implementing types *must* have the size and alignment described by their `TYPE` and `COMPONENT` constants. Therefore, all implementing types must have the same size and alignment as eachother.
#[allow(unsafe_code)]
pub unsafe trait Attribute: Sized {
    const TYPE: AttributeType;
    const COMPONENT: AttributeComponent;
}

mod _impl_attr {
    use crate::primitive::attribute::{
        Attribute, AttributeComponent, AttributeComponentType, AttributeType,
    };

    macro_rules! impl_attr_comp_type {
        ($Target:ident: $comp:expr) => {
            impl AttributeComponentType for $Target {
                const COMPONENT: AttributeComponent = $comp;
            }
        };
    }

    impl_attr_comp_type!(u8: AttributeComponent::U8);
    impl_attr_comp_type!(u16: AttributeComponent::U16);
    impl_attr_comp_type!(u32: AttributeComponent::U32);
    impl_attr_comp_type!(i8: AttributeComponent::I8);
    impl_attr_comp_type!(i16: AttributeComponent::I16);
    impl_attr_comp_type!(f32: AttributeComponent::F32);
    // macro_rules! impl_attr_o {
    //     ($t:ident<$C:ident> => $Target:ty$({$($param:tt),+})?) => {
    //         unsafe impl<$C: $crate::AttributeComponentType $($(+ $param)+)?> $crate::Attribute for $Target {
    //             const TYPE: $crate::AttributeType = $crate::AttributeType::$t;
    //             const COMPONENT: $crate::AttributeComponent = $C::COMPONENT;
    //         }
    //     };
    //     ($t:ident<$C:ident> => $Target:ty$({$($fparam:tt),+})?, $($Targets:ty$({$($fparams:tt),+})?),+) => {
    //         impl_attr!($t<$C> => $Target$({$($fparam),+})?);
    //         $(impl_attr!($t<$C> => $Targets$({$($fparams),+})?);)+
    //     }
    // }

    macro_rules! impl_attr {
        ($t:ident<$($C:ty, $c:ident);+: $CAlias:ident> => $Target:ty) => {
            $( // for every ($C, $c)
               const _: () = { // anonymous module
                type $CAlias = $C;
                // "size of type $C == size of component $c"
                static_assertions::const_assert_eq!(std::mem::size_of::<$CAlias>(), AttributeComponent::$c.size());
                // "size of type $Target == size of attribute $t with component $c"
                static_assertions::const_assert_eq!(std::mem::size_of::<$Target>(), AttributeType::$t.size_bytes(AttributeComponent::$c));
                // "alignment of type $Target == alignment of attribute $t with component $c"
                static_assertions::const_assert_eq!(std::mem::align_of::<$Target>(), AttributeType::$t.alignment(AttributeComponent::$c));
                #[allow(unsafe_code)]
                unsafe impl Attribute for $Target {
                    const TYPE: AttributeType = AttributeType::$t;
                    const COMPONENT: AttributeComponent = AttributeComponent::$c;
                }
               };
            )+
        };
        ($t:ident<$($C:ident, $c:ident);+> => $Target:ty) => {
            impl_attr!($t<$($C, $c);+: __MacroImplC> => $Target);
        };
        ($t:ident<$CAlias:ident> => $Target:ty) => {
            impl_attr!($t<
                u8, U8;
                u16, U16;
                u32, U32;
                i8, I8;
                i16, I16;
                f32, F32: $CAlias> => $Target);
        };
    }

    impl_attr!(Scalar<C> => C);
    impl_attr!(Scalar<C> => [C; 1]);

    impl_attr!(Vec2<C> => nalgebra::Vector2<C>);
    impl_attr!(Vec2<C> => nalgebra::Point2<C>);
    impl_attr!(Vec2<C> => [C; 2]);
    impl_attr!(Vec2<C> => (C, C));

    impl_attr!(Vec3<C> => nalgebra::Vector3<C>);
    impl_attr!(Vec3<C> => nalgebra::Point3<C>);
    impl_attr!(Vec3<C> => [C; 3]);
    impl_attr!(Vec3<C> => (C, C, C));

    impl_attr!(Vec4<C> => nalgebra::Vector4<C>);
    impl_attr!(Vec4<C> => nalgebra::Point4<C>);
    impl_attr!(Vec4<C> => [C; 4]);
    impl_attr!(Vec4<C> => (C, C, C, C));
    impl_attr!(Vec4<f32, F32> => super::Tangent);

    impl_attr!(Mat2<C> => nalgebra::Matrix2<C>);
    impl_attr!(Mat3<C> => nalgebra::Matrix3<C>);
    impl_attr!(Mat3<C> => [C; 9]);
    impl_attr!(Mat4<C> => nalgebra::Matrix4<C>);
    impl_attr!(Mat4<C> => [C; 16]);
}

pub type Position = Point3<f32>;
pub type Normal = Vector3<f32>;
// tangent defined below

pub type Texcoord<C> = Point2<C>; // gltf: u8 | u16 | f32
pub type Rgb<C> = Point3<C>; // gltf: u8 | u16 | f32
pub type Rgba<C> = Point4<C>; // gltf: u8 | u16 | f32
pub type Joints<C> = [C; 4]; // gltf: u8 | u16
pub type Weights<C> = [C; 4]; // gltf: u8 | u16 | f32

/// The handedness of a tangent attribute
#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Handedness {
    Negative = -1,
    Positive = 1,
}

/// Tangent vector of a vertex
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tangent(Vector3<f32>, Handedness);

impl_mul_div!(self: Tangent, rhs: f32;
        (Tangent(self.0 * rhs.to_owned(), self.1); self.0 *= rhs.to_owned());
        (Tangent(self.0 / rhs.to_owned(), self.1); self.0 /= rhs.to_owned()));

impl From<Tangent> for Vector3<f32> {
    fn from(value: Tangent) -> Self {
        value.0
    }
}

impl Handedness {
    /// Convert self to a value usable within a glTF asset
    #[inline]
    pub fn to_gltf(self) -> f32 {
        (self as i8) as f32
    }
}

impl Tangent {
    /// Convert self to value usable within a glTF asset
    #[inline]
    pub fn to_gltf(&self) -> Vector4<f32> {
        nalgebra::vector![self.0.x, self.0.y, self.0.z, self.1.to_gltf()]
    }
}
