use qvek::vek::{Vec2, Vec3, Vec4};
use std::{
    fmt::{self, Debug},
    marker::PhantomData,
    ops::{Index, IndexMut},
    slice::{Iter, IterMut},
};

/// Trait abstracting compile-time and runtime dimensions (proven to be useless perfomance-wise, TODO: remove?).
pub trait Dim3: Copy + Clone + Default {
    /// Size along X axis.
    fn x(&self) -> usize;
    /// Size along Y axis.
    fn y(&self) -> usize;
    /// Size along Z axis.
    fn z(&self) -> usize;

    // vec2 (x() y())
    fn xy(&self) -> Vec2<usize> {
        Vec2 {
            x: self.x(),
            y: self.y(),
        }
    }
    // vec3 (x() y() y())
    fn xyz(&self) -> Vec3<usize> {
        Vec3 {
            x: self.x(),
            y: self.y(),
            z: self.z(),
        }
    }
    // vec2 (y() z())
    fn yz(&self) -> Vec2<usize> {
        Vec2 {
            x: self.y(),
            y: self.z(),
        }
    }

    /// Total number of elements (x * y * z).
    fn total_len(&self) -> usize {
        self.x() * self.y() * self.z()
    }
}

/// Runtime dimensions for non-const sizes.
#[derive(Clone, Copy, Debug, Default)]
pub struct RuntimeDims {
    pub x: usize,
    pub y: usize,
    pub z: usize,
}

impl Dim3 for RuntimeDims {
    fn x(&self) -> usize {
        self.x
    }
    fn y(&self) -> usize {
        self.y
    }
    fn z(&self) -> usize {
        self.z
    }
}

/// Compile-time dimensions using const generics.
#[derive(Clone, Copy, Debug, Default)]
pub struct ConstDims<const X: usize, const Y: usize, const Z: usize>;

impl<const X: usize, const Y: usize, const Z: usize> Dim3 for ConstDims<X, Y, Z> {
    fn x(&self) -> usize {
        X
    }
    fn y(&self) -> usize {
        Y
    }
    fn z(&self) -> usize {
        Z
    }
}

/// Generic 3D array, parameterized by a Dim3, which allows runtime flexibility or template perfomance.
pub struct Array3D<T, D: Dim3> {
    pub data: Box<[T]>,
    pub dims: D,
}

impl<T, D: Dim3> Array3D<T, D> {
    /// Creates a new uninitialized array from a dimension provider and data.
    fn from_boxed(dims: D, data: Box<[T]>) -> Self {
        assert_eq!(dims.total_len(), data.len());
        Self { dims, data }
    }

    /// Computes flat index for (x, y, z).
    // TODO: look into different indexing and move it into generic
    pub fn index_internal(&self, x: usize, y: usize, z: usize) -> usize {
        debug_assert!(x < self.dims.x() && y < self.dims.y() && z < self.dims.z());
        x + y * self.dims.x() + z * self.dims.x() * self.dims.y()
    }

    /// Returns the dimensions as a tuple.
    pub fn dimensions(&self) -> (usize, usize, usize) {
        (self.dims.x(), self.dims.y(), self.dims.z())
    }

    /// Shared reference at (x, y, z).
    pub fn get(&self, x: usize, y: usize, z: usize) -> &T {
        &self.data[self.index_internal(x, y, z)]
    }

    /// Mutable reference at (x, y, z).
    pub fn get_mut(&mut self, x: usize, y: usize, z: usize) -> &mut T {
        &mut self.data[self.index_internal(x, y, z)]
    }

    /// Sets value at (x, y, z).
    pub fn set(&mut self, x: usize, y: usize, z: usize, value: T) {
        let idx = self.index_internal(x, y, z);
        self.data[idx] = value;
    }

    /// Immutable iterator over all elements.
    pub fn iter(&self) -> Iter<'_, T> {
        self.data.iter()
    }

    /// Mutable iterator over all elements.
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.data.iter_mut()
    }

    /// Unchecked shared reference (no bounds checks).
    /// # Safety
    /// slice OOB (out-of-bounds) when array OOB (so UB)
    pub unsafe fn get_unchecked(&self, x: usize, y: usize, z: usize) -> &T {
        self.data.get_unchecked(self.index_internal(x, y, z))
    }

    /// Unchecked mutable reference (no bounds checks)
    /// # Safety
    /// slice OOB (out-of-bounds) when array OOB (so UB)
    pub unsafe fn get_unchecked_mut(&mut self, x: usize, y: usize, z: usize) -> &mut T {
        self.data.get_unchecked_mut(self.index_internal(x, y, z))
    }

    /// Unchecked setter without bounds checks
    /// # Safety
    /// slice OOB (out-of-bounds) when array OOB (so UB)
    pub unsafe fn set_unchecked(&mut self, x: usize, y: usize, z: usize, value: T) {
        *self.get_unchecked_mut(x, y, z) = value;
    }
}

impl<T: Clone, D: Dim3> Array3D<T, D> {
    /// Creates a new array with all elements cloned from `value`.
    pub fn new_filled(dims: D, value: T) -> Self {
        let len = dims.total_len();
        let data = vec![value; len].into_boxed_slice();
        Self::from_boxed(dims, data)
    }

    /// Fills every element with `value`.
    pub fn fill(&mut self, value: T) {
        self.data.fill(value);
    }

    /// Copies data from another array of same dims.
    pub fn copy_data_from(&mut self, other: &Self) {
        debug_assert_eq!(self.dimensions(), other.dimensions());
        self.data.clone_from_slice(&other.data);
    }

    /// Returns a cloned copy of the element at (x, y, z).
    pub fn get_cloned(&self, x: usize, y: usize, z: usize) -> T {
        self.get(x, y, z).clone()
    }
}

impl<T: Clone + Default, D: Dim3> Array3D<T, D> {
    /// Creates array filled with `T::default()`.
    pub fn new_default(dims: D) -> Self {
        Self::new_filled(dims, T::default())
    }
}

impl<T, D: Dim3> Array3D<T, D> {
    /// Creates array via a generator function.
    pub fn from_fn<F: Fn() -> T>(dims: D, generator: F) -> Self {
        let len = dims.total_len();
        let data = (0..len).map(|_| generator()).collect::<Vec<_>>().into_boxed_slice();
        Self::from_boxed(dims, data)
    }
}

/// Trait converting an indexable type into (usize, usize, usize).
pub trait ToUsize3 {
    fn to_usize3(&self) -> (usize, usize, usize);
}

// Blanket impls for tuples
macro_rules! impl_tousize3 {
    ($($t:ty),*) => {
        $(impl ToUsize3 for $t {
            fn to_usize3(&self) -> (usize, usize, usize) { (self.0 as usize, self.1 as usize, self.2 as usize) }
        })*
    };
}
impl_tousize3!(
    (usize, usize, usize),
    (i8, i8, i8),
    (i32, i32, i32),
    (isize, isize, isize)
);
impl ToUsize3 for Vec3<i8> {
    fn to_usize3(&self) -> (usize, usize, usize) {
        (self.x as usize, self.y as usize, self.z as usize)
    }
}
impl ToUsize3 for Vec3<i32> {
    fn to_usize3(&self) -> (usize, usize, usize) {
        (self.x as usize, self.y as usize, self.z as usize)
    }
}
impl ToUsize3 for Vec4<i8> {
    fn to_usize3(&self) -> (usize, usize, usize) {
        (self.x as usize, self.y as usize, self.z as usize)
    }
}
impl ToUsize3 for Vec4<i32> {
    fn to_usize3(&self) -> (usize, usize, usize) {
        (self.x as usize, self.y as usize, self.z as usize)
    }
}

impl<T, D: Dim3, I: ToUsize3> Index<I> for Array3D<T, D> {
    type Output = T;
    fn index(&self, index: I) -> &Self::Output {
        let (x, y, z) = index.to_usize3();
        self.get(x, y, z)
    }
}

impl<T, D: Dim3, I: ToUsize3> IndexMut<I> for Array3D<T, D> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        let (x, y, z) = index.to_usize3();
        self.get_mut(x, y, z)
    }
}

impl<T: Debug, D: Dim3> Debug for Array3D<T, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (x, y, z) = self.dimensions();
        writeln!(f, "Array3D [{x} x {y} x {z}]:")?;
        for z_ in 0..z {
            for y_ in 0..y {
                write!(f, "[ ")?;
                for x_ in 0..x {
                    write!(f, "{:?} ", self[(x_, y_, z_)])?;
                }
                writeln!(f, "]")?;
            }
        }
        Ok(())
    }
}

/// Read-only view converting element type via `Into<U>`.
pub struct Array3DView<'a, T, U, D: Dim3> {
    array: &'a Array3D<T, D>,
    _phantom: PhantomData<U>,
}

impl<'a, T, U, D: Dim3> Array3DView<'a, T, U, D>
where
    T: Into<U> + Clone,
{
    pub fn get(&self, index: impl ToUsize3) -> U {
        let (x, y, z) = index.to_usize3();
        self.array.get(x, y, z).clone().into()
    }
}

/// Mutable view converting element type via `From<U>`.
pub struct Array3DViewMut<'a, T, U, D: Dim3> {
    array: &'a mut Array3D<T, D>,
    _phantom: PhantomData<U>,
}

impl<'a, T, U, D: Dim3> Array3DViewMut<'a, T, U, D> {
    pub fn set(&mut self, index: impl ToUsize3, value: U)
    where
        T: From<U>,
    {
        let (x, y, z) = index.to_usize3();
        self.array.set(x, y, z, T::from(value));
    }

    pub fn get(&self, index: impl ToUsize3) -> U
    where
        T: Into<U> + Clone,
    {
        let (x, y, z) = index.to_usize3();
        self.array.get(x, y, z).clone().into()
    }
}

impl<T, D: Dim3> Array3D<T, D> {
    /// Creates a read-only converting view.
    pub fn as_view<U>(&self) -> Array3DView<'_, T, U, D> {
        Array3DView {
            array: self,
            _phantom: PhantomData,
        }
    }

    /// Creates a mutable converting view.
    pub fn as_view_mut<U>(&mut self) -> Array3DViewMut<'_, T, U, D> {
        Array3DViewMut {
            array: self,
            _phantom: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_and_static() {
        let mut r = Array3D::new_filled(RuntimeDims { x: 2, y: 2, z: 2 }, 1u8);
        r.set(1, 1, 1, 5);
        assert_eq!(*r.get(1, 1, 1), 5);

        let mut s = Array3D::new_filled(ConstDims::<2, 2, 2>, 2u8);
        s[(1, 1, 1)] = 8;
        assert_eq!(s[(1, 1, 1)], 8);
    }
}
