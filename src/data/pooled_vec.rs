use std::fmt;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::slice::{Iter, IterMut};
use std::sync::{Arc, Mutex};

use super::memory_pool::{MemoryPool, PooledBuffer, SharedMemoryPool};

/// A Vec-like container that uses pooled memory for efficient allocation
/// Provides the same interface as Vec<T> but with memory pool backing
pub struct PooledVec<T> {
    /// The underlying pooled buffer
    buffer: PooledBuffer<T>,
    /// Current length (number of initialized elements)
    len: usize,
    /// Shared reference to the memory pool for growth
    pool: SharedMemoryPool<T>,
}

impl<T> PooledVec<T> {
    /// Create a new empty PooledVec with a shared memory pool
    pub fn new(pool: SharedMemoryPool<T>) -> Self {
        let buffer = pool.acquire(16); // Start with reasonable capacity
        Self {
            buffer,
            len: 0,
            pool,
        }
    }

    /// Create a new PooledVec with specified initial capacity
    pub fn with_capacity(capacity: usize, pool: SharedMemoryPool<T>) -> Self {
        let buffer = pool.acquire(capacity);
        Self {
            buffer,
            len: 0,
            pool,
        }
    }

    /// Create a PooledVec from existing data
    pub fn from_slice(data: &[T], pool: SharedMemoryPool<T>) -> Self
    where
        T: Clone,
    {
        let mut vec = Self::with_capacity(data.len(), pool);
        vec.extend_from_slice(data);
        vec
    }

    /// Get the number of elements in the vector
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the vector is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the capacity of the vector
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// Add an element to the end of the vector
    pub fn push(&mut self, value: T) {
        if self.len >= self.capacity() {
            self.grow();
        }

        unsafe {
            std::ptr::write(self.buffer.as_mut_ptr().add(self.len), value);
        }
        self.len += 1;
    }

    /// Remove and return the last element
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            Some(unsafe { std::ptr::read(self.buffer.as_ptr().add(self.len)) })
        }
    }

    /// Extend the vector with elements from a slice
    pub fn extend_from_slice(&mut self, other: &[T])
    where
        T: Clone,
    {
        self.reserve(other.len());
        for item in other {
            self.push(item.clone());
        }
    }

    /// Reserve capacity for at least additional elements
    pub fn reserve(&mut self, additional: usize) {
        let required_capacity = self.len + additional;
        if required_capacity > self.capacity() {
            self.grow_to(required_capacity);
        }
    }

    /// Clear all elements from the vector
    pub fn clear(&mut self) {
        // Drop all elements in reverse order
        while let Some(_) = self.pop() {}
    }

    /// Get a slice of all elements
    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.buffer.as_ptr(), self.len) }
    }

    /// Get a mutable slice of all elements  
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.buffer.as_mut_ptr(), self.len) }
    }

    /// Get a raw pointer to the underlying data
    pub fn as_ptr(&self) -> *const T {
        self.buffer.as_ptr()
    }

    /// Get a mutable raw pointer to the underlying data
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.buffer.as_mut_ptr()
    }

    /// Insert an element at the specified index
    pub fn insert(&mut self, index: usize, element: T) {
        assert!(index <= self.len, "Index out of bounds");

        if self.len >= self.capacity() {
            self.grow();
        }

        unsafe {
            // Shift elements to the right
            let ptr = self.buffer.as_mut_ptr().add(index);
            std::ptr::copy(ptr, ptr.add(1), self.len - index);
            // Insert the new element
            std::ptr::write(ptr, element);
        }
        self.len += 1;
    }

    /// Remove and return the element at the specified index
    pub fn remove(&mut self, index: usize) -> T {
        assert!(index < self.len, "Index out of bounds");

        unsafe {
            let ptr = self.buffer.as_mut_ptr().add(index);
            let element = std::ptr::read(ptr);

            // Shift elements to the left
            std::ptr::copy(ptr.add(1), ptr, self.len - index - 1);

            self.len -= 1;
            element
        }
    }

    /// Resize the vector to the specified length
    pub fn resize(&mut self, new_len: usize, value: T)
    where
        T: Clone,
    {
        if new_len > self.len {
            self.reserve(new_len - self.len);
            while self.len < new_len {
                self.push(value.clone());
            }
        } else {
            self.truncate(new_len);
        }
    }

    /// Truncate the vector to the specified length
    pub fn truncate(&mut self, len: usize) {
        if len < self.len {
            // Drop elements beyond the new length
            for i in len..self.len {
                unsafe {
                    std::ptr::drop_in_place(self.buffer.as_mut_ptr().add(i));
                }
            }
            self.len = len;
        }
    }

    /// Get an iterator over the elements
    pub fn iter(&self) -> Iter<T> {
        self.as_slice().iter()
    }

    /// Get a mutable iterator over the elements
    pub fn iter_mut(&mut self) -> IterMut<T> {
        self.as_mut_slice().iter_mut()
    }

    /// Grow the buffer to accommodate more elements
    fn grow(&mut self) {
        let new_capacity = (self.capacity() * 2).max(8);
        self.grow_to(new_capacity);
    }

    /// Grow the buffer to at least the specified capacity
    fn grow_to(&mut self, min_capacity: usize) {
        let new_capacity = min_capacity.max(self.capacity() * 2);

        // Get a new larger buffer from the pool
        let mut new_buffer = self.pool.acquire(new_capacity);

        // Copy existing elements to the new buffer
        unsafe {
            std::ptr::copy_nonoverlapping(self.buffer.as_ptr(), new_buffer.as_mut_ptr(), self.len);
        }

        // Replace the old buffer with the new one
        self.buffer = new_buffer;
    }
}

impl<T> Deref for PooledVec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T> DerefMut for PooledVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T> Index<usize> for PooledVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl<T> IndexMut<usize> for PooledVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.as_mut_slice()[index]
    }
}

impl<T> Drop for PooledVec<T> {
    fn drop(&mut self) {
        self.clear(); // Drop all elements
        // Buffer will be returned to pool automatically
    }
}

impl<T: Clone> Clone for PooledVec<T> {
    fn clone(&self) -> Self {
        Self::from_slice(self.as_slice(), self.pool.clone())
    }
}

impl<T: fmt::Debug> fmt::Debug for PooledVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_slice().fmt(f)
    }
}

impl<T: PartialEq> PartialEq for PooledVec<T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: Eq> Eq for PooledVec<T> {}

// Implement IntoIterator for owned PooledVec
impl<T> IntoIterator for PooledVec<T> {
    type Item = T;
    type IntoIter = PooledVecIntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        PooledVecIntoIter {
            vec: self,
            index: 0,
        }
    }
}

// Implement IntoIterator for borrowed PooledVec
impl<'a, T> IntoIterator for &'a PooledVec<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// Implement IntoIterator for mutably borrowed PooledVec
impl<'a, T> IntoIterator for &'a mut PooledVec<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// Iterator that owns the PooledVec and yields owned elements
pub struct PooledVecIntoIter<T> {
    vec: PooledVec<T>,
    index: usize,
}

impl<T> Iterator for PooledVecIntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.vec.len() {
            let item = unsafe { std::ptr::read(self.vec.buffer.as_ptr().add(self.index)) };
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.vec.len() - self.index;
        (remaining, Some(remaining))
    }
}

impl<T> ExactSizeIterator for PooledVecIntoIter<T> {}

impl<T> Drop for PooledVecIntoIter<T> {
    fn drop(&mut self) {
        // Drop any remaining elements
        while self.next().is_some() {}
    }
}

// Conversion from Vec<T>
impl<T> From<Vec<T>> for PooledVec<T> {
    fn from(vec: Vec<T>) -> Self {
        // This creates a new pool for the conversion
        let pool = SharedMemoryPool::new(vec.len().max(16));
        let mut pooled = Self::with_capacity(vec.len(), pool);

        // Move elements from Vec to PooledVec
        for item in vec {
            pooled.push(item);
        }
        pooled
    }
}

// Conversion to Vec<T>
impl<T: Clone> From<PooledVec<T>> for Vec<T> {
    fn from(pooled_vec: PooledVec<T>) -> Self {
        pooled_vec.as_slice().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pooled_vec_basic_operations() {
        let pool = SharedMemoryPool::new(100);
        let mut vec = PooledVec::new(pool);

        assert_eq!(vec.len(), 0);
        assert!(vec.is_empty());

        vec.push(1);
        vec.push(2);
        vec.push(3);

        assert_eq!(vec.len(), 3);
        assert_eq!(vec[0], 1);
        assert_eq!(vec[1], 2);
        assert_eq!(vec[2], 3);

        assert_eq!(vec.pop(), Some(3));
        assert_eq!(vec.len(), 2);
    }

    #[test]
    fn test_pooled_vec_growth() {
        let pool = SharedMemoryPool::new(2); // Pool with small chunks
        let mut vec = PooledVec::with_capacity(2, pool);

        vec.push(1);
        vec.push(2);
        let initial_capacity = vec.capacity();

        vec.push(3); // Should trigger growth
        assert!(vec.capacity() >= initial_capacity);
        assert_eq!(vec.len(), 3);
        assert_eq!(vec[2], 3);
    }

    #[test]
    fn test_pooled_vec_extend() {
        let pool = SharedMemoryPool::new(100);
        let mut vec = PooledVec::new(pool);

        vec.extend_from_slice(&[1, 2, 3, 4, 5]);
        assert_eq!(vec.len(), 5);
        assert_eq!(vec.as_slice(), &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_pooled_vec_iteration() {
        let pool = SharedMemoryPool::new(100);
        let mut vec = PooledVec::new(pool);
        vec.extend_from_slice(&[1, 2, 3, 4, 5]);

        let sum: i32 = vec.iter().sum();
        assert_eq!(sum, 15);

        for item in &mut vec {
            *item *= 2;
        }
        assert_eq!(vec.as_slice(), &[2, 4, 6, 8, 10]);
    }

    #[test]
    fn test_pooled_vec_insert_remove() {
        let pool = SharedMemoryPool::new(100);
        let mut vec = PooledVec::new(pool);
        vec.extend_from_slice(&[1, 2, 4, 5]);

        vec.insert(2, 3);
        assert_eq!(vec.as_slice(), &[1, 2, 3, 4, 5]);

        let removed = vec.remove(2);
        assert_eq!(removed, 3);
        assert_eq!(vec.as_slice(), &[1, 2, 4, 5]);
    }

    #[test]
    fn test_pooled_vec_resize() {
        let pool = SharedMemoryPool::new(100);
        let mut vec = PooledVec::new(pool);
        vec.extend_from_slice(&[1, 2, 3]);

        vec.resize(5, 99);
        assert_eq!(vec.as_slice(), &[1, 2, 3, 99, 99]);

        vec.resize(2, 0);
        assert_eq!(vec.as_slice(), &[1, 2]);
    }

    #[test]
    fn test_pooled_vec_conversion() {
        let regular_vec = vec![1, 2, 3, 4, 5];
        let pooled_vec: PooledVec<i32> = regular_vec.into();

        assert_eq!(pooled_vec.as_slice(), &[1, 2, 3, 4, 5]);

        let back_to_vec: Vec<i32> = pooled_vec.into();
        assert_eq!(back_to_vec, vec![1, 2, 3, 4, 5]);
    }
}
