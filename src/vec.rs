//! [StableVec], etc.

use std::{
    alloc::Layout,
    collections::HashMap,
    iter::FromIterator,
    mem::{self, MaybeUninit},
    ops::{Index, IndexMut},
    ptr::{self, NonNull},
    slice,
};

use bitvec::{slice::BitSlice, vec::BitVec};
use num_traits::{AsPrimitive, PrimInt};

/// Create a [StableVec] containing the arguments, as with `std::vec![]`.
#[macro_export]
macro_rules! stablevec {
    () => {
        $crate::vec::StableVec::new()
    };
    ($elem:expr; $n:expr) => {
        unsafe {
            $crate::vec::StableVec::from_parts(
                Vec::from_elem($elem, $n),
                bitvec::vec::BitVec::repeat(true, $n),
            )
        }
    };
    ($($x:expr),+ $(,)?) => {
        unsafe{
            let data = Box::new([$(std::mem::MaybeUninit::new($x)),+]);
            let cap = data.len();
            $crate::vec::StableVec::from_parts(
                std::ptr::NonNull::new_unchecked(Box::leak(data) as *mut _),
                bitvec::vec::BitVec::repeat(true, cap),
                cap,
                cap,
                Some(cap)
            )
        }
    }
}

/// A vector type with indices stable across removes & inserts, and which reuses deleted indices; comparable to Vec<Option<T>>.
///
/// This is very similar to the [Slab](https://crates.io/crates/slab) crate, which we aren't using because Slab's Vec entries are at
/// least `size_of<usize>() + 1` bytes -- since we're using this primarily to store things smaller
/// than usize, it's much more efficient to use this.
///
/// There's also a [stable-vec crate](https://crates.io/crates/stable-vec) but that was last
/// updated in 2019, so I think it's unmaintained.
///
/// Honestly, at the moment, I'm not really happy with this. It's all a bit haphazard.
#[derive(Debug)]
pub struct StableVec<T> {
    data: NonNull<MaybeUninit<T>>,
    flags: BitVec,
    cap: usize,
    /// number of initialized values
    init_amt: usize,
    /// index of the initialized value farthest from the beginning of the vector
    farthest_init: Option<usize>,
}

impl<T> Default for StableVec<T> {
    fn default() -> Self {
        Self {
            data: NonNull::dangling(),
            flags: BitVec::new(),
            cap: 0,
            init_amt: 0,
            farthest_init: None,
        }
    }
}

impl<T: Copy> Clone for StableVec<T> {
    fn clone(&self) -> Self {
        let (cap, data) = match self.flags.last_one() {
            Some(l) => unsafe {
                let cap = l + 1;
                if std::mem::size_of::<T>() == 0 {
                    (cap, NonNull::dangling())
                } else {
                    let p = Self::alloc(cap);
                    ptr::copy_nonoverlapping(self.data.as_ptr(), p.as_ptr(), cap);
                    (cap, p)
                }
            },
            None => (0, NonNull::dangling()),
        };
        Self {
            data,
            flags: BitVec::from_bitslice(&self.flags[0..cap + 1]),
            cap,
            init_amt: self.init_amt,
            farthest_init: self.farthest_init,
        }
    }
}

impl<T> StableVec<T> {
    /// # Safety
    ///
    /// * `data` must be allocated
    /// * `flags` must have length = length of `data` allocation
    /// * `cap` = length of `data` allocation
    /// * `init_amt` = number of initialized entries in `data`
    /// * `farthest_init` = index of initialized value at highest index, or None if no initialized values
    pub unsafe fn from_parts(
        data: NonNull<MaybeUninit<T>>,
        flags: BitVec,
        cap: usize,
        init_amt: usize,
        farthest_init: Option<usize>,
    ) -> Self {
        Self {
            data,
            flags,
            cap,
            init_amt,
            farthest_init,
        }
    }

    pub fn new() -> Self {
        Self {
            data: NonNull::dangling(),
            flags: BitVec::default(),
            cap: 0,
            init_amt: 0,
            farthest_init: None,
        }
    }

    pub fn farthest_init_index(&self) -> Option<usize> {
        self.farthest_init
    }

    pub fn with_capacity(cap: usize) -> Self {
        let mut res = Self::new();
        unsafe {
            res.realloc_unchecked(cap);
        }
        res
    }

    /// Get the number of empty spaces within this StableVec
    pub fn spare_slots(&self) -> usize {
        self.flags.len() - self.init_amt
    }

    /// Get the number of elements that can be added to this StableVec without allocation
    pub fn spare_capacity(&self) -> usize {
        self.cap - self.init_amt
    }

    /// Get the number of initialized elements in the StableVec
    pub fn len_init(&self) -> usize {
        self.init_amt
    }

    /// # Safety
    ///
    /// * `index` < `self.cap`
    pub unsafe fn get_raw_unsafe(&self, index: usize) -> &MaybeUninit<T> {
        debug_assert!(index < self.cap);
        &*self.data.as_ptr().add(index)
    }

    /// # Safety
    ///
    /// * `index` < `self.cap`
    pub unsafe fn get_raw_unsafe_mut(&mut self, index: usize) -> &mut MaybeUninit<T> {
        debug_assert!(index < self.cap);
        &mut *self.data.as_ptr().add(index)
    }

    /// # Safety
    ///
    /// * `index` < `self.cap`
    /// * `self[index]` must already be initialized
    pub unsafe fn get_unsafe(&self, index: usize) -> &T {
        debug_assert!(self.is_init(index));
        self.get_raw_unsafe(index).assume_init_ref()
    }

    /// # Safety
    ///
    /// * `index` < `self.cap`
    /// * `self[index]` must already be initialized
    pub unsafe fn get_unsafe_mut(&mut self, index: usize) -> &mut T {
        debug_assert!(self.is_init(index));
        self.get_raw_unsafe_mut(index).assume_init_mut()
    }

    pub fn get_unchecked(&self, index: usize) -> &T {
        debug_assert!(self.is_init(index));
        if !self.flags[index] {
            panic!("attempted to access uninitialized value in StableVec")
        }
        unsafe { self.get_unsafe(index) }
    }

    pub fn get_unchecked_mut(&mut self, index: usize) -> &mut T {
        debug_assert!(self.is_init(index));
        if !self.flags[index] {
            panic!("attempted to access uninitialized value in StableVec")
        }
        unsafe { self.get_unsafe_mut(index) }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        match self.flags.get(index).as_deref() {
            None | Some(false) => None,
            Some(true) => Some(unsafe { self.get_unsafe(index) }),
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        match self.flags.get(index).map(|b| *b) {
            None | Some(false) => None,
            Some(true) => Some(unsafe { self.get_unsafe_mut(index) }),
        }
    }

    /// Drop everything.
    pub fn clear(&mut self) {
        // TODO :: skip this for T: ?Drop
        unsafe {
            let p = self.data.as_ptr();
            for i in self.flags.iter_ones() {
                ptr::drop_in_place::<T>(&mut *p.add(i).cast::<T>());
            }
        }
        self.init_amt = 0;
        self.farthest_init = None;
        self.flags.clear();
    }

    /// Drop everything and shrink to minimum size.
    pub fn dealloc(&mut self) {
        if self.cap == 0 {
            return;
        }
        self.clear();
        self.cap = 0;
        self.flags.shrink_to_fit();
        unsafe {
            std::alloc::dealloc(
                self.data.as_ptr() as *mut u8,
                Layout::array::<MaybeUninit<T>>(self.cap).unwrap(),
            )
        };
    }

    unsafe fn alloc(cap: usize) -> NonNull<MaybeUninit<T>> {
        if mem::size_of::<T>() == 0 || cap == 0 {
            return NonNull::dangling();
        }
        NonNull::new_unchecked(
            std::alloc::alloc(Layout::array::<MaybeUninit<T>>(cap).unwrap()) as *mut MaybeUninit<T>,
        )
    }

    /// # Safety
    ///
    /// * `new_cap` >= self.flags.len()
    unsafe fn realloc_unchecked(&mut self, new_cap: usize) {
        debug_assert!(new_cap >= self.flags.len());
        if mem::size_of::<T>() == 0 {
            self.cap = new_cap;
            return;
        }

        let new_data = Self::alloc(new_cap);
        if self.cap > 0 {
            ptr::copy_nonoverlapping(self.data.as_ptr(), new_data.as_ptr(), new_cap);
            std::alloc::dealloc(
                self.data.as_ptr() as *mut u8,
                Layout::array::<MaybeUninit<T>>(self.cap).unwrap(),
            );
        }
        self.cap = new_cap;
        self.data = new_data;
    }

    /// # Panics
    ///
    /// * `new_cap` < `self.flags.len()`
    fn realloc_exact(&mut self, new_cap: usize) {
        if new_cap < self.flags.len() {
            panic!()
        }

        if mem::size_of::<T>() == 0 {
            self.cap = new_cap;
            return;
        }

        if self.cap == new_cap {
            return;
        }

        unsafe { self.realloc_unchecked(new_cap) };
    }

    /// # Panics
    ///
    /// * `new_cap` < self.flags.len()
    fn realloc_at_least(&mut self, new_cap: usize) {
        if new_cap < self.flags.len() {
            panic!()
        }

        if mem::size_of::<T>() == 0 {
            self.cap = self.cap.max(new_cap);
            return;
        }

        if self.cap >= new_cap {
            return;
        }

        unsafe { self.realloc_unchecked(new_cap) };
    }

    pub fn shrink_to_fit(&mut self) {
        if self.cap == 0 || self.cap == self.init_amt {
            return;
        }
        if self.init_amt == 0 {
            self.dealloc();
            return;
        }
        unsafe { self.realloc_unchecked(self.flags.iter_ones().last().unwrap() + 1) };
        self.flags.truncate(self.cap);
        self.flags.shrink_to_fit();
    }

    pub fn from_vec(data: Vec<T>) -> Self {
        let cap = data.capacity();
        let len = data.len();
        Self {
            data: unsafe {
                NonNull::new_unchecked(data.leak().as_mut_ptr() as *mut MaybeUninit<T>)
            },
            flags: BitVec::repeat(true, len),
            cap,
            init_amt: len,
            farthest_init: Some(len - 1),
        }
    }

    pub fn spare_capacity_mut(&mut self) -> &mut [MaybeUninit<T>] {
        let f_idx = self.farthest_init.unwrap_or(0);
        unsafe { slice::from_raw_parts_mut(self.data.as_ptr().add(f_idx), self.cap - f_idx) }
    }

    pub(crate) unsafe fn remove_at_unchecked(&mut self, index: usize) -> Option<T> {
        if self.flags.replace_unchecked(index, false) {
            self.init_amt -= 1;
            if self.farthest_init == Some(index) {
                self.farthest_init = self.flags.last_one();
            }
            Some(self.data.as_ptr().add(index).cast::<T>().read())
        } else {
            None
        }
    }

    unsafe fn set_at_unchecked(&mut self, index: usize, data: T) {
        (*self.data.as_ptr().add(index)).write(data);
        if !self.flags.replace_unchecked(index, true) {
            self.init_amt += 1;
            match self.farthest_init {
                Some(i) if i < index => self.farthest_init = Some(index),
                _ => {}
            }
        }
    }

    pub fn next_push_index(&self) -> usize {
        if self.spare_capacity() == 0 {
            self.cap
        } else {
            self.flags.first_zero().unwrap_or(self.cap)
        }
    }

    pub fn push(&mut self, data: T) -> usize {
        let idx;
        if self.spare_capacity() == 0 {
            idx = self.cap;
            self.reserve(1);
        } else {
            idx = self.flags.first_zero().unwrap_or(self.cap);
        }
        unsafe {
            self.set_at_unchecked(idx, data);
        }
        idx
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index >= self.cap {
            return None;
        }
        unsafe { self.remove_at_unchecked(index) }
    }

    pub fn replace(&mut self, index: usize, data: T) -> Option<T> {
        if index >= self.cap {
            return None;
        }
        if !unsafe { self.flags.replace_unchecked(index, true) } {
            self.init_amt += 1;
            unsafe {
                self.set_at_unchecked(index, data);
                None
            }
        } else {
            unsafe {
                let res = self.data.as_ptr().add(index).cast::<T>().read();
                self.set_at_unchecked(index, data);
                Some(res)
            }
        }
    }

    pub fn reserve_exact(&mut self, mut amt: usize) {
        amt = match amt.checked_sub(self.spare_capacity()) {
            None | Some(0) => return,
            Some(a) => a,
        };
        self.flags.reserve_exact(amt);
        if std::mem::size_of::<T>() == 0 {
            self.cap += amt;
        } else {
            unsafe { self.realloc_unchecked(amt + self.cap) };
        }
    }

    pub fn reserve(&mut self, mut amt: usize) {
        amt = match 8.max(amt).checked_sub(self.spare_capacity()) {
            None | Some(0) => return,
            Some(a) => a,
        };
        self.flags.reserve(amt);
        if std::mem::size_of::<T>() == 0 {
            self.cap += amt;
        } else {
            unsafe { self.realloc_unchecked(self.cap + amt) };
        }
    }

    #[must_use]
    #[allow(clippy::double_must_use)]
    pub fn push_iter<'s, Idx: PrimInt + 'static>(
        &'s mut self,
        iter: impl IntoIterator<Item = T> + 's,
    ) -> impl Iterator<Item = Idx> + 's
    where
        usize: AsPrimitive<Idx>,
    {
        iter.into_iter().map(|item| self.push(item).as_())
    }

    pub fn extend_from_iter(&mut self, iter: impl IntoIterator<Item = T>) -> Vec<usize> {
        let mut res = Vec::new();
        for item in iter {
            res.push(self.push(item));
        }
        res
    }

    pub fn extend_from_other(&mut self, mut other: Self) -> HashMap<usize, usize> {
        match other.farthest_init {
            Some(f) => {
                self.reserve_exact(other.len_init());
                let odata = other.data.as_ptr() as *mut T;
                let mut res = HashMap::new();
                for i in 0..=f {
                    if other.flags[i] {
                        res.insert(i, self.push(unsafe { odata.add(i).read() }));
                        unsafe { other.flags.set_unchecked(i, false) };
                    }
                }
                res
            }
            None => HashMap::with_capacity(0),
        }
    }

    /// # Safety
    ///
    /// * Must initialize all returned values
    #[must_use]
    #[allow(clippy::uninit_assumed_init, clippy::needless_lifetimes)]
    pub unsafe fn prepare_raw_array<'s, const LEN: usize>(
        &'s mut self,
    ) -> [&'s mut MaybeUninit<T>; LEN] {
        let sp = self as *mut Self;
        // this is pretty similar to the array initialization example from the std library docs
        let mut res: [&mut MaybeUninit<T>; LEN] = MaybeUninit::uninit().assume_init();
        self.reserve_exact(LEN);
        let mut idx = 0;
        for e in &mut res {
            idx = self.next_push_index();
            *e = (*sp).get_raw_unsafe_mut(idx);
            self.flags.set(idx, true);
        }
        self.init_amt += LEN;
        self.farthest_init = Some(self.farthest_init.map_or(idx, |f| f.max(idx)));
        res
    }

    pub fn extend_from_slice_cloned(&mut self, slice: &[T]) -> Vec<usize>
    where
        T: Clone,
    {
        todo!()
    }

    pub fn extend_from_slice_copied(&mut self, slice: &[T]) -> Vec<usize>
    where
        T: Copy,
    {
        todo!()
    }

    pub fn as_uninit_slice(&self) -> &[MaybeUninit<T>] {
        unsafe { slice::from_raw_parts(self.data.as_ptr() as *const _, self.cap) }
    }

    pub fn as_uninit_slice_mut(&mut self) -> &mut [MaybeUninit<T>] {
        unsafe { slice::from_raw_parts_mut(self.data.as_ptr() as *mut _, self.cap) }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.flags.iter_ones().map(|i| &self[i])
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.flags.iter_ones().map(|i| todo!())
    }

    pub fn enumerate(&self) -> impl Iterator<Item = (usize, &T)> {
        self.flags.iter_ones().map(|i| (i, &self[i]))
    }

    pub fn is_init(&self, index: usize) -> bool {
        self.flags.get(index).map(|b| *b).unwrap_or(false)
    }

    pub fn is_fragmented(&self) -> bool {
        match self.flags.first_zero() {
            None => false,
            Some(i) => i <= self.init_amt,
        }
    }

    /// # Safety
    ///
    /// * `a` < `self.cap`
    /// * `b` < `self.cap`
    pub unsafe fn swap_unchecked(&mut self, a: usize, b: usize) {
        let data = self.data.as_ptr();
        let ap = data.add(a);
        let bp = data.add(b);
        std::ptr::swap_nonoverlapping(ap, bp, 1);
        self.flags.swap_unchecked(a, b);
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        if a >= self.cap || b >= self.cap {
            panic!()
        }
        unsafe {
            self.swap_unchecked(a, b);
        }
    }

    /// Partition self such that initialized values are stored contiguously from index 0; return the new locations of
    /// each value in the form `(from, to)`.
    pub fn defragment(&mut self) -> Vec<(usize, usize)> {
        match self.farthest_init {
            Some(f) => {
                let mut first_uninit = match self.flags.first_zero() {
                    Some(l) => l,
                    None => return Vec::with_capacity(0), // not fragmented, because `self` is
                                                          // completely full
                };
                let mut res = Vec::new();
                let mut i = f;
                let data = self.data.as_ptr();
                while i > first_uninit {
                    if self.flags[i] {
                        unsafe {
                            // `copy` instead of `swap` because we don't actually need to care
                            // about whatever data was in `self[first_uninit]`
                            std::ptr::copy_nonoverlapping(data.add(i), data.add(first_uninit), 1);
                            self.flags.set_unchecked(first_uninit, true);
                            self.flags.set_unchecked(i, false);
                        }
                        res.push((i, first_uninit));
                        first_uninit = match self.flags[(first_uninit + 1)..i].first_zero() {
                            Some(z) => z,
                            None => break, // fully partitioned
                        };
                    }
                    i -= 1;
                }
                self.farthest_init = Some(self.init_amt - 1);
                res
            }
            None => Vec::with_capacity(0),
        }
    }

    /// [Defragment](Self::defragment) & reallocate self such that `self.cap` == `self.init_amt`.
    pub fn compress(&mut self) -> Vec<(usize, usize)> {
        let res = self.defragment();
        self.flags.truncate(self.init_amt);
        unsafe {
            self.realloc_unchecked(self.init_amt);
        }
        res
    }

    pub fn as_slice(&self) -> Option<&[T]> {
        if self.is_fragmented() {
            None
        } else {
            Some(unsafe {
                std::slice::from_raw_parts(self.data.as_ptr() as *const T, self.init_amt)
            })
        }
    }

    pub fn as_slice_mut(&mut self) -> Option<&mut [T]> {
        if self.is_fragmented() {
            None
        } else {
            Some(unsafe {
                std::slice::from_raw_parts_mut(self.data.as_ptr() as *mut T, self.init_amt)
            })
        }
    }

    pub fn init_flags(&self) -> &BitVec {
        &self.flags
    }
}

impl<T> Drop for StableVec<T> {
    fn drop(&mut self) {
        self.dealloc();
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
        self.get_unchecked(index)
    }
}

impl<T> IndexMut<usize> for StableVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_unchecked_mut(index)
    }
}

impl<T> FromIterator<T> for StableVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::from_vec(iter.into_iter().collect())
    }
}

// impl<T> IntoIterator for StableVec<T> {
//     type Item = T;
//     type IntoIter = ();
//     fn into_iter(self) -> Self::IntoIter {
//         self.flags.iter_ones().map(|i| self[])
//     }
// }
