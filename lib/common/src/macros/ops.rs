/// Implement a trait (and its `*Assign` sibling) from std::ops for every permutation of two types as owned and as references.
#[macro_export]
macro_rules! impl_op {
  // Implement $Op for each pair in [($Lhs, $Rhs), ($Lhs, &$Rhs), (&$Lhs, $Rhs), (&$Lhs, &$Rhs)]
  (. $Op:ident, $op:ident -> $Result:ty;
    $lhs:ident: $Lhs:ty, $rhs:ident: $Rhs:ty;
    $act:expr) => {
      impl $Op<$Rhs> for $Lhs {
          type Output = $Result;
          #[inline]
          fn $op($lhs, $rhs: $Rhs) -> Self::Output {
              $act
          }
      }
      impl $Op<$Rhs> for &$Lhs {
          type Output = $Result;
          #[inline]
          fn $op($lhs, $rhs: $Rhs) -> Self::Output {
              $act
          }
      }
      impl $Op<&$Rhs> for $Lhs {
          type Output = $Result;
          #[inline]
          fn $op($lhs, $rhs: &$Rhs) -> Self::Output {
              $act
          }
      }
      impl $Op<&$Rhs> for &$Lhs {
          type Output = $Result;
          #[inline]
          fn $op($lhs, $rhs: &$Rhs) -> Self::Output {
              $act
          }
      }
  };
  // Implement $Op for each pair in [($Lhs, $Rhs), ($Lhs, &$Rhs), (&$Lhs, $Rhs), (&$Lhs, &$Rhs)], where the output type is $Lhs.
  (. $Op:ident, $op:ident;
    $lhs:ident: $Lhs:ty, $rhs:ident: $Rhs:ty;
    $act:expr) => { $crate::impl_op!($Op, $op; $Lhs, $Rhs, $Lhs; $lhs, $rhs; $act) };
  // Implement $OpAssign for $Lhs over [$Rhs, &$Rhs]
  (= $Assign:ident, $assign:ident;
    $lhs:ident: $Lhs:ty, $rhs:ident: $Rhs:ty;
    $act:expr) => {
      impl $Assign<$Rhs> for $Lhs {
          #[inline]
          fn $assign(&mut $lhs, $rhs: $Rhs) {
              $act
          }
      }
      impl $Assign<&$Rhs> for $Lhs {
          #[inline]
          fn $assign(&mut $lhs, $rhs: &$Rhs) {
              $act
          }
      }
  };
  ($Op:ident, $op:ident -> $Result:ty, $Assign:ident, $assign:ident;
    $lhs:ident: $Lhs:ty, $rhs:ident: $Rhs:ty;
    $op_act:expr;
    $assign_act:expr
    ) => {
      $crate::impl_op!(. $Op, $op -> $Result; $lhs: $Lhs, $rhs: $Rhs; $op_act);
      $crate::impl_op!(= $Assign, $assign; $lhs: $Lhs, $rhs: $Rhs; $assign_act);
  };
  ($Op:ident, $op:ident, $Assign:ident, $assign:ident;
    $lhs:ident: $Lhs:ty, $rhs:ident: $Rhs:ty;
    $op_act:expr;
    $assign_act:expr
    ) => {
      $crate::impl_op!{$Op, $op -> $Lhs, $Assign, $assign; $lhs: $Lhs, $rhs: $Rhs; $op_act; $assign_act}
  };
}

#[macro_export]
macro_rules! impl_add_sub {
    ($lhs:ident: $Lhs:ty, $rhs:ident: $Rhs:ty; ($add:expr; $add_asn:expr); ($sub:expr; $sub_asn:expr)) => {
        $crate::impl_op! {Add, add, AddAssign, add_assign; $lhs: $Lhs, $rhs: $Rhs; $add; $add_asn}
        $crate::impl_op! {Sub, sub, SubAssign, sub_assign; $lhs: $Lhs, $rhs: $Rhs; $sub; $sub_asn}
    };
}

#[macro_export]
macro_rules! impl_mul_div {
    ($lhs:ident: $Lhs:ty, $rhs:ident: $Rhs:ty; ($mul:expr; $mul_asn:expr); ($div:expr; $div_asn:expr)) => {
        $crate::impl_op! {Mul, mul, MulAssign, mul_assign; $lhs: $Lhs, $rhs: $Rhs; $mul; $mul_asn}
        $crate::impl_op! {Div, div, DivAssign, div_assign; $lhs: $Lhs, $rhs: $Rhs; $div; $div_asn}
    };
}
