use nalgebra::{proptest::vector, U3};
use proptest::prelude::*;

use crate::spatial::Aabb;

proptest! {
    #[test]
    fn new(a in vector(-1024.0..1024.0, U3), b in vector(-1024.0..1024.0, U3)) {
        let Aabb { mins, maxs } = Aabb::new(nalgebra::point![a.x, a.y, a.z], nalgebra::point![b.x, b.y, b.z]);
        assert!(mins.x <= maxs.x && mins.y <= maxs.y && mins.z <= maxs.z, "aabb.mins.* > aabb.maxs.*");
    }
}
