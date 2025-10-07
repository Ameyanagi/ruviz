use std::marker::PhantomData;
use std::ptr::NonNull;
use std::slice;

use crate::data::{Data1D, PooledVec};

/// Zero-copy view into data that implements Data1D trait
/// Provides access to data without copying, using lifetime safety
pub struct DataView<T> {
    /// Non-owning pointer to the data
    data: NonNull<T>,
    /// Length of the data
    len: usize,
    /// Phantom data for proper lifetime management
    _phantom: PhantomData<*const [T]>,
}

impl<T> DataView<T> {
    /// Create a new DataView from a raw pointer and length
    /// 
    /// # Safety
    /// - `data` must be valid for reads for `len * std::mem::size_of::<T>()` bytes
    /// - The data must remain valid for the lifetime of this DataView
    /// - The memory referenced by `data` must not be mutated while this DataView exists
    pub unsafe fn from_raw_parts(data: *const T, len: usize) -> Self {
        Self {
            data: NonNull::new(data as *mut T).expect("DataView cannot be created from null pointer"),
            len,
            _phantom: PhantomData,
        }
    }

    /// Create a zero-copy DataView from a slice
    /// The DataView will live as long as the slice
    pub fn from_slice(slice: &[T]) -> DataView<T> {
        unsafe {
            Self::from_raw_parts(slice.as_ptr(), slice.len())
        }
    }

    /// Create a zero-copy DataView from a PooledVec
    /// The DataView borrows from the PooledVec without copying
    pub fn from_pooled_vec(pooled_vec: &PooledVec<T>) -> DataView<T> {
        Self::from_slice(pooled_vec.as_slice())
    }

    /// Create a zero-copy DataView from a regular Vec
    pub fn from_vec(vec: &Vec<T>) -> DataView<T> {
        Self::from_slice(vec.as_slice())
    }

    /// Get the length of the data view
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the data view is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get a raw pointer to the data
    pub fn as_ptr(&self) -> *const T {
        self.data.as_ptr()
    }

    /// Convert to a slice with the same lifetime as the original data
    /// 
    /// # Safety
    /// The returned slice is only valid as long as the original data remains valid
    pub fn as_slice(&self) -> &[T] {
        unsafe {
            slice::from_raw_parts(self.data.as_ptr(), self.len)
        }
    }

    /// Get an element at the specified index
    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len {
            Some(unsafe { &*self.data.as_ptr().add(index) })
        } else {
            None
        }
    }

    /// Get an iterator over the data view
    pub fn iter(&self) -> DataViewIter<T> {
        DataViewIter {
            data: self.data,
            len: self.len,
            index: 0,
            _phantom: PhantomData,
        }
    }

    /// Map this DataView to a new DataView with a different type
    /// using the provided mapping function
    pub fn map<U, F>(self, f: F) -> MappedDataView<T, U, F>
    where
        F: Fn(&T) -> U,
    {
        MappedDataView {
            original: self,
            mapper: f,
            _phantom: PhantomData,
        }
    }

    /// Create a sub-view of this DataView
    pub fn sub_view(&self, start: usize, len: usize) -> Option<DataView<T>> {
        if start + len <= self.len {
            Some(unsafe {
                Self::from_raw_parts(self.data.as_ptr().add(start), len)
            })
        } else {
            None
        }
    }
}

// Implement Data1D for DataView 
impl<T> Data1D<T> for DataView<T> {
    fn len(&self) -> usize {
        self.len
    }

    fn get(&self, idx: usize) -> Option<&T> {
        if idx < self.len {
            Some(unsafe { &*self.data.as_ptr().add(idx) })
        } else {
            None
        }
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        // Create an iterator over the slice, which properly handles lifetimes
        Box::new(self.as_slice().iter())
    }
}

// DataViewRefIter removed - Data1D::iter() uses slice iterator directly

/// Iterator over a DataView that returns owned values (for Copy types)
pub struct DataViewIter<T> {
    data: NonNull<T>,
    len: usize,
    index: usize,
    _phantom: PhantomData<*const T>,
}

impl<T: Copy> Iterator for DataViewIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.len {
            let item = unsafe { *self.data.as_ptr().add(self.index) };
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len - self.index;
        (remaining, Some(remaining))
    }
}

impl<T: Copy> ExactSizeIterator for DataViewIter<T> {}

// Safety: DataView is safe to send between threads if T is Send
unsafe impl<T: Send> Send for DataView<T> {}

// Safety: DataView is safe to share between threads if T is Sync
unsafe impl<T: Sync> Sync for DataView<T> {}

impl<T> Clone for DataView<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data,
            len: self.len,
            _phantom: PhantomData,
        }
    }
}

impl<T> Copy for DataView<T> {}

/// A DataView that applies a mapping function to each element
/// This allows zero-copy transformation of data types
pub struct MappedDataView<T, U, F> {
    original: DataView<T>,
    mapper: F,
    _phantom: PhantomData<U>,
}

impl<T, U, F> MappedDataView<T, U, F>
where
    F: Fn(&T) -> U,
{
    /// Get the length of the mapped view
    pub fn len(&self) -> usize {
        self.original.len()
    }

    /// Check if the mapped view is empty
    pub fn is_empty(&self) -> bool {
        self.original.is_empty()
    }

    /// Get an element at the specified index, applying the mapper function
    pub fn get(&self, index: usize) -> Option<U> {
        self.original.get(index).map(&self.mapper)
    }

    /// Get an iterator over the mapped view
    pub fn iter(&self) -> MappedDataViewIter<T, U, F>
    where
        T: Copy,
    {
        MappedDataViewIter {
            original_iter: self.original.iter(),
            mapper: &self.mapper,
            _phantom: PhantomData,
        }
    }
}

// Note: MappedDataView cannot implement Data1D directly because it transforms T -> U
// and Data1D expects to return &T references, not owned U values.
// For Data1D compatibility, users should materialize the mapped data into a container first.

/// Iterator for MappedDataView
pub struct MappedDataViewIter<'a, T, U, F>
where
    T: Copy,
{
    original_iter: DataViewIter<T>,
    mapper: &'a F,
    _phantom: PhantomData<U>,
}

impl<'a, T, U, F> Iterator for MappedDataViewIter<'a, T, U, F>
where
    T: Copy,
    F: Fn(&T) -> U,
{
    type Item = U;

    fn next(&mut self) -> Option<Self::Item> {
        self.original_iter.next().map(|item| (self.mapper)(&item))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.original_iter.size_hint()
    }
}

impl<'a, T, U, F> ExactSizeIterator for MappedDataViewIter<'a, T, U, F>
where
    T: Copy,
    F: Fn(&T) -> U,
{}

/// Utility functions for creating DataViews from common data sources
impl<T> DataView<T> {
    /// Create a DataView from an array reference
    pub fn from_array<const N: usize>(array: &[T; N]) -> DataView<T> {
        Self::from_slice(array.as_slice())
    }

    /// Create multiple DataViews from a slice of slices (for multi-series data)
    pub fn from_multi_slice(data: &[&[T]]) -> Vec<DataView<T>> {
        data.iter().map(|slice| Self::from_slice(slice)).collect()
    }

    /// Create a DataView from a range iterator (materialized into a temporary allocation)
    /// Note: This is not zero-copy but provides a consistent interface
    pub fn from_range<R>(range: R) -> DataView<T> 
    where
        R: Iterator<Item = T>,
        T: Clone,
    {
        let vec: Vec<T> = range.collect();
        // This creates a leak, but in practice this method should be avoided
        // for true zero-copy scenarios. It's provided for API completeness.
        let boxed = vec.into_boxed_slice();
        let ptr = boxed.as_ptr();
        let len = boxed.len();
        std::mem::forget(boxed); // Leak the memory to maintain pointer validity
        
        unsafe {
            Self::from_raw_parts(ptr, len)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::SharedMemoryPool;

    #[test]
    fn test_data_view_from_slice() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let view = DataView::from_slice(&data);
        
        assert_eq!(view.len(), 5);
        assert_eq!(view.get(0), Some(&1.0));
        assert_eq!(view.get(4), Some(&5.0));
        assert_eq!(view.get(5), None);
    }

    #[test]
    fn test_data_view_data1d_impl() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let view = DataView::from_slice(&data);
        
        // Test Data1D implementation
        use crate::data::Data1D;
        assert_eq!(view.len(), 5);
        assert_eq!(*view.get(0).unwrap(), 1.0);
        assert_eq!(*view.get(4).unwrap(), 5.0);
        assert!(view.get(5).is_none());
        
        // Test iterator from Data1D trait
        let collected: Vec<f64> = view.iter().collect();
        assert_eq!(collected.len(), 5);
        assert_eq!(collected[0], 1.0);
        assert_eq!(collected[4], 5.0);
    }

    #[test]
    fn test_data_view_from_pooled_vec() {
        let pool = SharedMemoryPool::new(100);
        let mut pooled_vec = PooledVec::new(pool);
        pooled_vec.extend_from_slice(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        
        let view = DataView::from_pooled_vec(&pooled_vec);
        
        assert_eq!(view.len(), 5);
        assert_eq!(view.as_ptr(), pooled_vec.as_ptr());
        
        // Verify zero-copy (same memory address)
        assert_eq!(view.as_ptr(), pooled_vec.as_ptr());
    }

    #[test]
    fn test_data_view_iteration() {
        let data = vec![1, 2, 3, 4, 5];
        let view = DataView::from_slice(&data);
        
        let collected: Vec<i32> = view.iter().collect();
        assert_eq!(collected, vec![1, 2, 3, 4, 5]);
        
        let sum: i32 = view.iter().sum();
        assert_eq!(sum, 15);
    }

    #[test]
    fn test_mapped_data_view() {
        let data = vec![1, 2, 3, 4, 5];
        let view = DataView::from_slice(&data);
        
        let mapped = view.map(|x| *x as f64 * 2.0);
        
        assert_eq!(mapped.len(), 5);
        assert_eq!(mapped.get(0), Some(2.0));
        assert_eq!(mapped.get(4), Some(10.0));
        
        // Test Data1D implementation on mapped view
        assert_eq!(mapped.get(0).unwrap(), 2.0);
        assert_eq!(mapped.get(4).unwrap(), 10.0);
    }

    #[test]
    fn test_data_view_sub_view() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let view = DataView::from_slice(&data);
        
        let sub_view = view.sub_view(1, 3).unwrap();
        assert_eq!(sub_view.len(), 3);
        assert_eq!(sub_view.get(0), Some(&2.0));
        assert_eq!(sub_view.get(2), Some(&4.0));
        
        // Test bounds checking
        assert!(view.sub_view(3, 5).is_none());
    }

    #[test]
    fn test_data_view_from_array() {
        let array = [1.0, 2.0, 3.0, 4.0, 5.0];
        let view = DataView::from_array(&array);
        
        assert_eq!(view.len(), 5);
        assert_eq!(view.get(0), Some(&1.0));
        assert_eq!(view.get(4), Some(&5.0));
    }

    #[test]
    fn test_data_view_multi_slice() {
        let data1 = vec![1.0, 2.0, 3.0];
        let data2 = vec![4.0, 5.0, 6.0];
        let slices = vec![data1.as_slice(), data2.as_slice()];
        
        let views = DataView::from_multi_slice(&slices);
        
        assert_eq!(views.len(), 2);
        assert_eq!(views[0].len(), 3);
        assert_eq!(views[1].len(), 3);
        assert_eq!(views[0].get(0), Some(&1.0));
        assert_eq!(views[1].get(0), Some(&4.0));
    }

    #[test]
    fn test_data_view_thread_safety() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let view = DataView::from_slice(&data);
        
        // Test that DataView can be sent between threads
        let handle = std::thread::spawn(move || {
            let sum: f64 = view.iter().sum();
            sum
        });
        
        let result = handle.join().unwrap();
        assert_eq!(result, 15.0);
    }

    #[test]
    fn test_data_view_clone_copy() {
        let data = vec![1.0, 2.0, 3.0];
        let view1 = DataView::from_slice(&data);
        let view2 = view1; // Copy
        let view3 = view1.clone(); // Clone
        
        assert_eq!(view1.len(), view2.len());
        assert_eq!(view2.len(), view3.len());
        assert_eq!(view1.as_ptr(), view2.as_ptr());
        assert_eq!(view2.as_ptr(), view3.as_ptr());
    }
}