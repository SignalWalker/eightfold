use std::{borrow::Borrow, mem::MaybeUninit};

use crate::primitive::attribute::{
    storage::DynamicAttributeError, Attribute, AttributeComponent, AttributeType,
};

/// Abstraction over various methods of storing primitive attribute data, with runtime typing.
pub struct DynAttrStore<Base> {
    base: Base,
    ty: AttributeType,
    comp: AttributeComponent,
}

impl<Base: Borrow<[u8]>> DynAttrStore<Base> {
    #[inline(always)]
    pub fn attr_type(&self) -> AttributeType {
        self.ty
    }
    #[inline(always)]
    pub fn attr_component(&self) -> AttributeComponent {
        self.comp
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.base.borrow().len() / self.ty.size_bytes(self.comp)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// # Safety
    ///
    /// * alignment of `base` must equal alignment of `comp` type
    /// * `base.len() % ty.size_bytes(comp) == 0`
    ///
    /// i.e. you must be able to safely interpret `base` as `Borrow<[T]>`, for whatever `T` is described
    /// by `ty` and `comp`.
    ///
    /// Also, data in `base` must be initialized as the type described by `ty` and `comp`; i.e. must be able to comply with the initialization invariant of [MaybeUninit].
    #[inline]
    #[allow(unsafe_code)]
    pub const unsafe fn from_bytes_unsafe(
        base: Base,
        ty: AttributeType,
        comp: AttributeComponent,
    ) -> Self {
        Self { base, ty, comp }
    }

    /// # Panics
    ///
    /// * if alignment of `b` is not equal to alignment of `comp` type
    /// * if `b.len() % ty.size_bytes(comp) != 0`
    ///
    /// i.e. you must be able to safely interpret `base` as `Borrow<[T]>`, for whatever `T` is described
    /// by `ty` and `comp`.
    ///
    /// # Safety
    ///
    /// Data in `base` must be initialized as the type described by `ty` and `comp`; i.e. must be able to comply with the initialization invariant of [MaybeUninit].
    #[allow(unsafe_code)]
    pub unsafe fn from_bytes_unchecked(
        base: Base,
        ty: AttributeType,
        comp: AttributeComponent,
    ) -> Self {
        {
            let bytes = base.borrow();
            assert!(bytes.as_ptr() as usize % ty.alignment(comp) == 0);
            assert!(bytes.len() % ty.size_bytes(comp) == 0);
        }
        Self { base, ty, comp }
    }
    /// # Safety
    ///
    /// Data in `base` must be initialized as the type described by `ty` and `comp`; i.e. must be able to comply with the initialization invariant of [MaybeUninit].
    #[allow(unsafe_code)]
    pub unsafe fn from_bytes(
        base: Base,
        ty: AttributeType,
        comp: AttributeComponent,
    ) -> Result<Self, DynamicAttributeError> {
        // safety checks
        {
            let bytes: &[u8] = base.borrow();
            // ensure safe alignment
            // TODO :: switch to ptr::is_aligned once that's stable
            if bytes.as_ptr() as usize % ty.alignment(comp) != 0 {
                return Err(DynamicAttributeError::Alignment);
            }
            // ensure correct size
            if bytes.len() % ty.size_bytes(comp) != 0 {
                return Err(DynamicAttributeError::Size);
            }
        }
        Ok(Self { base, ty, comp })
    }

    /// Try to borrow self as a `[T]`, where `T: Attribute` for the attribute described by `self.ty` and `self.comp`.
    #[allow(unsafe_code)]
    pub fn try_borrow<T: Attribute>(&self) -> Result<&[MaybeUninit<T>], DynamicAttributeError> {
        if T::TYPE != self.ty {
            return Err(DynamicAttributeError::Width);
        }
        if T::COMPONENT != self.comp {
            return Err(DynamicAttributeError::Component);
        }
        Ok(unsafe { self.borrow_unchecked::<T>() })
    }

    /// # Panics
    ///
    /// * if `T` is a zero-sized type
    ///
    /// # Safety
    ///
    /// * alignment of `T` must match that of `self.base`
    /// * `self.base.len() % std::mem::size_of::<T>()` must equal 0.
    #[inline]
    #[allow(unsafe_code)]
    pub unsafe fn borrow_unchecked<T>(&self) -> &[MaybeUninit<T>] {
        let bytes: &[u8] = self.base.borrow();
        unsafe {
            std::slice::from_raw_parts(
                bytes.as_ptr() as *const MaybeUninit<T>,
                bytes.len() / std::mem::size_of::<T>(),
            )
        }
    }

    /// Try to borrow self as a `[T]`, where `T` has the correct size and alignment for the base data slice.
    #[allow(unsafe_code)]
    pub fn try_borrow_relaxed<T>(&self) -> Result<&[MaybeUninit<T>], DynamicAttributeError> {
        if std::mem::size_of::<T>() == 0 {
            return Err(DynamicAttributeError::BorrowedAsZST);
        }
        let bytes: &[u8] = self.base.borrow();
        if bytes.len() % std::mem::size_of::<T>() != 0 {
            return Err(DynamicAttributeError::Size);
        }
        if bytes.as_ptr() as usize % std::mem::align_of::<T>() != 0 {
            return Err(DynamicAttributeError::Alignment);
        }
        Ok(unsafe { self.borrow_unchecked::<T>() })
    }
}
