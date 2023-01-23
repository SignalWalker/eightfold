use eightfold::Octree;

/// Ensure that Octrees can compile with any unsigned index type with width <= `size_of::`<usize>
#[test]
#[allow(clippy::just_underscores_and_digits)]
fn tree_index() {
    let _8 = Octree::<(), u8>::new();
    let _16 = Octree::<(), u16>::new();
    #[cfg(not(target_pointer_width = "16"))]
    {
        // size_of::<usize>() > 16
        let _32 = Octree::<(), u32>::new();
        #[cfg(not(target_pointer_width = "32"))]
        {
            // size_of::<usize>() > 32
            let _64 = Octree::<(), u64>::new();
        }
    }
    let _size = Octree::<(), usize>::new();
}
