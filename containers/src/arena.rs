use std::collections::VecDeque;

#[derive(Default)]
pub struct Arena<T> {
    // Actual elements, with `None` representing free slots.
    storage: Vec<Option<T>>,
    /// Sorted deque of indices.
    free_indices: VecDeque<usize>, // Keeps track of available slots
}

impl<T> Arena<T> {
    /// Creates an arena with a given initial size.
    pub fn new(initial_size: usize) -> Self {
        let free_indices = (0..initial_size).collect();
        let storage = (0..initial_size).map(|_| None).collect();
        Self {
            storage,
            free_indices,
        }
    }

    /// Allocates a new object in the arena, returning a handle.
    pub fn allocate(&mut self, value: T) -> Option<usize> {
        if let Some(index) = self.free_indices.pop_front() {
            self.storage[index] = Some(value);
            Some(index)
        } else {
            let grow_to = if self.storage.is_empty() {
                1
            } else {
                self.storage.len() * 2
            };
            self.grow(grow_to);
            if let Some(index) = self.free_indices.pop_front() {
                self.storage[index] = Some(value);
                Some(index)
            } else {
                unreachable!()
            }
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.storage.get(index)?.as_ref()
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.storage.get_mut(index)?.as_mut()
    }

    pub fn take(&mut self, index: usize) -> Option<T> {
        let temp = self.storage[index].take()?;
        self.free(index);
        Some(temp)
    }

    pub fn free(&mut self, index: usize) {
        if index < self.storage.len() && self.storage[index].is_some() {
            self.storage[index] = None;
            match self.free_indices.binary_search(&index) {
                Ok(_) => unreachable!(),
                Err(pos) => self.free_indices.insert(pos, index),
            }
        }
    }

    pub fn clear(&mut self) {
        self.storage.iter_mut().for_each(|slot| *slot = None);
        self.free_indices.clear();
        self.free_indices.extend(0..self.storage.len());
    }

    pub fn grow(&mut self, new_size: usize) {
        let old_size = self.storage.len();
        if new_size > old_size {
            self.storage.resize_with(new_size, || None);
            self.free_indices.extend(old_size..new_size);
        }
    }

    pub fn total_size(&self) -> usize {
        self.storage.len()
    }
}

/// Immutable iterator over (`index`, `&T`) of live entries in the arena.
pub struct Iter<'a, T> {
    inner: std::iter::Enumerate<std::slice::Iter<'a, Option<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((idx, opt)) = self.inner.next() {
            if let Some(val) = opt.as_ref() {
                return Some((idx, val));
            }
        }
        None
    }
}

/// Mutable iterator over (`index`, `&mut T`) of live entries in the arena.
pub struct IterMut<'a, T> {
    inner: std::iter::Enumerate<std::slice::IterMut<'a, Option<T>>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (usize, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((idx, opt)) = self.inner.next() {
            if let Some(val) = opt.as_mut() {
                return Some((idx, val));
            }
        }
        None
    }
}

impl<'a, T> IntoIterator for &'a Arena<T> {
    type Item = (usize, &'a T);
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            inner: self.storage.iter().enumerate(),
        }
    }
}

impl<'a, T> IntoIterator for &'a mut Arena<T> {
    type Item = (usize, &'a mut T);
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        IterMut {
            inner: self.storage.iter_mut().enumerate(),
        }
    }
}

/// Consuming iterator over `T` of live entries in the arena (order by index).
pub struct IntoIter<T> {
    inner: std::vec::IntoIter<Option<T>>,
    idx: usize,
}

impl<T> Iterator for IntoIter<T> {
    type Item = (usize, T);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(opt) = self.inner.next() {
            let current = self.idx;
            self.idx += 1;
            if let Some(val) = opt {
                return Some((current, val));
            }
        }
        None
    }
}

impl<T> IntoIterator for Arena<T> {
    type Item = (usize, T);
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            inner: self.storage.into_iter(),
            idx: 0,
        }
    }
}
