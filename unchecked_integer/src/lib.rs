#![feature(core_intrinsics)]
#![allow(non_camel_case_types)]
use std::ops::{Add, Sub, Mul, Div};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UncheckedInt<T>(pub T);

impl<T> UncheckedInt<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn get(self) -> T {
        self.0
    }
}

macro_rules! impl_unchecked_ops {
    ($($t:ty),*) => {
        $(
            impl Add<$t> for UncheckedInt<$t> {
                type Output = Self;

                fn add(self, rhs: $t) -> Self::Output {
                    Self(unsafe { std::intrinsics::unchecked_add(self.0, rhs) })
                }
            }

            impl Add<UncheckedInt<$t>> for $t {
                type Output = UncheckedInt<$t>;

                fn add(self, rhs: UncheckedInt<$t>) -> Self::Output {
                    UncheckedInt(unsafe { std::intrinsics::unchecked_add(self, rhs.0) })
                }
            }

            impl Add for UncheckedInt<$t> {
                type Output = Self;

                fn add(self, rhs: Self) -> Self::Output {
                    Self(unsafe { std::intrinsics::unchecked_add(self.0, rhs.0) })
                }
            }

            impl Sub for UncheckedInt<$t> {
                type Output = Self;

                fn sub(self, rhs: Self) -> Self::Output {
                    // SAFETY: The caller must ensure no overflow occurs.
                    Self(unsafe { self.0.unchecked_sub(rhs.0) })
                }
            }

            impl Mul for UncheckedInt<$t> {
                type Output = Self;

                fn mul(self, rhs: Self) -> Self::Output {
                    // SAFETY: The caller must ensure no overflow occurs.
                    Self(unsafe { self.0.unchecked_mul(rhs.0) })
                }
            }

            impl Div for UncheckedInt<$t> {
                type Output = Self;

                fn div(self, rhs: Self) -> Self::Output {
                    // SAFETY: The caller must ensure no overflow occurs (e.g., no division by zero).
                    Self(unsafe { self.0.unchecked_div(rhs.0) })
                }
            }

            impl From<UncheckedInt<$t>> for $t {
                fn from(value: UncheckedInt<$t>) -> Self {
                    value.0
                }
            }

            impl From<$t> for UncheckedInt<$t> {
                fn from(value: $t) -> Self {
                    Self(value)
                }
            }

            // impl<FromType, ToType> From<UncheckedInt<FromType>> for UncheckedInt<ToType>
            // where
            //     FromType: Into<ToType>,
            // {
            //     fn from(value: UncheckedInt<FromType>) -> Self {
            //         Self(value.0.into())
            //     }
            // }

            // impl From<usize> for UncheckedInt<$t> {fn from(value: usize) -> Self {Self(value as $t)}}
        )*
    };
}

// Implement unchecked operations for primitive integer types
impl_unchecked_ops!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);
pub type i8u = UncheckedInt<i8>;
pub type i16u = UncheckedInt<i16>;
pub type i32u = UncheckedInt<i32>;
pub type i64u = UncheckedInt<i64>;
pub type i128u = UncheckedInt<i128>;
pub type u8u = UncheckedInt<u8>;
pub type u16u = UncheckedInt<u16>;
pub type u32u = UncheckedInt<u32>;
pub type u64u = UncheckedInt<u64>;
pub type u128u = UncheckedInt<u128>;

// Add support for unchecked operations
trait UncheckedArithmetic {
    unsafe fn unchecked_add(self, other: Self) -> Self;
    unsafe fn unchecked_sub(self, other: Self) -> Self;
    unsafe fn unchecked_mul(self, other: Self) -> Self;
    unsafe fn unchecked_div(self, other: Self) -> Self;
}

macro_rules! impl_unchecked_arithmetic {
    ($($t:ty),*) => {
        $(
            impl UncheckedArithmetic for $t {
                unsafe fn unchecked_add(self, other: Self) -> Self {
                    std::intrinsics::unchecked_add(self, other)
                }

                unsafe fn unchecked_sub(self, other: Self) -> Self {
                    std::intrinsics::unchecked_sub(self, other)
                }

                unsafe fn unchecked_mul(self, other: Self) -> Self {
                    std::intrinsics::unchecked_mul(self, other)
                }

                unsafe fn unchecked_div(self, other: Self) -> Self {
                    // MORE UB TO THE GOD OF UB 
                    if other == 0 {
                        std::hint::unreachable_unchecked();
                    }
                    self / other
                }
            }
        )*
    };
}

impl_unchecked_arithmetic!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unchecked_add() {
        let a: i32u = UncheckedInt(5);
        let b = 10;
        let c = a + b;
        assert_eq!(c.get(), 15);
    }

    #[test]
    fn unchecked_sub() {
        let a = i16u::new(5);
        let b = UncheckedInt::new(10);
        let c = a - b;
        assert_eq!(c.get(), -5);
    }

    #[test]
    fn unchecked_mul() {
        let a: i16u = UncheckedInt::new(5);
        let b = UncheckedInt::new(10);
        let c: i32u = (a * b).try_into().unwrap();
        assert_eq!(c.get(), 50);
    }

    #[test]
    fn unchecked_div() {
        let a = UncheckedInt::new(5);
        let b = UncheckedInt::new(10);
        let c = a / b;
        assert_eq!(c.get(), 5);
    }

    #[test]
    #[should_panic]
    fn unchecked_div_by_zero() {
        let a = UncheckedInt::new(5);
        let b = UncheckedInt::new(0);
        let _ = a / b;
    }
}


// fn main() {
//     let a = UncheckedInt::new(100u32);
//     let b = UncheckedInt::new(50u32);

//     let c = a + b; // Unchecked addition
//     println!("{:?}", c.get());
// }
