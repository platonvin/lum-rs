use std::fmt::{self, Debug};
use std::iter::IntoIterator;
use std::ops::{Index, IndexMut};
use std::slice::{Iter, IterMut};

use crate::types::{ivec3, ivec4};

/*
 * You should index into it in this order
 *  for z in 0..z_size {
 *  for y in 0..y_size {
 *  for x in 0..x_size {
 * Currently, Rust cannot optimize the order automatically. TODO: MIR-OPT coherence transform 
 */

#[derive(Clone)]
pub struct Array3D<T> {
    pub data: Vec<T>,
    pub x_size: usize,
    pub y_size: usize,
    pub z_size: usize,
}

impl<T: Default + Clone> Array3D<T>
where
    T: Default + Clone,
{
    /// Creates a new 3D array with given dimensions, initialized with `Default` values.
    pub fn new(x_size: usize, y_size: usize, z_size: usize) -> Self {
        Self::new_filled(x_size, y_size, z_size, T::default())
    }
    
    pub fn new_filled(x_size: usize, y_size: usize, z_size: usize, value: T) -> Self {
        assert!(
            x_size > 0 && y_size > 0 && z_size > 0,
            "Dimensions must be greater than zero"
        );
        let data = vec![value; x_size * y_size * z_size];
        Self {
            data,
            x_size,
            y_size,
            z_size,
        }
    }

    /// Returns the index in the flat data array for given (x, y, z) coordinates.
    pub fn index_internal(&self, x: usize, y: usize, z: usize) -> usize {
        debug_assert!(
            x < self.x_size && y < self.y_size && z < self.z_size,
            "Index out of bounds"
        );
        // optimal for
        // for x in 0..x_size {
        //     for y in 0..y_size {
        //         for z in 0..z_size {
        // indexing
        // x * self.y_size * self.z_size + y * self.z_size + z

        // optimal for
        // for z in 0..x_size {
        //     for y in 0..y_size {
        //         for x in 0..z_size {
        // indexing
        x + y * self.x_size + z * self.x_size * self.y_size
    }

    /// Returns the dimensions of the array.
    pub fn dimensions(&self) -> (usize, usize, usize) {
        (self.x_size, self.y_size, self.z_size)
    }

    /// Returns an iterator over the array.
    pub fn iter(&self) -> Iter<'_, T> {
        self.data.iter()
    }

    /// Returns a mutable iterator over the array.
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.data.iter_mut()
    }

    pub fn copy_data_from(&mut self, other: &Array3D<T>) {
        self.data = other.data.clone();
    }
}

impl<T: Default + Clone> Index<(usize, usize, usize)> for Array3D<T> {
    type Output = T;

    fn index(&self, index: (usize, usize, usize)) -> &Self::Output {
        let (x, y, z) = index; // unpack the tuple
        let index_internal = self.index_internal(x, y, z);
        &self.data[index_internal]
    }
}

impl<T: Default + Clone> Index<ivec3> for Array3D<T> {
    type Output = T;

    fn index(&self, index: ivec3) -> &Self::Output {
        let index_internal = self.index_internal(
            index.x as usize,
            index.y as usize, 
            index.z as usize
        ); 
        &self.data[index_internal]
    }
}
impl<T: Default + Clone> Index<ivec4> for Array3D<T> {
    type Output = T;

    fn index(&self, index: ivec4) -> &Self::Output {
        let index_internal = self.index_internal(
            index.x as usize,
            index.y as usize, 
            index.z as usize
        ); 
        &self.data[index_internal]
    }
}

impl<T: Default + Clone> IndexMut<(usize, usize, usize)> for Array3D<T> {
    fn index_mut(&mut self, index: (usize, usize, usize)) -> &mut Self::Output {
        let (x, y, z) = index; // unpack the tuple
        let index_internal = self.index_internal(x, y, z);
        &mut self.data[index_internal]
    }
}

impl<T: Default + Clone> Debug for Array3D<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (x_size, y_size, z_size) = self.dimensions();
        write!(f, "Array3D [{} x {} x {}]:\n", x_size, y_size, z_size)?;
        for x in 0..x_size {
            for y in 0..y_size {
                write!(f, "[ ")?;
                for z in 0..z_size {
                    write!(f, "{:?} ", self[(x, y, z)])?;
                }
                write!(f, "]\n")?;
            }
        }
        Ok(())
    }
}

impl<T: Default + Clone> IntoIterator for Array3D<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

impl<'a, T: Default + Clone> IntoIterator for &'a Array3D<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: Default + Clone> IntoIterator for &'a mut Array3D<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creation_and_access() {
        let mut array = Array3D::new(2, 3, 4);
        assert_eq!(array.dimensions(), (2, 3, 4));

        array[(0, 0, 0)] = 42;
        array[(1, 2, 3)] = 99;

        assert_eq!(array[(0, 0, 0)], 42);
        assert_eq!(array[(1, 2, 3)], 99);
    }

    #[test]
    fn test_iteration() {
        let array: Array3D<i32> = Array3D::new(2, 2, 2);
        assert_eq!(array.iter().count(), 8);
    }

    #[test]
    #[should_panic]
    fn test_out_of_bounds() {
        let array = Array3D::new(2, 2, 2);
        let _: i32 = array[(2, 2, 2)]; // Should panic in debug mode
    }
}
