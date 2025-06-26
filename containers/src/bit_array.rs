use crate::array3d::Dim3;
use std::ops::{BitAnd, BitAndAssign, BitOrAssign, Not, Shl, Shr};

/// 3D array, but each element is a boolean
#[derive(Debug)]
pub struct BitArray3d<T, D: Dim3> {
    pub data: Box<[T]>,
    pub dims: D,
}

impl<T, D> BitArray3d<T, D>
where
    D: Dim3,
    T: Default
        + Copy
        + BitAnd<Output = T>
        + BitOrAssign
        + BitAndAssign
        + Shl<usize, Output = T>
        + Shr<usize, Output = T>
        + Not<Output = T>
        + PartialEq
        + From<u8>,
{
    const BITS: usize = std::mem::size_of::<T>() * 8;

    pub fn new(dims: D) -> Self {
        let total_bits = dims.total_len();
        let total_chunks = total_bits.div_ceil(Self::BITS);

        Self {
            dims,
            data: vec![T::default(); total_chunks].into_boxed_slice(),
        }
    }

    pub fn new_filled(dims: D, value: bool) -> Self {
        let total_bits = dims.total_len();
        let total_chunks = total_bits.div_ceil(Self::BITS);
        let fill_value = if value { !T::default() } else { T::default() };

        Self {
            dims,
            data: vec![fill_value; total_chunks].into_boxed_slice(),
        }
    }

    pub fn fill(&mut self, value: bool) {
        let fill_value = if value { !T::default() } else { T::default() };
        for slot in self.data.iter_mut() {
            *slot = fill_value;
        }
    }

    pub fn dimensions(&self) -> (usize, usize, usize) {
        (self.dims.x(), self.dims.y(), self.dims.z())
    }

    pub fn linear_index(&self, x: usize, y: usize, z: usize) -> usize {
        debug_assert!(x < self.dims.x() && y < self.dims.y() && z < self.dims.z());
        x + y * self.dims.x() + z * self.dims.x() * self.dims.y()
    }

    pub fn get(&self, x: usize, y: usize, z: usize) -> bool {
        let pos = self.linear_index(x, y, z);
        let chunk = pos / Self::BITS;
        let bit = pos % Self::BITS;

        if chunk >= self.data.len() {
            return false;
        }

        let one: T = 1_u8.into();
        let mask = one << bit;

        (self.data[chunk] & mask) != T::default()
    }

    pub unsafe fn get_unchecked(&self, x: usize, y: usize, z: usize) -> bool {
        let pos = self.linear_index(x, y, z);
        let chunk = pos / Self::BITS;
        let bit = pos % Self::BITS;
        let one: T = 1_u8.into();
        let mask = one << bit;
        (*self.data.get_unchecked(chunk) & mask) != T::default()
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, value: bool) {
        let pos = self.linear_index(x, y, z);
        let chunk = pos / Self::BITS;
        let bit = pos % Self::BITS;

        if chunk >= self.data.len() {
            return;
        }

        let one: T = 1_u8.into();
        let mask = one << bit;

        if value {
            self.data[chunk] |= mask;
        } else {
            self.data[chunk] &= !mask;
        }
    }

    pub unsafe fn set_unchecked(&mut self, x: usize, y: usize, z: usize, value: bool) {
        let pos = self.linear_index(x, y, z);
        let chunk = pos / Self::BITS;
        let bit = pos % Self::BITS;
        let one: T = 1_u8.into();
        let mask = one << bit;
        let slot = self.data.get_unchecked_mut(chunk);
        if value {
            *slot |= mask;
        } else {
            *slot &= !mask;
        }
    }
}
