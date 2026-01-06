use std::{f32::consts::TAU as TAU_32, f64::consts::TAU as TAU_64};

use bytemuck::{Pod, Zeroable};
use nalgebra::{vector, Unit, UnitComplex, Vector2};

type TurnPart = u16;

/// A unit of plane angle equal to `Tau / 2^16` Radians, with wrapping operations.
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash, Pod, Zeroable)]
#[repr(transparent)]
pub struct Turn(pub TurnPart);

impl std::fmt::Debug for Turn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(p) = f.precision() {
            write!(f, "Turn({} ({self:.p$}))", self.0)
        } else {
            write!(f, "Turn({} ({self}))", self.0)
        }
    }
}

impl std::fmt::Display for Turn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(precision) = f.precision() {
            write!(f, "{:.precision$}tr", self.as_turn32())
        } else {
            write!(f, "{}tr", self.as_turn32())
        }
    }
}

impl From<TurnPart> for Turn {
    #[inline]
    fn from(r: TurnPart) -> Self {
        Self(r)
    }
}

impl From<Turn> for TurnPart {
    #[inline]
    fn from(r: Turn) -> Self {
        r.0
    }
}

impl std::ops::Neg for Turn {
    type Output = Turn;

    fn neg(self) -> Self::Output {
        self.negate()
    }
}

macro_rules! impl_turn_ops {
    ($Lhs:ty, $Rhs:ty; $rhs:ident -> $val:expr) => {
        impl std::ops::Add<$Rhs> for $Lhs {
            type Output = Turn;
            #[inline]
            fn add(self, $rhs: $Rhs) -> Self::Output {
                Turn(self.0.wrapping_add($val))
            }
        }
        impl std::ops::Sub<$Rhs> for $Lhs {
            type Output = Turn;
            #[inline]
            fn sub(self, $rhs: $Rhs) -> Self::Output {
                Turn(self.0.wrapping_sub($val))
            }
        }
        //impl std::ops::Mul<$Rhs> for $Lhs {
        //    type Output = Turn;
        //    #[inline]
        //    fn mul(self, $rhs: $Rhs) -> Self::Output {
        //        Turn(self.0.wrapping_mul($val))
        //    }
        //}
        //impl std::ops::Div<$Rhs> for $Lhs {
        //    type Output = Turn;
        //    #[inline]
        //    fn div(self, $rhs: $Rhs) -> Self::Output {
        //        Turn(self.0.wrapping_div($val))
        //    }
        //}
    };
}

macro_rules! impl_op_assign {
    ($Lhs:ty, $Rhs:ty; $rhs:ident -> $val:expr) => {
        impl std::ops::AddAssign<$Rhs> for $Lhs {
            #[inline]
            fn add_assign(&mut self, $rhs: $Rhs) {
                self.0 = self.0.wrapping_add($val);
            }
        }
        impl std::ops::SubAssign<$Rhs> for $Lhs {
            #[inline]
            fn sub_assign(&mut self, $rhs: $Rhs) {
                self.0 = self.0.wrapping_sub($val);
            }
        }
        //impl std::ops::MulAssign<$Rhs> for $Lhs {
        //    #[inline]
        //    fn mul_assign(&mut self, $rhs: $Rhs) {
        //        self.0 = self.0.wrapping_mul($val);
        //    }
        //}
        //impl std::ops::DivAssign<$Rhs> for $Lhs {
        //    #[inline]
        //    fn div_assign(&mut self, $rhs: $Rhs) {
        //        self.0 = self.0.wrapping_div($val);
        //    }
        //}
    };
}

macro_rules! impl_ops {
    ($Rhs:ty; $rhs:ident -> $val:expr) => {
        impl_turn_ops!(Turn, $Rhs; $rhs -> $val);
        impl_turn_ops!(&Turn, $Rhs; $rhs -> $val);
        impl_op_assign!(Turn, $Rhs; $rhs -> $val);
    };
}

impl_ops!(TurnPart; rhs -> rhs);
impl_ops!(&TurnPart; rhs -> *rhs);
// impl_ops!(Wrapping<TurnPart>; rhs -> rhs.0);
// impl_ops!(&Wrapping<TurnPart>; rhs -> rhs.0);
// impl_ops!(Saturating<TurnPart>; rhs -> rhs.0);
// impl_ops!(&Saturating<TurnPart>; rhs -> rhs.0);
impl_ops!(Turn; rhs -> rhs.0);
impl_ops!(&Turn; rhs -> rhs.0);
impl_ops!(f32; rhs -> Turn::from_radians32(rhs).0);
impl_ops!(&f32; rhs -> Turn::from_radians32(*rhs).0);
impl_ops!(f64; rhs -> Turn::from_radians64(rhs).0);
impl_ops!(&f64; rhs -> Turn::from_radians64(*rhs).0);

impl Turn {
    /// The maximum turn before wrapping.
    pub const MAX: Self = Self::new(TurnPart::MAX);
    pub const MAX_F32: f32 = TurnPart::MAX as f32;
    pub const MAX_F64: f64 = TurnPart::MAX as f64;
    pub const HALF: Self = Self::new(2u16.pow(15));
    pub const FOURTH: Self = Self::new(2u16.pow(14));

    #[inline]
    pub const fn new(part: TurnPart) -> Self {
        Self(part)
    }

    /// Compares and returns the maximum of two angles.
    pub fn max(self, rhs: Self) -> Self {
        Self(self.0.max(rhs.0))
    }

    /// Compares and returns the minimum of two angles.
    pub fn min(self, rhs: Self) -> Self {
        Self(self.0.min(rhs.0))
    }

    #[inline]
    pub const fn negate(self) -> Self {
        // TODO :: i feel like there is a slightly cleverer way to do this
        Self(0u16.wrapping_sub(self.0))
    }

    #[inline]
    pub const fn inverse(self) -> Self {
        Self(self.0.wrapping_add(Self::HALF.0))
    }

    #[inline]
    pub fn unit_complex32(self) -> UnitComplex<f32> {
        UnitComplex::new((-self).to_radians32())
    }

    #[inline]
    pub fn normal32(self) -> Unit<Vector2<f32>> {
        let sc = self.sin_cos32();
        Unit::new_unchecked(vector![sc.0, sc.1])
    }
}

macro_rules! impl_rads {
    ($tau:expr; $max:expr; $from:ident, $to:ident -> $Radians:ty) => {
        impl Turn {
            #[inline]
            pub fn $from(rads: $Radians) -> Self {
                let rads = rads.rem_euclid($tau);
                Self::new(((rads / $tau) * $max) as TurnPart)
            }
            #[inline]
            pub const fn $to(self) -> $Radians {
                (self.0 as $Radians / ($max + 1.0)) * $tau
            }
        }

        impl From<$Radians> for Turn {
            fn from(rads: $Radians) -> Self {
                Self::$from(rads)
            }
        }

        impl From<Turn> for $Radians {
            fn from(turn: Turn) -> Self {
                turn.$to()
            }
        }
    };
}

impl_rads!(TAU_32; Turn::MAX_F32; from_radians32, to_radians32 -> f32);
impl_rads!(TAU_64; Turn::MAX_F64; from_radians64, to_radians64 -> f64);

macro_rules! impl_turn_frac {
    ($tau:expr, $MAX:expr; $Output:ty; $as_turn:ident, $from_turn:ident) => {
        impl Turn {
            #[inline]
            pub const fn $as_turn(self) -> $Output {
                self.0 as $Output / ($MAX + 1.0)
            }

            #[inline]
            pub const fn $from_turn(t: $Output) -> Self {
                Self::new((t * $MAX) as TurnPart)
            }
        }
    };
}

impl_turn_frac!(std::f32::consts::TAU, Turn::MAX_F32; f32; as_turn32, from_turn32);
impl_turn_frac!(std::f64::consts::TAU, Turn::MAX_F64; f64; as_turn64, from_turn64);

//pub trait TurnFloat {
//    const TAU: Self;
//    const TURN: Self;
//    fn rem_euclid(self, div: Self) -> Self;
//    fn sin(self) -> Self;
//    fn cos(self) -> Self;
//    fn sin_cos(self) -> (Self, Self);
//    fn from_turnpart(p: TurnPart) -> Self;
//}
//
//macro_rules! impl_turn_float {
//    ($Float:ty, $TAU:expr) => {
//        impl TurnFloat for $Float {
//            const TAU: Self = $TAU;
//            const TURN: Self = Turn::MAX.0 as $Float + 1.0;
//            #[inline]
//            fn rem_euclid(self, div: Self) -> Self {
//                <$Float>::rem_euclid(self, div)
//            }
//            #[inline]
//            fn sin(self) -> Self {
//                <$Float>::sin(self)
//            }
//            #[inline]
//            fn cos(self) -> Self {
//                <$Float>::cos(self)
//            }
//            #[inline]
//            fn sin_cos(self) -> (Self, Self) {
//                <$Float>::sin_cos(self)
//            }
//            #[inline]
//            fn from_turnpart(p: TurnPart) -> Self {
//                p as $Float
//            }
//        }
//    };
//}
//
//impl_turn_float!(f32, ::std::f32::consts::TAU);

pub trait TurnExt<Float> {
    fn from_radians(rad: Float) -> Self;
    fn to_radians(self) -> Float;
    fn to_turn(self) -> Float;
    fn sin(self) -> Float;
    fn cos(self) -> Float;
    fn sin_cos(self) -> (Float, Float);
}

macro_rules! impl_turn_ext {
    ($Float:ty, $TAU:expr, $MAX:expr) => {
        impl TurnExt<$Float> for Turn {
            fn from_radians(rad: $Float) -> Self {
                Self::new(((rad.rem_euclid($TAU) / $TAU) * $MAX) as TurnPart)
            }
            fn to_radians(self) -> $Float {
                (self.0 as $Float / $MAX) * $TAU
            }
            fn to_turn(self) -> $Float {
                self.0 as $Float / $MAX
            }
            fn sin(self) -> $Float {
                TurnExt::<$Float>::to_radians(self).sin()
            }
            fn cos(self) -> $Float {
                TurnExt::<$Float>::to_radians(self).cos()
            }
            fn sin_cos(self) -> ($Float, $Float) {
                TurnExt::<$Float>::to_radians(self).sin_cos()
            }
        }
    };
}

impl_turn_ext!(f32, ::std::f32::consts::TAU, Turn::MAX_F32);
impl_turn_ext!(f64, ::std::f64::consts::TAU, Turn::MAX_F64);

macro_rules! impl_trig {
    ($self:ident: $to_rads:expr => $output:ty; $sin:ident, $cos:ident, $sin_cos:ident) => {
        impl Turn {
            /// The sine of this angle.
            #[inline]
            pub fn $sin($self: Self) -> $output {
                $to_rads.sin()
            }

            /// The cosine of this angle.
            #[inline]
            pub fn $cos($self: Self) -> $output {
                $to_rads.cos()
            }

            /// The sine and cosine of this angle.
            #[inline]
            pub fn $sin_cos($self: Self) -> ($output, $output) {
                $to_rads.sin_cos()
            }
        }
    };
}

impl_trig!(self: self.to_radians32() => f32; sin32, cos32, sin_cos32);
impl_trig!(self: self.to_radians64() => f64; sin64, cos64, sin_cos64);
