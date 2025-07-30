use qvek::vek::Vec3;

use crate::array3d::*;
use std::{
    marker::PhantomData,
    ops::{Index, IndexMut},
};

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

impl<'a, T, U, D: Dim3> Array3DView<'a, T, U, D> {
    pub fn dimensions(&self) -> Vec3<usize> {
        self.array.dimensions()
    }
}

impl<'a, T, U, D: Dim3, I: ToUsize3> Index<I> for Array3DView<'a, T, U, D> {
    type Output = T;
    fn index(&self, index: I) -> &Self::Output {
        let (x, y, z) = index.to_usize3();
        self.array.get(x, y, z)
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

    pub fn dimensions(&self) -> Vec3<usize> {
        self.array.dimensions()
    }
}

impl<'a, T: Clone, U, D: Dim3> Array3DViewMut<'a, T, U, D> {
    pub fn fill(&mut self, value: T) {
        self.array.data.fill(value);
    }
}

impl<'a, T, U, D: Dim3, I: ToUsize3> Index<I> for Array3DViewMut<'a, T, U, D> {
    type Output = T;
    fn index(&self, index: I) -> &Self::Output {
        let (x, y, z) = index.to_usize3();
        self.array.get(x, y, z)
    }
}

impl<'a, T, U, D: Dim3, I: ToUsize3> IndexMut<I> for Array3DViewMut<'a, T, U, D> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        let (x, y, z) = index.to_usize3();
        self.array.get_mut(x, y, z)
    }
}

// extend behaviour of array to support views
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
