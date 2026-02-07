use std::fmt;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::ptr;
use std::slice::{Iter, IterMut};

use super::memory_pool::SharedMemoryPool;

/// A Vec-like container that uses pooled memory for efficient allocation.
pub struct PooledVec<T> {
    buffer: Vec<T>,
    pool: SharedMemoryPool<T>,
    recycle_on_drop: bool,
}

impl<T> PooledVec<T> {
    /// Create a new empty PooledVec with a shared memory pool.
    pub fn new(pool: SharedMemoryPool<T>) -> Self {
        Self::with_capacity(16, pool)
    }

    /// Create a new PooledVec with specified initial capacity.
    pub fn with_capacity(capacity: usize, pool: SharedMemoryPool<T>) -> Self {
        let mut vec = pool.acquire(capacity).into_inner();
        vec.clear();
        Self {
            buffer: vec,
            pool,
            recycle_on_drop: true,
        }
    }

    /// Create a PooledVec from existing data.
    pub fn from_slice(data: &[T], pool: SharedMemoryPool<T>) -> Self
    where
        T: Clone,
    {
        let mut vec = Self::with_capacity(data.len(), pool);
        vec.extend_from_slice(data);
        vec
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    pub fn push(&mut self, value: T) {
        if self.buffer.len() == self.buffer.capacity() {
            self.grow();
        }
        self.buffer.push(value);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.buffer.pop()
    }

    pub fn extend_from_slice(&mut self, other: &[T])
    where
        T: Clone,
    {
        self.reserve(other.len());
        self.buffer.extend_from_slice(other);
    }

    pub fn reserve(&mut self, additional: usize) {
        let required_capacity = self.len().saturating_add(additional);
        if required_capacity > self.capacity() {
            self.grow_to(required_capacity);
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn as_slice(&self) -> &[T] {
        self.buffer.as_slice()
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.buffer.as_mut_slice()
    }

    pub fn as_ptr(&self) -> *const T {
        self.buffer.as_ptr()
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.buffer.as_mut_ptr()
    }

    pub fn insert(&mut self, index: usize, element: T) {
        self.buffer.insert(index, element);
    }

    pub fn remove(&mut self, index: usize) -> T {
        self.buffer.remove(index)
    }

    pub fn resize(&mut self, new_len: usize, value: T)
    where
        T: Clone,
    {
        if new_len > self.capacity() {
            self.grow_to(new_len);
        }
        self.buffer.resize(new_len, value);
    }

    pub fn truncate(&mut self, len: usize) {
        self.buffer.truncate(len);
    }

    pub fn iter(&self) -> Iter<'_, T> {
        self.buffer.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.buffer.iter_mut()
    }

    fn grow(&mut self) {
        let new_capacity = (self.capacity() * 2).max(8);
        self.grow_to(new_capacity);
    }

    fn grow_to(&mut self, min_capacity: usize) {
        let target = min_capacity.max(self.capacity().saturating_mul(2)).max(8);
        let mut new_vec = self.pool.acquire(target).into_inner();
        new_vec.clear();
        new_vec.reserve(self.buffer.len());
        new_vec.append(&mut self.buffer);

        let old_vec = std::mem::replace(&mut self.buffer, new_vec);
        self.pool.release_vec(old_vec);
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
        if !self.recycle_on_drop {
            return;
        }

        let mut vec = Vec::new();
        std::mem::swap(&mut vec, &mut self.buffer);
        self.pool.release_vec(vec);
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

impl<T> IntoIterator for PooledVec<T> {
    type Item = T;
    type IntoIter = PooledVecIntoIter<T>;

    fn into_iter(mut self) -> Self::IntoIter {
        let owned = std::mem::take(&mut self.buffer);
        self.recycle_on_drop = false;
        PooledVecIntoIter::new(owned, self.pool.clone())
    }
}

impl<'a, T> IntoIterator for &'a PooledVec<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut PooledVec<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

pub struct PooledVecIntoIter<T> {
    ptr: *mut T,
    len: usize,
    cap: usize,
    next_index: usize,
    pool: Option<SharedMemoryPool<T>>,
}

impl<T> PooledVecIntoIter<T> {
    fn new(buffer: Vec<T>, pool: SharedMemoryPool<T>) -> Self {
        let mut buffer = ManuallyDrop::new(buffer);
        let len = buffer.len();
        let cap = buffer.capacity();
        let ptr = buffer.as_mut_ptr();

        Self {
            ptr,
            len,
            cap,
            next_index: 0,
            pool: Some(pool),
        }
    }
}

impl<T> Iterator for PooledVecIntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_index >= self.len {
            return None;
        }

        // SAFETY: `next_index` is always < len, each element is moved out at most once.
        let item = unsafe { self.ptr.add(self.next_index).read() };
        self.next_index += 1;
        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len.saturating_sub(self.next_index);
        (remaining, Some(remaining))
    }
}

impl<T> ExactSizeIterator for PooledVecIntoIter<T> {}

impl<T> Drop for PooledVecIntoIter<T> {
    fn drop(&mut self) {
        // SAFETY: unread elements are dropped exactly once; consumed elements were moved out by `next`.
        // Reconstructing `Vec` with len=0 preserves allocation for recycling back to the pool.
        unsafe {
            let remaining = self.len.saturating_sub(self.next_index);
            if remaining > 0 {
                ptr::drop_in_place(ptr::slice_from_raw_parts_mut(
                    self.ptr.add(self.next_index),
                    remaining,
                ));
            }

            let recycle = Vec::from_raw_parts(self.ptr, 0, self.cap);

            if let Some(pool) = self.pool.take() {
                pool.release_vec(recycle);
            } else {
                drop(recycle);
            }
        }
    }
}

impl<T> From<Vec<T>> for PooledVec<T> {
    fn from(vec: Vec<T>) -> Self {
        let pool = SharedMemoryPool::new(vec.len().max(16));
        let mut pooled = Self::with_capacity(vec.len(), pool);
        // Keep pool tracking coherent by moving elements into pool-backed storage.
        pooled.buffer.extend(vec);
        pooled
    }
}

impl<T> From<PooledVec<T>> for Vec<T> {
    fn from(mut pooled_vec: PooledVec<T>) -> Self {
        pooled_vec.recycle_on_drop = false;
        let mut out = Vec::new();
        std::mem::swap(&mut out, &mut pooled_vec.buffer);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

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
        let pool = SharedMemoryPool::new(2);
        let mut vec = PooledVec::with_capacity(2, pool);

        vec.push(1);
        vec.push(2);
        let initial_capacity = vec.capacity();

        vec.push(3);
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

    #[test]
    fn test_from_vec_keeps_pool_backed_allocation() {
        let source = vec![1, 2, 3, 4];
        let source_ptr = source.as_ptr();
        let pooled: PooledVec<i32> = source.into();

        assert_eq!(pooled.as_slice(), &[1, 2, 3, 4]);
        assert_ne!(pooled.as_ptr(), source_ptr);
    }

    #[test]
    fn test_into_iter_recycles_allocation_when_fully_consumed() {
        let pool = SharedMemoryPool::new(8);
        let mut vec = PooledVec::with_capacity(8, pool.clone());
        vec.extend_from_slice(&[1, 2, 3, 4]);
        let cap = vec.capacity();

        let collected: Vec<_> = vec.into_iter().collect();
        assert_eq!(collected, vec![1, 2, 3, 4]);

        let stats = pool.statistics();
        assert!(stats.available_count >= 1);
        assert!(stats.total_capacity >= cap);
    }

    #[test]
    fn test_into_iter_recycles_allocation_when_partially_consumed() {
        let pool = SharedMemoryPool::new(8);
        let mut vec = PooledVec::with_capacity(8, pool.clone());
        vec.extend_from_slice(&[10, 20, 30, 40]);
        let cap = vec.capacity();

        let mut iter = vec.into_iter();
        assert_eq!(iter.next(), Some(10));
        assert_eq!(iter.next(), Some(20));
        drop(iter);

        let stats = pool.statistics();
        assert!(stats.available_count >= 1);
        assert!(stats.total_capacity >= cap);
    }

    #[test]
    fn test_into_iter_partial_drop_releases_unread_elements_once() {
        struct DropCounter {
            drops: Arc<AtomicUsize>,
        }

        impl Drop for DropCounter {
            fn drop(&mut self) {
                self.drops.fetch_add(1, Ordering::SeqCst);
            }
        }

        let pool = SharedMemoryPool::new(8);
        let drops = Arc::new(AtomicUsize::new(0));
        let mut vec = PooledVec::with_capacity(8, pool.clone());
        for _ in 0..4 {
            vec.push(DropCounter {
                drops: Arc::clone(&drops),
            });
        }

        let mut iter = vec.into_iter();
        let first = iter.next().expect("first item should exist");
        drop(first);
        assert_eq!(drops.load(Ordering::SeqCst), 1);

        drop(iter);
        assert_eq!(drops.load(Ordering::SeqCst), 4);

        let stats = pool.statistics();
        assert!(stats.available_count >= 1);
    }
}
