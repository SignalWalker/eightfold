#![doc = include_str!("../README.md")]
#![allow(
    unsafe_code,
    // reason = "this entire crate relies heavily on scary memory tricks"
)]

use std::{
    collections::HashMap,
    mem::{self, MaybeUninit},
    ops::{Index, IndexMut},
    ptr,
};

use bitvec::vec::BitVec;

mod alloc;
mod conv;
mod debug;
mod macros;

// reexport so the macro works outside of this package
pub use bitvec;

/// A vector type with indices stable across removals & insertions, and which reuses deleted indices; comparable to Vec<Option<T>>.
///
/// The API for this is really bad right now; tread carefully.
///
/// # Prior Work
///
/// This is very similar to the [Slab](https://crates.io/crates/slab) crate, which we aren't using because Slab's Vec entries are at
/// least `size_of<usize>() + 1` bytes -- since we're using this primarily to store things smaller
/// than usize, it should be more efficient to use this.
///
/// There's also the [stable-vec](https://crates.io/crates/stable-vec) crate but that was last
/// updated in 2019, so I think it's unmaintained. Also, it doesn't reuse empty indices, which we
/// want for this.
///
/// # Invariants
///
/// * `flags.len` == `data.len`
/// * `data[i]` is initialized ⟺ `flags[i] == true`
#[derive(Debug)]
pub struct StableVec<T> {
    /// Pointer to the array of `T` values.
    data: Box<[MaybeUninit<T>]>,
    /// Array of flags indicating whether a data entry is initialized.
    flags: BitVec<usize, bitvec::order::Lsb0>,
    /// Number of initialized values in the array
    count: usize,
}

impl<T> Default for StableVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> StableVec<T> {
    /// Get the total size of this StableVec.
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        self.data.len()
    }

    /// Get the number of empty slots in this StableVec.
    #[inline(always)]
    pub fn spare_capacity(&self) -> usize {
        self.capacity() - self.count
    }

    /// Get the number of initialized elements in the StableVec
    #[inline(always)]
    pub fn len_init(&self) -> usize {
        self.count
    }

    pub fn init_flags(&self) -> &BitVec {
        &self.flags
    }

    /// Set the value at a specific index.
    ///
    /// # Panics
    ///
    /// * `index` >= `self.capacity()`
    ///
    /// # Safety
    ///
    /// * `self.data[index]` must either be `?Drop` or uninitialized
    pub unsafe fn set_unchecked(&mut self, index: usize, data: T) {
        if !unsafe { self.flags.replace_unchecked(index, true) } {
            self.count += 1;
        }
        self.data[index].write(data);
    }

    /// Set the value at a specific index and return the previous value, if extant.
    pub fn set(&mut self, index: usize, data: T) -> Option<T> {
        let res = if self.flags.replace(index, true) {
            Some(unsafe { self.data[index].assume_init_read() })
        } else {
            self.count += 1;
            None
        };
        self.data[index].write(data);
        res
    }

    /// # Safety
    ///
    /// * `index` < `self.cap`
    pub unsafe fn remove_unchecked(&mut self, index: usize) -> Option<T> {
        if unsafe { self.flags.replace_unchecked(index, false) } {
            self.count -= 1;
            Some(unsafe { self.data.as_ptr().add(index).cast::<T>().read() })
        } else {
            None
        }
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index >= self.capacity() {
            return None;
        }
        unsafe { self.remove_unchecked(index) }
    }

    /// Get the value at a specific index.
    ///
    /// # Panics
    ///
    /// * `index` >= `self.capacity`
    ///
    /// # Safety
    ///
    /// * `self.data[index]` must already be initialized
    #[inline]
    pub const unsafe fn get_unchecked(&self, index: usize) -> &T {
        unsafe { self.data[index].assume_init_ref() }
    }

    /// Get the value at a specific index.
    ///
    /// # Panics
    ///
    /// * `index` >= `self.capacity`
    ///
    /// # Safety
    ///
    /// * `self.data[index]` must already be initialized
    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut T {
        unsafe { self.data[index].assume_init_mut() }
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        match self.flags.get(index).as_deref() {
            None | Some(false) => None,
            Some(true) => Some(unsafe { self.get_unchecked(index) }),
        }
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        let b = self.flags.get(index).map(|b| *b);
        match b {
            None | Some(false) => None,
            Some(true) => Some(unsafe { self.get_unchecked_mut(index) }),
        }
    }

    pub fn push(&mut self, data: T) -> usize {
        let index = match self.flags.first_one() {
            Some(i) => i,
            None => {
                self.grow_amortized(1);
                self.capacity() - 1
            }
        };
        unsafe { self.set_unchecked(index, data) };
        index
    }

    pub fn replace(&mut self, index: usize, data: T) -> Option<T> {
        if index >= self.capacity() {
            return None;
        }
        if !unsafe { self.flags.replace_unchecked(index, true) } {
            self.count += 1;
            unsafe {
                self.set_unchecked(index, data);
                None
            }
        } else {
            unsafe {
                let res = self.data[index].assume_init_read();
                self.set_unchecked(index, data);
                Some(res)
            }
        }
    }

    /// Drop everything.
    pub fn clear(&mut self) {
        // TODO :: skip this for T: ?Drop
        if mem::size_of::<T>() != 0 {
            unsafe {
                let p = self.data.as_mut_ptr();
                for i in self.flags.iter_ones() {
                    ptr::drop_in_place::<T>(&mut *p.add(i).cast::<T>());
                }
            }
        }
        self.count = 0;
        self.flags.clear();
    }

    #[inline]
    pub fn push_iter<'s>(
        &'s mut self,
        iter: impl IntoIterator<Item = T> + 's,
    ) -> impl Iterator<Item = usize> + 's {
        iter.into_iter().map(|item| self.push(item))
    }

    #[inline]
    pub fn extend_from_iter(&mut self, iter: impl IntoIterator<Item = T>) -> Vec<usize> {
        self.push_iter(iter).collect()
    }

    pub fn extend_from_other(&mut self, mut other: Self) -> HashMap<usize, usize> {
        let last_init = match self.flags.last_one() {
            Some(l) => l,
            None => return HashMap::with_capacity(0),
        };
        self.reserve_exact(other.len_init());
        let odata = other.data.as_ptr() as *mut T;
        let mut res = HashMap::new();
        for i in 0..=last_init {
            if other.flags[i] {
                res.insert(i, self.push(unsafe { odata.add(i).read() }));
                unsafe { other.flags.set_unchecked(i, false) };
            }
        }
        res
    }

    pub fn as_uninit_slice(&self) -> &[MaybeUninit<T>] {
        &self.data
    }

    pub fn as_uninit_slice_mut(&mut self) -> &mut [MaybeUninit<T>] {
        &mut self.data
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.flags.iter_ones().map(|i| &self[i])
    }

    // pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
    //     self.flags.iter_ones().map(|i| todo!())
    // }

    pub fn enumerate(&self) -> impl Iterator<Item = (usize, &T)> {
        self.flags.iter_ones().map(|i| (i, &self[i]))
    }

    pub fn is_init(&self, index: usize) -> bool {
        self.flags.get(index).map(|b| *b).unwrap_or(false)
    }

    pub fn is_fragmented(&self) -> bool {
        match self.flags.first_zero() {
            None => false,
            Some(i) => i <= self.count,
        }
    }

    /// # Safety
    ///
    /// * `a` < `self.cap`
    /// * `b` < `self.cap`
    pub unsafe fn swap_unchecked(&mut self, a: usize, b: usize) {
        let data = self.data.as_mut_ptr();
        unsafe {
            let ap = data.add(a);
            let bp = data.add(b);
            std::ptr::swap_nonoverlapping(ap, bp, 1);
            self.flags.swap_unchecked(a, b);
        }
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        if a >= self.capacity() || b >= self.capacity() {
            panic!()
        }
        unsafe {
            self.swap_unchecked(a, b);
        }
    }

    /// Partition self such that initialized values are stored contiguously from index 0; return the new locations of
    /// each value in the form `(from, to)`.
    pub fn defragment(&mut self) -> HashMap<usize, usize> {
        if self.count == 0 {
            return HashMap::with_capacity(0);
        }

        let mut first_uninit = match self.flags.first_zero() {
            Some(l) => l,
            None => return HashMap::with_capacity(0), // not fragmented, because `self` is
                                                      // completely full
        };
        let mut res = HashMap::new();
        let mut i = unsafe { self.flags.last_one().unwrap_unchecked() };
        let data = self.data.as_mut_ptr();
        while i > first_uninit {
            if self.flags[i] {
                unsafe {
                    // `copy` instead of `swap` because we don't actually need to care
                    // about whatever data was in `self[first_uninit]`
                    std::ptr::copy_nonoverlapping(data.add(i), data.add(first_uninit), 1);
                    self.flags.set_unchecked(first_uninit, true);
                    self.flags.set_unchecked(i, false);
                }
                res.insert(i, first_uninit);
                first_uninit = match self.flags[(first_uninit + 1)..i].first_zero() {
                    Some(z) => z,
                    None => break, // fully partitioned
                };
            }
            i -= 1;
        }
        res
    }

    /// [Defragment](Self::defragment) & reallocate self such that `self.cap` == `self.init_amt`.
    pub fn compress(&mut self) -> HashMap<usize, usize> {
        let res = self.defragment();
        self.shrink_to_fit();
        res
    }

    pub fn as_slice(&self) -> Option<&[T]> {
        if self.is_fragmented() {
            None
        } else {
            Some(unsafe { mem::transmute::<&[MaybeUninit<T>], &[T]>(&*self.data) })
        }
    }

    pub fn as_slice_mut(&mut self) -> Option<&mut [T]> {
        if self.is_fragmented() {
            None
        } else {
            Some(unsafe { mem::transmute::<&mut [MaybeUninit<T>], &mut [T]>(&mut *self.data) })
        }
    }
}

impl<T> Drop for StableVec<T> {
    fn drop(&mut self) {
        for i in self.flags.iter_ones() {
            unsafe { self.data[i].as_mut_ptr().drop_in_place() };
        }
    }
}

// impl<T: Drop> Drop for StableVec<T> {
//     fn drop(&mut self) {
//         // drop all initialized data entries
//         for i in self.flags.iter_ones() {
//             unsafe{self.get_raw_unsafe(i).assume_init()};
//         }
//     }
// }

impl<T> Index<usize> for StableVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe { self.get_unchecked(index) }
    }
}

impl<T> IndexMut<usize> for StableVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe { self.get_unchecked_mut(index) }
    }
}

// impl<T> IntoIterator for StableVec<T> {
//     type Item = T;
//     type IntoIter = ();
//     fn into_iter(self) -> Self::IntoIter {
//         self.flags.iter_ones().map(|i| self[])
//     }
// }
