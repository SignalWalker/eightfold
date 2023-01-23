use std::{slice, sync::Arc};

use gltf::accessor::{DataType, Dimensions};

use super::BufferCacheData;

#[derive(Debug, thiserror::Error)]
pub enum AccessorError {
    #[error(
        "expected data of component type {expected:?}, received data of component type {actual:?}"
    )]
    MismatchedComponentType {
        expected: DataType,
        actual: DataType,
    },
    #[error("expected data of dimensions {expected:?}, received data of dimensions {actual:?}")]
    MismatchedDimensions {
        expected: Dimensions,
        actual: Dimensions,
    },
    #[error("sparse accessors are unsupported")]
    Sparse,
    #[error("normalized accessors are unsupported")]
    Normalized,
    #[error("accessors with non-zero stride are unsupported")]
    Stride,
}

#[derive(Debug)]
pub struct BufferAccessor<'buf> {
    pub(crate) buffer: Arc<BufferCacheData<'buf>>,
    pub(crate) offset: usize,
    pub(crate) count: usize,
    pub(crate) data_type: DataType,
    pub(crate) dimensions: Dimensions,
}

impl<'buf> BufferAccessor<'buf> {
    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }

    pub(crate) fn new(
        buffer: Arc<BufferCacheData<'buf>>,
        base: &gltf::Accessor,
    ) -> Result<Self, AccessorError> {
        let view = base.view().ok_or(AccessorError::Sparse)?;

        if base.normalized() {
            return Err(AccessorError::Normalized);
        }

        if view.stride().unwrap_or(0) > base.size() {
            return Err(AccessorError::Stride);
        }

        Ok(Self {
            buffer,
            offset: base.offset() + view.offset(),
            count: base.count(),
            data_type: base.data_type(),
            dimensions: base.dimensions(),
        })
    }

    /// # Safety
    ///
    /// * T::Component::TYPE == self.data_type
    /// * T::DIMENSIONS == self.dimensions
    #[allow(unsafe_code)]
    pub unsafe fn as_slice<T: BufferType>(&self) -> &'buf [T] {
        tracing::trace!("taking slice from BufferAccessor");
        unsafe {
            slice::from_raw_parts(
                self.buffer.as_ptr().add(self.offset) as *const T,
                self.count,
            )
        }
    }

    #[allow(unsafe_code)]
    pub fn try_as_slice<T: BufferType>(&self) -> Result<&'buf [T], AccessorError> {
        if T::DIMENSIONS != self.dimensions {
            return Err(AccessorError::MismatchedDimensions {
                expected: self.dimensions,
                actual: T::DIMENSIONS,
            });
        }
        if T::Component::TYPE != self.data_type {
            return Err(AccessorError::MismatchedComponentType {
                expected: self.data_type,
                actual: T::Component::TYPE,
            });
        }
        Ok(unsafe { self.as_slice() })
    }
}

#[allow(unsafe_code)]
pub unsafe trait BufferComponent {
    const TYPE: DataType;
}

#[allow(unsafe_code)]
pub unsafe trait BufferType {
    type Component: BufferComponent;
    const DIMENSIONS: Dimensions;
}

/// Implementations of [BufferComponent] and [BufferType] for many types that can represent glTF
/// buffer data.
mod _impl_traits {
    use super::{BufferComponent, BufferType};
    use gltf::accessor::{DataType, Dimensions};

    macro_rules! impl_bufcomponent {
        ($type:ty, $gltf_type:path) => {
            unsafe impl BufferComponent for $type {
                const TYPE: DataType = $gltf_type;
            }
        };
    }

    impl_bufcomponent!(i8, DataType::I8);
    impl_bufcomponent!(i16, DataType::I16);
    impl_bufcomponent!(u8, DataType::U8);
    impl_bufcomponent!(u16, DataType::U16);
    impl_bufcomponent!(u32, DataType::U32);
    impl_bufcomponent!(f32, DataType::F32);

    /// Get the size in bytes of a [DataType] at compile-time.
    #[allow(unused)]
    const fn comp_bytes(comp: DataType) -> usize {
        use std::mem::size_of;
        match comp {
            DataType::I8 => size_of::<i8>(),
            DataType::I16 => size_of::<i16>(),
            DataType::U8 => size_of::<u8>(),
            DataType::U16 => size_of::<u16>(),
            DataType::U32 => size_of::<u32>(),
            DataType::F32 => size_of::<f32>(),
        }
    }

    /// Get the size in bytes of an attribute type of a given [Dimensions] with a component type of
    /// a given [DataType], at compile-time.
    #[allow(unused)]
    const fn type_bytes(ty: Dimensions, comp: DataType) -> usize {
        match ty {
            Dimensions::Scalar => 1 * comp_bytes(comp),
            Dimensions::Vec2 => 2 * comp_bytes(comp),
            Dimensions::Vec3 => 3 * comp_bytes(comp),
            Dimensions::Vec4 => 4 * comp_bytes(comp),
            Dimensions::Mat2 => 4 * comp_bytes(comp),
            Dimensions::Mat3 => 9 * comp_bytes(comp),
            Dimensions::Mat4 => 16 * comp_bytes(comp),
        }
    }

    /// A complicated macro that implements [BufferType] for a given type `Target` and statically validates
    /// that implementation -- i.e. it asserts at compile-time that [BufferType] is safe to
    /// implement for `Target`.
    macro_rules! impl_buftype {
        ($t:ident<$($C:ty, $c:ident);+: $CAlias:ident> => $Target:ty) => {
            $( // for every ($C, $c)
               const _: () = { // anonymous module
                type $CAlias = $C;
                // "size of type $C == size of component $c"
                static_assertions::const_assert_eq!(std::mem::size_of::<$CAlias>(), comp_bytes(DataType::$c));
                // "size of type $Target == size of attribute $t with component $c"
                static_assertions::const_assert_eq!(std::mem::size_of::<$Target>(), type_bytes(Dimensions::$t, DataType::$c));
                unsafe impl BufferType for $Target {
                    type Component = $C;
                    const DIMENSIONS: Dimensions = Dimensions::$t;
                }
               };
            )+
        };
        ($t:ident<$($C:ident, $c:ident);+> => $Target:ty) => {
            impl_buftype!($t<$($C, $c);+: __MacroImplC> => $Target);
        };
        ($t:ident<$CAlias:ident> => $Target:ty) => {
            impl_buftype!($t<
                u8, U8;
                u16, U16;
                u32, U32;
                i8, I8;
                i16, I16;
                f32, F32: $CAlias> => $Target);
        };
    }

    impl_buftype!(Scalar<C> => C);
    impl_buftype!(Scalar<C> => [C; 1]);

    impl_buftype!(Vec2<C> => nalgebra::Vector2<C>);
    impl_buftype!(Vec2<C> => nalgebra::Point2<C>);
    impl_buftype!(Vec2<C> => [C; 2]);
    impl_buftype!(Vec2<C> => (C, C));

    impl_buftype!(Vec3<C> => nalgebra::Vector3<C>);
    impl_buftype!(Vec3<C> => nalgebra::Point3<C>);
    impl_buftype!(Vec3<C> => [C; 3]);
    impl_buftype!(Vec3<C> => (C, C, C));

    impl_buftype!(Vec4<C> => nalgebra::Vector4<C>);
    impl_buftype!(Vec4<C> => nalgebra::Point4<C>);
    impl_buftype!(Vec4<C> => [C; 4]);
    impl_buftype!(Vec4<C> => (C, C, C, C));

    impl_buftype!(Mat2<C> => nalgebra::Matrix2<C>);
    // impl_buftype!(Mat2<C> => [C; 4]); // this would be ambiguous with Vec4

    impl_buftype!(Mat3<C> => nalgebra::Matrix3<C>);
    impl_buftype!(Mat3<C> => [C; 9]);

    impl_buftype!(Mat4<C> => nalgebra::Matrix4<C>);
    impl_buftype!(Mat4<C> => [C; 16]);
}
