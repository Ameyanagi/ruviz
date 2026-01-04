/// Core data abstraction trait for 1D data series
///
/// This trait allows ruviz to accept various data types (Vec, arrays, slices)
/// without forcing users to convert their data into specific types.
pub trait Data1D<T> {
    /// Get the length of the data series
    fn len(&self) -> usize;

    /// Check if the data series is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a value at the specified index
    fn get(&self, index: usize) -> Option<&T>;

    /// Create an iterator over the data
    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_>;
}

// Blanket implementations for common Rust types

impl<T> Data1D<T> for Vec<T> {
    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn get(&self, index: usize) -> Option<&T> {
        <[T]>::get(self, index)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new(<[T]>::iter(self))
    }
}

impl<T> Data1D<T> for &Vec<T> {
    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn get(&self, index: usize) -> Option<&T> {
        (**self).get(index)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new((**self).iter())
    }
}

impl<T, const N: usize> Data1D<T> for [T; N] {
    fn len(&self) -> usize {
        N
    }

    fn get(&self, index: usize) -> Option<&T> {
        <[T]>::get(self, index)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new(<[T]>::iter(self))
    }
}

impl<T, const N: usize> Data1D<T> for &[T; N] {
    fn len(&self) -> usize {
        N
    }

    fn get(&self, index: usize) -> Option<&T> {
        (**self).get(index)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new((**self).iter())
    }
}

impl<T> Data1D<T> for &[T] {
    fn len(&self) -> usize {
        <[T]>::len(self)
    }

    fn get(&self, index: usize) -> Option<&T> {
        <[T]>::get(self, index)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new(<[T]>::iter(self))
    }
}

// Optional feature-gated implementations

#[cfg(feature = "ndarray_support")]
impl<T> Data1D<T> for ndarray::Array1<T> {
    fn len(&self) -> usize {
        self.len()
    }

    fn get(&self, index: usize) -> Option<&T> {
        self.get(index)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new(self.iter())
    }
}

#[cfg(feature = "polars_support")]
impl Data1D<f64> for polars::series::Series {
    fn len(&self) -> usize {
        self.len()
    }

    fn get(&self, _index: usize) -> Option<&f64> {
        // Polars Series doesn't provide stable references to its data
        // This would require storing values elsewhere to provide references
        // TODO: Implement proper polars integration with value storage
        None
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &f64> + '_> {
        // Simplified iterator - real implementation would handle all numeric types
        match self.dtype() {
            polars::datatypes::DataType::Float64 => {
                let values: Vec<f64> = self
                    .f64()
                    .unwrap()
                    .into_iter()
                    .map(|opt| opt.unwrap_or(0.0))
                    .collect();
                // Note: This creates owned values, not references
                // In a real implementation, we'd need a different approach
                Box::new(std::iter::empty())
            }
            _ => Box::new(std::iter::empty()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_data1d() {
        let data = vec![1.0, 2.0, 3.0];
        assert_eq!(data.len(), 3);
        assert_eq!(data.get(1), Some(&2.0));
        assert!(!data.is_empty());

        let collected: Vec<&f64> = data.iter().collect();
        assert_eq!(collected, vec![&1.0, &2.0, &3.0]);
    }

    #[test]
    fn test_array_data1d() {
        let data = [1.0, 2.0, 3.0];
        assert_eq!(data.len(), 3);
        assert_eq!(data.get(0), Some(&1.0));

        let collected: Vec<&f64> = data.iter().collect();
        assert_eq!(collected, vec![&1.0, &2.0, &3.0]);
    }

    #[test]
    fn test_slice_data1d() {
        let data: &[f64] = &[1.0, 2.0, 3.0];
        assert_eq!(data.len(), 3);
        assert_eq!(data.get(2), Some(&3.0));

        let collected: Vec<&f64> = data.iter().collect();
        assert_eq!(collected, vec![&1.0, &2.0, &3.0]);
    }

    #[test]
    fn test_empty_data() {
        let empty: Vec<f64> = vec![];
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
        assert_eq!(empty.get(0), None);
    }
}
