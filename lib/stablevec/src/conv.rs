//! Conversion of various types to [StableVec].

use std::{
    mem::{self, MaybeUninit},
    slice,
};

use bitvec::vec::BitVec;

use crate::StableVec;

impl<T> From<Box<[T]>> for StableVec<T> {
    fn from(b: Box<[T]>) -> Self {
        let len = b.len();
        if len == 0 {
            return Self::new();
        }
        Self {
            data: unsafe { mem::transmute::<Box<[T]>, Box<[MaybeUninit<T>]>>(b) },
            flags: BitVec::repeat(true, len),
            count: len,
        }
    }
}

impl<T> From<Vec<T>> for StableVec<T> {
    fn from(v: Vec<T>) -> Self {
        if v.is_empty() {
            return Self::new();
        }

        let (ptr, len, cap) = {
            #[cfg(feature = "nightly")]
            {
                v.into_raw_parts()
            };
            #[cfg(not(feature = "nightly"))]
            {
                let cap = v.capacity();
                let slice = v.leak();
                (slice.as_mut_ptr(), slice.len(), cap)
            }
        };

        Self {
            data: unsafe {
                Box::from_raw(slice::from_raw_parts_mut(ptr as *mut MaybeUninit<T>, cap))
            },
            flags: {
                let mut res = BitVec::repeat(true, len);
                let extra = cap - len;
                res.reserve_exact(extra);
                for _i in 0..extra {
                    res.push(false);
                }
                res
            },
            count: len,
        }
    }
}

impl<T> FromIterator<T> for StableVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::from(iter.into_iter().collect::<Vec<_>>())
    }
}
