//! Ensure that StableVec macros work correctly.

use stablevec::stablevec;

#[test]
fn empty() {
    let _: stablevec::StableVec<()> = stablevec![];
}

#[test]
fn repeat() {
    stablevec![false; 0];
    stablevec![true; 12];
}

#[test]
fn complex() {
    stablevec![0, 1, 2, 4, 4, 6, 7, 8, 9, 10, 11];
}
