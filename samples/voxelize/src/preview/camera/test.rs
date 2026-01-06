use proptest::prelude::Arbitrary;

macro_rules! prop_assert_relative_eq {
    ($given:expr, $expected:expr) => {
        ::proptest::prop_assert!(
            ::approx::relative_eq!($given, $expected),
            "assert_relative_eq!({}, {})
    given = {:?}
    expec = {:?}

",
            ::std::stringify!($given),
            ::std::stringify!($expected),
            $given,
            $expected
        )
    };
}

mod turn {
    use proptest::prelude::*;

    use crate::preview::Turn;

    macro_rules! assert_radians {
        ($a:expr, $Float:ident, $tau:ident, $max:ident, $b:expr) => {{
            #[allow(unused)]
            {
                use ::std::f32::consts::TAU as $tau;
                use f32 as $Float;
                const $max: f32 = $crate::preview::Turn::MAX_F32;
                prop_assert_relative_eq!(($a).to_radians32(), $b);
            }
            #[allow(unused)]
            {
                use ::std::f64::consts::TAU as $tau;
                use f64 as $Float;
                const $max: f64 = $crate::preview::Turn::MAX_F64;
                prop_assert_relative_eq!(($a).to_radians64(), $b);
            }
        }};
    }

    macro_rules! assert_float {
        ($Float:ident, $TAU:ident, $MAX:ident, $a:expr, $b:expr) => {{
            #[allow(unused)]
            {
                use ::std::f32::consts::TAU as $TAU;
                use f32 as $Float;
                const $MAX: f32 = $crate::preview::Turn::MAX_F32;
                prop_assert_relative_eq!($a, $b);
            }
            #[allow(unused)]
            {
                use ::std::f64::consts::TAU as $TAU;
                use f64 as $Float;
                const $MAX: f64 = $crate::preview::Turn::MAX_F64;
                prop_assert_relative_eq!($a, $b);
            }
        }};
    }

    proptest! {
        #[test]
        fn to_radians(t in any::<Turn>()) {
            assert_radians!(t, Float, tau, MAX, (Float::from(t.0) / (MAX + 1.0)) * tau);
        }
        #[test]
        fn from_radians(rad in 0.0..=::std::f64::consts::TAU) {
            use std::f64::consts::TAU;
            prop_assert_eq!(Turn::from_radians64(rad), Turn(((rad.rem_euclid(TAU) / TAU) * Turn::MAX_F64) as u16));
        }
        #[test]
        fn negate(t in any::<Turn>()) {
            prop_assert_eq!(t + (-t), Turn(0));
        }

        #[test]
        fn normal_is_normal(t in any::<Turn>()) {
            prop_assert_relative_eq!(t.normal32().into_inner().norm(), 1.0);
        }

        #[test]
        fn inverse(t in any::<Turn>()) {
            prop_assert_relative_eq!(t.inverse().normal32(), -t.normal32());
        }
    }
}

mod polar {
    use approx::assert_relative_eq;
    use nalgebra::Unit;
    use proptest::prelude::*;

    use crate::preview::{Polar3, Turn};

    proptest! {
        #[test]
        fn into_cartesian_is_left_handed(p in any::<Polar3>()) {
            fn b_to_s(b: bool) -> &'static str {
                if b {
                    "positive"
                } else {
                    "negative"
                }
            }
            use std::f64::consts::PI;
            let inc = p.inc.to_radians64();
            let inc_flipped = (PI / 2.0..PI * 3.0 / 2.0).contains(&inc);
            prop_assume!(!inc_flipped);
            let azi = p.azi.to_radians64();
            let azi_forward = !(PI / 2.0..PI * 3.0 / 2.0).contains(&azi);
            let right = azi <= PI;
            let up = inc <= PI;
            let forward = azi_forward ^ inc_flipped;
            let c = p.into_cartesian();
            prop_assert!((right && c.x >= 0.0) || (!right && c.x <= 0.0), "X should be {}, but is {}", b_to_s(right), c.x);
            prop_assert!((up && c.y >= 0.0) || (!up && c.y <= 0.0), "Y should be {}, but is {}", b_to_s(up), c.y);
            prop_assert!((forward && c.z >= 0.0) || (!forward && c.z <= 0.0), "Z should be {}, but is {}", b_to_s(forward), c.z);
        }

        #[test]
        fn cartesian_normal_is_normal(inc in any::<Turn>(), azi in any::<Turn>()) {
            let p = Polar3::new(1.0, inc, azi);
            prop_assert_relative_eq!(p.into_cartesian_normal().norm(), 1.0);
        }

        #[test]
        fn cartesian_normal_eq_normalized_cartesian(inc in any::<Turn>(), azi in any::<Turn>()) {
            let p = Polar3::new(10.0, inc, azi);
            let c = Unit::new_normalize(p.into_cartesian_vector());
            let n = p.into_cartesian_normal();
            prop_assert_relative_eq!(n, c);
        }
    }
}
