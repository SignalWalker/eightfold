use nalgebra::{point, vector, Point3, Unit, Vector3};

use crate::preview::Turn;

#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct Polar3 {
    /// Distance from center.
    #[cfg_attr(test, proptest(strategy = "::proptest::num::f32::POSITIVE"))]
    pub rad: f32,
    /// Inclination from horizon.
    pub inc: Turn,
    /// Azimuth (rotation clockwise from forward (+Z)).
    pub azi: Turn,
}

impl std::fmt::Display for Polar3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(prec) = f.precision() {
            write!(
                f,
                "[{:.prec$}, {:.prec$}, {:.prec$}]",
                self.rad, self.inc, self.azi
            )
        } else {
            write!(f, "[{}, {}, {}]", self.rad, self.inc, self.azi)
        }
    }
}

impl Polar3 {
    pub const ORIGIN: Self = Self::new(0.0, Turn(0), Turn(0));

    #[inline]
    pub const fn new(rad: f32, inc: Turn, azi: Turn) -> Self {
        Self { rad, inc, azi }
    }

    /// Convert this polar point to a cartesian point.
    /// +X is right, +Y is up, +Z is forward.
    pub fn into_cartesian(self) -> Point3<f32> {
        let (azi_sin, azi_cos) = self.azi.sin_cos32();
        let (inc_sin, inc_cos) = self.inc.sin_cos32();
        let rad_inc_cos = self.rad * inc_cos;
        point![
            rad_inc_cos * azi_sin,
            self.rad * inc_sin,
            rad_inc_cos * azi_cos,
        ]
    }

    pub fn into_cartesian_vector(self) -> Vector3<f32> {
        let p = self.into_cartesian();
        Vector3::new(p.x, p.y, p.z)
    }

    /// Convert a cartesian point in the glTF coordinate system to a polar point.
    pub fn from_cartesian(p: Point3<f32>) -> Self {
        todo!()
    }

    /// The normal vector pointing from the origin to the position described by `self`.
    pub fn into_cartesian_normal(self) -> Unit<Vector3<f32>> {
        let (azi_sin, azi_cos) = self.azi.sin_cos32();
        let (inc_sin, inc_cos) = self.inc.sin_cos32();
        Unit::new_unchecked(vector![azi_sin * inc_cos, inc_sin, azi_cos * inc_cos])
    }
}
