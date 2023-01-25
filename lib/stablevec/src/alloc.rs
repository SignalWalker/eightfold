//! Utilities related to allocation & deallocation of [`StableVecs`](StableVec).
//!
//! Much of this is very similar to (or taken almost verbatim from) [`std::alloc::RawVec`], which would be used directly if it was
//! part of std's public interface.

use std::{
    alloc::{handle_alloc_error, Layout, LayoutError},
    mem::{self, MaybeUninit},
    ptr::NonNull,
    slice,
};

use bitvec::vec::BitVec;

use crate::StableVec;

#[inline(always)]
pub(crate) const fn is_zst<T>() -> bool {
    mem::size_of::<T>() == 0
}

fn capacity_overflow() -> ! {
    panic!("capacity overflow")
}

#[inline]
const fn can_alloc(size: usize) -> bool {
    usize::BITS >= 64 || size <= isize::MAX as usize
}

fn allocate<T>(cap: usize) -> Box<[MaybeUninit<T>]> {
    if is_zst::<T>() || cap == 0 {
        return unsafe {
            Box::from_raw(slice::from_raw_parts_mut(
                NonNull::<MaybeUninit<T>>::dangling().as_mut(),
                cap,
            ))
        };
    }

    let layout = match Layout::array::<MaybeUninit<T>>(cap) {
        Ok(l) => l,
        Err(_) => capacity_overflow(),
    };
    if !can_alloc(layout.size()) {
        capacity_overflow()
    }
    let ptr = unsafe { std::alloc::alloc(layout) };
    if ptr.is_null() {
        handle_alloc_error(layout)
    }

    unsafe {
        let slice = slice::from_raw_parts_mut(ptr.cast::<std::mem::MaybeUninit<T>>(), cap);
        Box::<[MaybeUninit<T>]>::from_raw(slice)
    }
}

/// # Safety
///
/// * Caller must ensure that `T`'s [Copy]/[Clone] rules are followed
// unsafe fn allocate_copy_with_unsafe<T>(
//     new_cap: usize,
//     data: &[MaybeUninit<T>],
// ) -> Box<[MaybeUninit<T>]> {
//     assert!(new_cap >= data.len());
//     let mut res = allocate::<T>(new_cap);

//     if is_zst::<T>() || data.len() == 0 {
//         return res;
//     }

//     unsafe {
//         data.as_ptr()
//             .copy_to_nonoverlapping(res.as_mut_ptr(), data.len());
//     }

//     res
// }

// fn allocate_copy_with<T: Copy>(new_cap: usize, data: &[MaybeUninit<T>]) -> Box<[MaybeUninit<T>]> {
//     unsafe { allocate_copy_with_unsafe(new_cap, data) }
// }

// fn allocate_copy<T: Copy>(data: &[MaybeUninit<T>]) -> Box<[MaybeUninit<T>]> {
//     allocate_copy_with(data.len(), data)
// }

#[inline(never)]
#[warn(clippy::needless_pass_by_value)]
fn finish_grow(
    new_layout: Result<Layout, LayoutError>,
    current_memory: Option<(NonNull<u8>, Layout)>,
) -> NonNull<u8> {
    let new_layout = match new_layout {
        Ok(l) => l,
        Err(_) => capacity_overflow(),
    };
    if !can_alloc(new_layout.size()) {
        capacity_overflow()
    }

    let res = unsafe { std::alloc::alloc(new_layout) };
    if res.is_null() {
        handle_alloc_error(new_layout)
    }

    if let Some((ptr, old_layout)) = current_memory {
        debug_assert_eq!(old_layout.align(), new_layout.align());
        unsafe {
            ptr.as_ptr().copy_to_nonoverlapping(res, old_layout.size());
            std::alloc::dealloc(ptr.as_ptr(), old_layout);
        }
    }
    unsafe { NonNull::new_unchecked(res) }
}

impl<T: Clone> From<&[T]> for StableVec<T> {
    fn from(data: &[T]) -> Self {
        if data.is_empty() {
            return Self::new();
        }

        let mut res = allocate::<T>(data.len());
        for (i, e) in data.iter().enumerate() {
            res[i].write(e.clone());
        }

        Self {
            data: res,
            flags: BitVec::repeat(true, data.len()),
            count: data.len(),
            cap: data.len(),
        }
    }
}

// impl<T: Copy> From<&[T]> for StableVec<T> {
//     fn from(data: &[T]) -> Self {
//         todo!()
//     }
// }

impl<T: Clone> Clone for StableVec<T> {
    fn clone(&self) -> Self {
        let mut res = Self::with_capacity(self.capacity());
        for i in self.flags.iter_ones() {
            unsafe { res.data[i].write(self.data[i].assume_init_ref().clone()) };
        }
        res.count = self.count;
        res
    }
}

// impl<T: Copy> Clone for StableVec<T> {
//     fn clone(&self) -> Self {
//         todo!()
//     }
// }

impl<T> StableVec<T> {
    /// Minimum reserved capacity. Strategy taken from the standard library's `RawVec` type.
    const MIN_NON_ZERO_CAP: usize = if mem::size_of::<T>() == 1 {
        8
    } else if mem::size_of::<T>() <= 1024 {
        4
    } else {
        1
    };

    /// Construct a [`StableVec`] from its raw components.
    ///
    /// # Safety
    ///
    /// * `flags[i]` ⟺ `data[i]` is initialized
    /// * `flags.len()` == `data.len()`
    pub unsafe fn from_raw_parts(data: Box<[MaybeUninit<T>]>, flags: BitVec, count: usize) -> Self {
        let cap = data.len();
        Self {
            data,
            flags,
            count,
            cap,
        }
    }

    /// Create a new, empty [`StableVec`].
    pub fn new() -> Self {
        Self {
            data: Box::new([]),
            flags: BitVec::new(),
            count: 0,
            cap: 0,
        }
    }

    /// Create a new [`StableVec`] with a specific capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            data: allocate::<T>(cap),
            flags: BitVec::repeat(false, cap),
            count: 0,
            cap,
        }
    }

    pub(crate) fn expand_flags(&mut self, new_len: usize) {
        let extra = match new_len.checked_sub(self.flags.len()) {
            Some(e) => e,
            None => return,
        };
        self.flags.reserve_exact(extra);
        for _i in 0..extra {
            self.flags.push(false);
        }
    }

    /// Ensure that at least `additional` more elements can be added to the `StableVec` without
    /// reallocation.
    pub fn reserve(&mut self, additional: usize) {
        if let Some(amt) = additional.checked_sub(self.spare_capacity()) {
            self.grow_amortized(amt);
        }
    }

    /// Ensure that at least `additional` more elements can be added to the `StableVec` without
    /// reallocation.
    pub fn reserve_exact(&mut self, additional: usize) {
        if let Some(amt) = additional.checked_sub(self.spare_capacity()) {
            self.grow_exact(amt);
        }
    }

    pub fn shrink_to_fit(&mut self) {
        let new_cap = match self.flags.last_one() {
            Some(c) => c + 1,
            None => return,
        };
        unsafe { self.shrink(new_cap) };
    }
}

impl<T> StableVec<T> {
    fn leak_memory(&mut self) -> Option<(NonNull<u8>, Layout)> {
        if is_zst::<T>() || self.capacity() == 0 {
            None
        } else {
            unsafe {
                // must do this before raw-ifying self.data, because `self.capacity()` relies on
                // `self.data.len()`
                let layout = Layout::array::<MaybeUninit<T>>(self.capacity()).unwrap_unchecked();
                Some((
                    NonNull::new_unchecked(
                        // use `std::mem::replace` to swap `self.data` with a dummy value, so that
                        // we don't accidentally double-free the data we're leaking here
                        Box::<[MaybeUninit<T>]>::into_raw(std::mem::replace(
                            &mut self.data,
                            Box::new([]),
                        ))
                        .cast::<u8>(),
                    ),
                    layout,
                ))
            }
        }
    }

    /// # Safety
    ///
    /// * `self.capacity()` <= `new_cap`
    #[inline(always)]
    unsafe fn grow(&mut self, new_cap: usize) {
        if !is_zst::<T>() {
            let new_layout = Layout::array::<T>(new_cap);
            let mem = finish_grow(new_layout, self.leak_memory());
            unsafe {
                self.data = Box::from_raw(slice::from_raw_parts_mut(
                    mem.as_ptr().cast::<MaybeUninit<T>>(),
                    new_cap,
                ));
            }
        }
        self.cap = new_cap;
        self.expand_flags(self.capacity());
    }

    pub(crate) fn grow_exact(&mut self, additional: usize) {
        unsafe {
            self.grow(match self.capacity().checked_add(additional) {
                Some(c) => c,
                None => capacity_overflow(),
            })
        }
    }
    pub(crate) fn grow_amortized(&mut self, additional: usize) {
        unsafe {
            self.grow(match self.capacity().checked_add(additional) {
                Some(c) => c.max(self.len_init() * 2).max(Self::MIN_NON_ZERO_CAP),
                None => capacity_overflow(),
            })
        }
    }

    /// # Safety
    ///
    /// * `self.capacity()` >= `new_cap`
    pub(crate) unsafe fn shrink(&mut self, new_cap: usize) {
        debug_assert!(new_cap <= self.capacity());
        let (ptr, layout) = if let Some(mem) = self.leak_memory() {
            mem
        } else {
            // `T` is a zero-sized type
            return;
        };
        let ptr = unsafe {
            let new_layout = Layout::array::<MaybeUninit<T>>(new_cap).unwrap_unchecked();
            let res = std::alloc::alloc(new_layout);
            if res.is_null() {
                handle_alloc_error(layout);
            }
            ptr.as_ptr().copy_to_nonoverlapping(res, new_layout.size());
            std::alloc::dealloc(ptr.as_ptr(), layout);
            res
        };
        self.data = unsafe {
            Box::from_raw(slice::from_raw_parts_mut(
                ptr.cast::<MaybeUninit<T>>(),
                new_cap,
            ))
        };
        self.flags.truncate(new_cap);
        self.flags.shrink_to_fit();
    }
}
