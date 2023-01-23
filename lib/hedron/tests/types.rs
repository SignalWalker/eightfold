// Ensure that DataSets can compile with any unsigned index type with width <= size_of::<usize>
// #[test]
// #[allow(clippy::just_underscores_and_digits)]
// fn dataset_index() {
//     let _8 = DataSet::<u8>::empty();
//     let _16 = DataSet::<u16>::empty();
//     #[cfg(not(target_pointer_width = "16"))]
//     {
//         // size_of::<usize>() > 16
//         let _32 = DataSet::<u32>::empty();
//         #[cfg(not(target_pointer_width = "32"))]
//         {
//             // size_of::<usize>() > 32
//             let _64 = DataSet::<u64>::empty();
//         }
//     }
//     let _size = DataSet::<usize>::empty();
// }
