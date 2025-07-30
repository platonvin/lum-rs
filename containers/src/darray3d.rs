use qvek::vek::Vec3;

use crate::{array3d::RuntimeDims, Array3D};
use std::fmt::{self, Debug};
use std::ops::{Index, IndexMut};

/// Dynamic 3D array wrapper specialized for runtime dimensions.
pub struct DArray3D<T> {
    pub arr: Array3D<T, RuntimeDims>,
}

impl<T> DArray3D<T> {
    /// Create an array with all elements cloned from `value`.
    pub fn new_filled(x: usize, y: usize, z: usize, value: T) -> Self
    where
        T: Clone,
    {
        let dims = RuntimeDims { x, y, z };
        let arr = Array3D::new_filled(dims, value);
        Self { arr }
    }

    /// Create an array filled with `T::default()`.
    pub fn new_default(x: usize, y: usize, z: usize) -> Self
    where
        T: Clone + Default,
    {
        let dims = RuntimeDims { x, y, z };
        let arr = Array3D::new_default(dims);
        Self { arr }
    }

    /// Create an array with generator function.
    pub fn from_fn<F: Fn() -> T>(x: usize, y: usize, z: usize, generator: F) -> Self {
        let dims = RuntimeDims { x, y, z };
        let arr = Array3D::from_fn(dims, generator);
        Self { arr }
    }

    /// Returns the dimensions `(x, y, z)`.
    pub fn dimensions(&self) -> Vec3<usize> {
        self.arr.dimensions()
    }

    /// Shared reference at `(x, y, z)`.
    pub fn get(&self, x: usize, y: usize, z: usize) -> &T {
        self.arr.get(x, y, z)
    }

    /// Mutable reference at `(x, y, z)`.
    pub fn get_mut(&mut self, x: usize, y: usize, z: usize) -> &mut T {
        self.arr.get_mut(x, y, z)
    }

    /// Sets value at `(x, y, z)`.
    pub fn set(&mut self, x: usize, y: usize, z: usize, value: T) {
        self.arr.set(x, y, z, value);
    }

    /// Immutable iterator over all elements.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.arr.iter()
    }

    /// Mutable iterator over all elements.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.arr.iter_mut()
    }

    /// Unchecked shared reference (no bounds checks).
    /// # Safety
    /// OOB => UB
    pub unsafe fn get_unchecked(&self, x: usize, y: usize, z: usize) -> &T {
        unsafe { self.arr.get_unchecked(x, y, z) }
    }

    /// Unchecked mutable reference (no bounds checks).
    /// # Safety
    /// OOB => UB
    pub unsafe fn get_unchecked_mut(&mut self, x: usize, y: usize, z: usize) -> &mut T {
        unsafe { self.arr.get_unchecked_mut(x, y, z) }
    }

    /// Reset all elements to `value`.
    pub fn fill(&mut self, value: T)
    where
        T: Clone,
    {
        self.arr.fill(value)
    }

    /// Copy data from another array of same dimensions.
    pub fn copy_from(&mut self, other: &Self)
    where
        T: Clone,
    {
        self.arr.copy_data_from(&other.arr);
    }
}

impl<T, I> Index<I> for DArray3D<T>
where
    I: crate::array3d::ToUsize3,
{
    type Output = T;
    fn index(&self, index: I) -> &Self::Output {
        let (x, y, z) = index.to_usize3();
        self.get(x, y, z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_usage() {
        let mut a = DArray3D::new_filled(2, 2, 2, 0u8);
        assert_eq!(a[(1, 1, 1)], 0);
        a.set(1, 1, 1, 7);
        assert_eq!(a.get(1, 1, 1), &7);

        for v in a.iter_mut() {
            *v = 1;
        }
        assert!(a.iter().all(|&v| v == 1));
    }
}
