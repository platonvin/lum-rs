use std::collections::VecDeque;

use crate::assert_unreachable;

#[derive(Default)]
pub struct Arena<T> {
    storage: Vec<Option<T>>, // Stores elements, with `None` representing free slots
    // sorted deque of indices
    // front is smallest index (what we need to allocate to if we want memory coherence)
    free_indices: VecDeque<usize>, // Keeps track of available slots
}

impl<T> Arena<T> {
    /// Creates a new arena with a given initial size.
    pub fn new(initial_size: usize) -> Self {
        let free_indices = (0..initial_size).collect();

        let storage = (0..initial_size).map(|_| None).collect();

        Self {
            storage,
            free_indices,
        }
    }

    /// Allocates a new object in the arena, returning a handle
    pub fn allocate(&mut self, value: T) -> Option<usize> {
        if let Some(index) = self.free_indices.pop_front() {
            // Always get the smallest index
            self.storage[index] = Some(value);
            Some(index)
        } else {
            if self.storage.is_empty() {
                self.grow(1); // Grow the arena to 1 element
            } else {
                self.grow(self.storage.len() * 2); // Double the size of the arena
            }
            if let Some(index) = self.free_indices.pop_front() {
                // Always get the smallest index
                self.storage[index] = Some(value);
                Some(index)
            } else {
                assert_unreachable!()
            }
        }
    }

    /// Retrieves a reference to an allocated object by index.
    pub fn get(&self, index: usize) -> Option<&T> {
        self.storage.get(index)?.as_ref()
    }

    /// Retrieves a mutable reference to an allocated object by index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.storage.get_mut(index)?.as_mut()
    }

    pub fn take(&mut self, index: usize) -> Option<T> {
        let temp = self.storage[index].take()?;
        self.free(index);
        Some(temp)
    }

    /// Frees the object at the given index, making the slot available again.
    pub fn free(&mut self, index: usize) {
        if index < self.storage.len() && self.storage[index].is_some() {
            self.storage[index] = None;

            // Insert in sorted order to maintain contiguous allocation (O(n) worst, O(1) amortized)
            match self.free_indices.binary_search(&index) {
                Ok(_) => {
                    assert_unreachable!(); // Should never happen
                } // Should never happen
                Err(pos) => self.free_indices.insert(pos, index), // Maintain sorted order
            }
        }
    }

    /// Clears the arena, making all slots available.
    pub fn clear(&mut self) {
        self.storage.iter_mut().for_each(|slot| *slot = None);
        self.free_indices.clear();
        self.free_indices.extend(0..self.storage.len()); // Reset free indices in order
    }

    /// Grows the arena to a new size, keeping existing objects intact.
    pub fn grow(&mut self, new_size: usize) {
        let old_size = self.storage.len();

        if new_size > old_size {
            self.storage.resize_with(new_size, || None);
            self.free_indices.extend(old_size..new_size); // Add new indices in order
        }
    }

    /// Returns the total size of the arena.
    pub fn total_size(&self) -> usize {
        self.storage.len()
    }
}
