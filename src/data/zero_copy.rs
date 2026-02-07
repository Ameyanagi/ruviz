use std::slice;

use crate::data::{Data1D, PooledVec};

/// Zero-copy view into borrowed data.
#[derive(Clone, Copy)]
pub struct DataView<'a, T> {
    slice: &'a [T],
}

impl<'a, T> DataView<'a, T> {
    /// Create a zero-copy DataView from a slice.
    pub fn from_slice(slice: &'a [T]) -> Self {
        Self { slice }
    }

    /// Create a zero-copy DataView from a PooledVec.
    pub fn from_pooled_vec(pooled_vec: &'a PooledVec<T>) -> Self {
        Self::from_slice(pooled_vec.as_slice())
    }

    /// Create a zero-copy DataView from a regular Vec.
    pub fn from_vec(vec: &'a Vec<T>) -> Self {
        Self::from_slice(vec.as_slice())
    }

    pub fn len(&self) -> usize {
        self.slice.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slice.is_empty()
    }

    pub fn as_ptr(&self) -> *const T {
        self.slice.as_ptr()
    }

    pub fn as_slice(&self) -> &'a [T] {
        self.slice
    }

    pub fn get(&self, index: usize) -> Option<&'a T> {
        self.slice.get(index)
    }

    pub fn iter(&self) -> DataViewIter<'a, T>
    where
        T: Copy,
    {
        DataViewIter {
            inner: self.slice.iter(),
        }
    }

    pub fn map<U, F>(self, f: F) -> MappedDataView<'a, T, U, F>
    where
        F: Fn(&T) -> U,
    {
        MappedDataView {
            original: self,
            mapper: f,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn sub_view(&self, start: usize, len: usize) -> Option<Self> {
        let end = start.checked_add(len)?;
        self.slice.get(start..end).map(Self::from_slice)
    }

    pub fn from_array<const N: usize>(array: &'a [T; N]) -> Self {
        Self::from_slice(array.as_slice())
    }

    pub fn from_multi_slice(data: &[&'a [T]]) -> Vec<Self> {
        data.iter().copied().map(Self::from_slice).collect()
    }
}

impl<'a, T> Data1D<T> for DataView<'a, T> {
    fn len(&self) -> usize {
        self.slice.len()
    }

    fn get(&self, idx: usize) -> Option<&T> {
        self.slice.get(idx)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new(self.slice.iter())
    }
}

/// Owned view storage for materialized data sources.
pub struct OwnedDataView<T> {
    storage: Box<[T]>,
}

impl<T> OwnedDataView<T> {
    pub fn new(storage: Box<[T]>) -> Self {
        Self { storage }
    }

    pub fn from_range<R>(range: R) -> Self
    where
        R: Iterator<Item = T>,
    {
        let data: Vec<T> = range.collect();
        Self {
            storage: data.into_boxed_slice(),
        }
    }

    pub fn view(&self) -> DataView<'_, T> {
        DataView::from_slice(&self.storage)
    }

    pub fn as_slice(&self) -> &[T] {
        &self.storage
    }
}

/// Iterator over a DataView that returns owned values (for Copy types).
pub struct DataViewIter<'a, T>
where
    T: Copy,
{
    inner: slice::Iter<'a, T>,
}

impl<'a, T> Iterator for DataViewIter<'a, T>
where
    T: Copy,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().copied()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, T> ExactSizeIterator for DataViewIter<'a, T> where T: Copy {}

/// A DataView that applies a mapping function to each element.
pub struct MappedDataView<'a, T, U, F> {
    original: DataView<'a, T>,
    mapper: F,
    _phantom: std::marker::PhantomData<U>,
}

impl<'a, T, U, F> MappedDataView<'a, T, U, F>
where
    F: Fn(&T) -> U,
{
    pub fn len(&self) -> usize {
        self.original.len()
    }

    pub fn is_empty(&self) -> bool {
        self.original.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<U> {
        self.original.get(index).map(&self.mapper)
    }

    pub fn iter(&self) -> MappedDataViewIter<'_, 'a, T, U, F>
    where
        T: Copy,
    {
        MappedDataViewIter {
            original_iter: self.original.iter(),
            mapper: &self.mapper,
            _phantom: std::marker::PhantomData,
        }
    }
}

pub struct MappedDataViewIter<'m, 'a, T, U, F>
where
    T: Copy,
{
    original_iter: DataViewIter<'a, T>,
    mapper: &'m F,
    _phantom: std::marker::PhantomData<U>,
}

impl<'m, 'a, T, U, F> Iterator for MappedDataViewIter<'m, 'a, T, U, F>
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

impl<'m, 'a, T, U, F> ExactSizeIterator for MappedDataViewIter<'m, 'a, T, U, F>
where
    T: Copy,
    F: Fn(&T) -> U,
{
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

        use crate::data::Data1D;
        assert_eq!(view.len(), 5);
        assert_eq!(*view.get(0).expect("missing item"), 1.0);
        assert_eq!(*view.get(4).expect("missing item"), 5.0);
        assert!(view.get(5).is_none());

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
    }

    #[test]
    fn test_data_view_sub_view() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let view = DataView::from_slice(&data);

        let sub_view = view.sub_view(1, 3).expect("subview should exist");
        assert_eq!(sub_view.len(), 3);
        assert_eq!(sub_view.get(0), Some(&2.0));
        assert_eq!(sub_view.get(2), Some(&4.0));

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

        std::thread::scope(|scope| {
            let view = DataView::from_slice(&data);
            let handle = scope.spawn(move || {
                let sum: f64 = view.iter().sum();
                sum
            });

            assert_eq!(handle.join().expect("thread panicked"), 15.0);
        });
    }

    #[test]
    fn test_data_view_clone_copy() {
        let data = vec![1.0, 2.0, 3.0];
        let view1 = DataView::from_slice(&data);
        let view2 = view1;
        let view3 = view1;

        assert_eq!(view1.len(), view2.len());
        assert_eq!(view2.len(), view3.len());
        assert_eq!(view1.as_ptr(), view2.as_ptr());
        assert_eq!(view2.as_ptr(), view3.as_ptr());
    }

    #[test]
    fn test_owned_data_view_from_range() {
        let owned = OwnedDataView::from_range(0..5);
        let view = owned.view();
        assert_eq!(view.as_slice(), &[0, 1, 2, 3, 4]);
    }
}
