use std::ops::{BitAnd, BitAndAssign, BitOrAssign, Not, Shl, Shr};

#[derive(Debug)]
pub struct BitArray3d<T> {
    x_size: usize,
    y_size: usize,
    z_size: usize,
    data: Vec<T>,
}

impl<T> BitArray3d<T>
where
    T: Default
        + Copy
        + BitAnd<Output = T>
        + BitOrAssign
        + BitAndAssign
        + Shl<usize, Output = T>
        + Shr<usize, Output = T>
        + Not<Output = T>
        + PartialEq
        + From<u8>, // Ensure we can construct values safely
{
    const BITS: usize = std::mem::size_of::<T>() * 8;

    pub fn new(x_size: usize, y_size: usize, z_size: usize) -> Self {
        let total_bits = x_size
            .checked_mul(y_size)
            .and_then(|xy| xy.checked_mul(z_size))
            .expect("BitArray3d dimensions cause overflow");
        let total_chunks = total_bits.div_ceil(Self::BITS);

        BitArray3d {
            x_size,
            y_size,
            z_size,
            data: vec![T::default(); total_chunks],
        }
    }

    pub fn new_filled(x_size: usize, y_size: usize, z_size: usize, value: bool) -> Self {
        let total_bits = x_size
            .checked_mul(y_size)
            .and_then(|xy| xy.checked_mul(z_size))
            .expect("BitArray3d dimensions cause overflow");
        let total_chunks = total_bits.div_ceil(Self::BITS);

        BitArray3d {
            x_size,
            y_size,
            z_size,
            data: vec![if value { !T::default() } else { T::default() }; total_chunks],
        }
    }

    pub fn fill(&mut self, value: bool) {
        let fill_value = if value { !T::default() } else { T::default() };
        self.data.fill(fill_value);
    }

    pub fn linear_index(&self, x: usize, y: usize, z: usize) -> usize {
        debug_assert!(
            x < self.x_size && y < self.y_size && z < self.z_size,
            "Index out of bounds"
        );
        x + y * self.x_size + z * self.x_size * self.y_size
        // x * self.y_size * self.z_size + y * self.z_size + z
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
}
