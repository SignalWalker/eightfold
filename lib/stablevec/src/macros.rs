/// Create a [StableVec] containing the arguments, as with [std::vec!]`.
#[macro_export]
macro_rules! stablevec {
    [] => {
        $crate::StableVec::new()
    };
    [$elem:expr; $n:expr] => {
        unsafe {
            $crate::StableVec::from_raw_parts(
                Box::new([std::mem::MaybeUninit::new($elem); $n]),
                $crate::bitvec::vec::BitVec::repeat(true, $n),
                $n
            )
        }
    };
    [$($x:expr),+ $(,)?] => {
        unsafe{
            let data = Box::new([$(std::mem::MaybeUninit::new($x)),+]);
            let cap = data.len();
            $crate::StableVec::from_raw_parts(
                data,
                $crate::bitvec::vec::BitVec::repeat(true, cap),
                cap
            )
        }
    }
}
