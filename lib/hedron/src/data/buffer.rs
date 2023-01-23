use std::{
    mem::MaybeUninit,
    ops::{Deref, Index, IndexMut},
    rc::Rc,
    sync::Arc,
};

#[cfg(feature = "wgpu")]
mod wgpu;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("BufferView range out of bounds of Buffer")]
    RangeOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferType {
    Array,
    ElementArray,
}

impl BufferType {
    pub fn to_gltf(self) -> u16 {
        match self {
            Self::Array => 34962,
            Self::ElementArray => 34963,
        }
    }

    pub fn from_gltf(val: u16) -> Result<Self, &'static str> {
        match val {
            34962 => Ok(Self::Array),
            34963 => Ok(Self::ElementArray),
            _ => Err("invalid BufferType value"),
        }
    }
}

/// A data blob which can be sent to the GPU or accessed through [BufferViews](BufferView)
#[derive(Default)]
pub struct Buffer {
    pub data: Vec<u8>,
}

impl Buffer {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        self.data.as_slice()
    }

    #[inline]
    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        self.data.as_mut_slice()
    }

    #[inline]
    pub fn view<T>(&self, offset: usize, len: usize) -> Result<BufferView<T, &Self>, Error> {
        BufferView::new(offset, len, self)
    }

    #[inline]
    pub fn view_mut<T>(
        &mut self,
        offset: usize,
        len: usize,
    ) -> Result<BufferView<T, &mut Self>, Error> {
        BufferView::new(offset, len, self)
    }
}

impl From<Vec<u8>> for Buffer {
    fn from(data: Vec<u8>) -> Self {
        Self { data }
    }
}

pub trait BufferRefMut {
    fn buf_mut(&mut self) -> &mut Buffer;
}

impl BufferRefMut for Buffer {
    #[inline(always)]
    fn buf_mut(&mut self) -> &mut Buffer {
        self
    }
}

impl BufferRefMut for &mut Buffer {
    #[inline(always)]
    fn buf_mut(&mut self) -> &mut Buffer {
        self
    }
}

impl BufferRefMut for Rc<Buffer> {
    #[inline]
    fn buf_mut(&mut self) -> &mut Buffer {
        Rc::get_mut(self).unwrap()
    }
}

impl BufferRefMut for Arc<Buffer> {
    #[inline]
    fn buf_mut(&mut self) -> &mut Buffer {
        Arc::get_mut(self).unwrap()
    }
}

/// A typed slice of a [Buffer]
#[derive(Debug)]
pub struct BufferView<T, B: Deref<Target = Buffer>> {
    data: B,
    offset: usize,
    count: usize,
    _ty: std::marker::PhantomData<T>,
}

impl<T, B: Deref<Target = Buffer>> BufferView<T, B> {
    /// Construct a BufferView without performing safety checks
    ///
    /// # Safety
    ///
    /// * `data.data.as_ptr() as usize + offset` must be aligned to T
    /// * `len * size_of::<T> <= isize::MAX`
    /// * `offset + len * size_of::<T>() < data.data.len()`
    #[inline]
    #[allow(unsafe_code)]
    pub unsafe fn new_unsafe(offset: usize, len: usize, data: B) -> Self {
        Self {
            data,
            offset,
            count: len,
            _ty: std::marker::PhantomData {},
        }
    }
    /// Construct a BufferView without performing bounds checks
    ///
    /// # Panics
    ///
    /// * if `(data.data.as_ptr() as usize + offset) % std::mem::align_of::<T>() != 0`; this would result in unaligned memory access later
    /// * if `len * size_of::<T>() > isize::MAX`; see [std::slice safety](https://doc.rust-lang.org/nightly/std/slice/fn.from_raw_parts.html#safety)
    ///
    /// # Safety
    ///
    /// * `offset + len * size_of::<T>() < data.data.len()`
    #[inline]
    #[allow(unsafe_code)]
    pub unsafe fn new_unchecked(offset: usize, len: usize, data: B) -> Self {
        // safety checks
        // maintain alignment requirements
        assert!(
            (data.data.as_ptr() as usize).checked_add(offset).unwrap() % std::mem::align_of::<T>()
                == 0
        );
        // maintain slice length requirements
        assert!(len * std::mem::size_of::<T>() <= isize::MAX as usize);
        unsafe { Self::new_unsafe(offset, len, data) }
    }

    /// Construct a BufferView
    #[allow(unsafe_code)]
    pub fn new(offset: usize, len: usize, data: B) -> Result<Self, Error> {
        use std::mem::size_of;
        if offset + len * size_of::<T>() >= data.data.len() {
            Err(Error::RangeOverflow)
        } else {
            Ok(unsafe { Self::new_unchecked(offset, len, data) })
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.count
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    #[inline]
    #[allow(unsafe_code)]
    pub fn as_slice(&self) -> &[MaybeUninit<T>] {
        // safety guarantees upheld by constructors
        unsafe {
            std::slice::from_raw_parts(
                self.data.data.as_ptr().add(self.offset) as *const MaybeUninit<T>,
                self.count,
            )
        }
    }

    #[inline]
    pub fn get_unchecked(&self, index: usize) -> &MaybeUninit<T> {
        &self.as_slice()[index]
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&MaybeUninit<T>> {
        self.as_slice().get(index)
    }
}

impl<T, B: Deref<Target = Buffer> + BufferRefMut> BufferView<T, B> {
    #[inline]
    #[allow(unsafe_code)]
    pub fn as_slice_mut(&mut self) -> &mut [MaybeUninit<T>] {
        // safety guarantees upheld by constructors
        unsafe {
            std::slice::from_raw_parts_mut(
                self.data.buf_mut().data.as_ptr().add(self.offset) as *mut MaybeUninit<T>,
                self.count,
            )
        }
    }

    #[inline]
    pub fn get_unchecked_mut(&mut self, index: usize) -> &mut MaybeUninit<T> {
        &mut self.as_slice_mut()[index]
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut MaybeUninit<T>> {
        self.as_slice_mut().get_mut(index)
    }
}

impl<T, B: Deref<Target = Buffer>> Index<usize> for BufferView<T, B> {
    type Output = MaybeUninit<T>;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.get_unchecked(index)
    }
}

impl<T, B: Deref<Target = Buffer> + BufferRefMut> IndexMut<usize> for BufferView<T, B> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_unchecked_mut(index)
    }
}
