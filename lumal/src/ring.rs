// vector that has index that moves by one untile reaches the end and then wraps
// primarly used for CPU-GPU resources
 
use Vec as vector;

pub struct Ring<T> {
    pub(crate) data: vector<T>,
    pub(crate) index: usize,
}

impl<T> Ring<T> {
    /// Creates a new `Ring` with a given size and initial value for all elements.
    pub fn new(size: usize, initial_value: T) -> Self
    where
        T: Clone,
    {
        Self {
            data: vec![initial_value; size],
            index: 0,
        }
    }

    /// Returns the current element in the ring.
    pub fn current(&self) -> &T {
        &self.data[self.index]
    }

    /// Moves to the next element in the ring (circularly).
    pub fn move_next(&mut self) {
        self.index = (self.index + 1) % self.data.len();
    }

    /// Mutably access the current element in the ring.
    pub fn current_mut(&mut self) -> &mut T {
        &mut self.data[self.index]
    }

    /// Returns the length of the ring.
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

/// Iterator for `Ring`
pub struct RingIterator<'a, T> {
    ring: &'a Ring<T>,
    position: usize,
}

impl<'a, T> IntoIterator for &'a Ring<T> {
    type Item = &'a T;
    type IntoIter = RingIterator<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        RingIterator {
            ring: self,
            position: 0,
        }
    }
}

impl<'a, T> Iterator for RingIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.ring.len() {
            let item = &self.ring.data[self.position];
            self.position += 1;
            Some(item)
        } else {
            None
        }
    }
}
