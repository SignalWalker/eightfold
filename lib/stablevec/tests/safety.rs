use stablevec::StableVec;

/// Test safety of capacity reservation functions
#[test]
fn reserve() {}

/// Test safety of `StableVec`<()>
#[test]
fn zst() {
    let mut zst = StableVec::<()>::new();
    zst.reserve(0);
    zst.reserve(1);
    zst.reserve_exact(0);
    zst.reserve_exact(1);
}
