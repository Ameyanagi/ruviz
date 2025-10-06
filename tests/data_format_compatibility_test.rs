// Data format compatibility tests
// Tests that various data types and formats work correctly with the plotting API

use ruviz::prelude::*;

#[test]
fn test_vec_f64() {
    // GIVEN: Vec<f64> data
    let x: Vec<f64> = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y: Vec<f64> = vec![0.0, 1.0, 4.0, 9.0, 16.0];

    // WHEN: Plotting with Vec<f64>
    let result = Plot::new()
        .line(&x, &y)
        .title("Vec<f64> Data")
        .save("test_output/data_vec_f64.png");

    // THEN: Should succeed
    assert!(result.is_ok(), "Vec<f64> failed: {:?}", result.err());
}

#[test]
fn test_vec_f32() {
    // GIVEN: Vec<f32> data
    let x: Vec<f32> = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y: Vec<f32> = vec![0.0, 1.0, 4.0, 9.0, 16.0];

    // Convert to f64 as required by API
    let x_f64: Vec<f64> = x.iter().map(|&v| v as f64).collect();
    let y_f64: Vec<f64> = y.iter().map(|&v| v as f64).collect();

    // WHEN: Plotting with converted f32
    let result = Plot::new()
        .line(&x_f64, &y_f64)
        .title("Vec<f32> Data")
        .save("test_output/data_vec_f32.png");

    // THEN: Should succeed
    assert!(result.is_ok(), "Vec<f32> conversion failed: {:?}", result.err());
}

#[test]
fn test_slice_f64() {
    // GIVEN: Slice data
    let x = [0.0, 1.0, 2.0, 3.0, 4.0];
    let y = [0.0, 1.0, 4.0, 9.0, 16.0];

    // WHEN: Plotting with slices
    let result = Plot::new()
        .line(&x, &y)
        .title("Slice Data")
        .save("test_output/data_slice.png");

    // THEN: Should succeed
    assert!(result.is_ok(), "Slice data failed: {:?}", result.err());
}

#[test]
fn test_integer_data() {
    // GIVEN: Integer data
    let x: Vec<i32> = vec![0, 1, 2, 3, 4];
    let y: Vec<i32> = vec![0, 1, 4, 9, 16];

    // Convert to f64 as required by API
    let x_f64: Vec<f64> = x.iter().map(|&v| v as f64).collect();
    let y_f64: Vec<f64> = y.iter().map(|&v| v as f64).collect();

    // WHEN: Plotting with integer data
    let result = Plot::new()
        .line(&x_f64, &y_f64)
        .title("Integer Data")
        .save("test_output/data_integer.png");

    // THEN: Should succeed
    assert!(result.is_ok(), "Integer data failed: {:?}", result.err());
}

#[test]
#[cfg(feature = "ndarray_support")]
fn test_ndarray_data() {
    use ndarray::Array1;

    // GIVEN: ndarray data
    let x = Array1::linspace(0.0_f64, 10.0_f64, 50);
    let y = x.mapv(|v: f64| v.sin());

    // WHEN: Plotting with ndarray
    let result = Plot::new()
        .line(&x, &y)
        .title("ndarray Data")
        .save("test_output/data_ndarray.png");

    // THEN: Should succeed
    assert!(result.is_ok(), "ndarray data failed: {:?}", result.err());
}

#[test]
fn test_string_categories() {
    // GIVEN: String categories
    let categories: Vec<String> = vec!["A".to_string(), "B".to_string(), "C".to_string()];
    let values = vec![10.0, 20.0, 15.0];

    // Convert to &str slices
    let cat_strs: Vec<&str> = categories.iter().map(|s| s.as_str()).collect();

    // WHEN: Plotting with String categories
    let result = Plot::new()
        .bar(&cat_strs, &values)
        .title("String Categories")
        .save("test_output/data_string_categories.png");

    // THEN: Should succeed
    assert!(result.is_ok(), "String categories failed: {:?}", result.err());
}

#[test]
fn test_str_slice_categories() {
    // GIVEN: &str slice categories
    let categories = ["Category A", "Category B", "Category C", "Category D"];
    let values = vec![25.0, 40.0, 30.0, 55.0];

    // WHEN: Plotting with &str slices
    let result = Plot::new()
        .bar(&categories, &values)
        .title("&str Slice Categories")
        .save("test_output/data_str_categories.png");

    // THEN: Should succeed
    assert!(result.is_ok(), "&str categories failed: {:?}", result.err());
}

#[test]
fn test_empty_data_error() {
    // GIVEN: Empty data
    let x: Vec<f64> = vec![];
    let y: Vec<f64> = vec![];

    // WHEN: Attempting to plot empty data
    let result = Plot::new()
        .line(&x, &y)
        .save("test_output/data_should_not_exist.png");

    // THEN: Should fail gracefully
    assert!(result.is_err(), "Empty data should produce error");
}

#[test]
fn test_mismatched_length_error() {
    // GIVEN: Mismatched length data
    let x = vec![1.0, 2.0, 3.0, 4.0];
    let y = vec![1.0, 2.0];

    // WHEN: Attempting to plot mismatched data
    let result = Plot::new()
        .line(&x, &y)
        .save("test_output/data_should_not_exist_2.png");

    // THEN: Should fail gracefully
    assert!(result.is_err(), "Mismatched data should produce error");
}

#[test]
fn test_single_point() {
    // GIVEN: Single data point
    let x = vec![1.0];
    let y = vec![1.0];

    // WHEN: Plotting single point
    let result = Plot::new()
        .scatter(&x, &y)
        .marker(MarkerStyle::Circle)
        .marker_size(10.0)
        .title("Single Point")
        .save("test_output/data_single_point.png");

    // THEN: May fail or succeed depending on implementation (single point is edge case)
    // Just verify it handles gracefully
    println!("Single point result: {:?}", result);
}

#[test]
fn test_two_points() {
    // GIVEN: Two data points (minimum for line)
    let x = vec![0.0, 1.0];
    let y = vec![0.0, 1.0];

    // WHEN: Plotting two points
    let result = Plot::new()
        .line(&x, &y)
        .title("Two Points")
        .save("test_output/data_two_points.png");

    // THEN: Should succeed
    assert!(result.is_ok(), "Two points failed: {:?}", result.err());
}

#[test]
fn test_large_values() {
    // GIVEN: Large values
    let x = vec![1e6, 2e6, 3e6, 4e6, 5e6];
    let y = vec![1e9, 2e9, 3e9, 4e9, 5e9];

    // WHEN: Plotting large values
    let result = Plot::new()
        .line(&x, &y)
        .title("Large Values")
        .save("test_output/data_large_values.png");

    // THEN: Should succeed
    assert!(result.is_ok(), "Large values failed: {:?}", result.err());
}

#[test]
fn test_small_values() {
    // GIVEN: Small values
    let x = vec![1e-6, 2e-6, 3e-6, 4e-6, 5e-6];
    let y = vec![1e-9, 2e-9, 3e-9, 4e-9, 5e-9];

    // WHEN: Plotting small values
    let result = Plot::new()
        .line(&x, &y)
        .title("Small Values")
        .save("test_output/data_small_values.png");

    // THEN: Should succeed
    assert!(result.is_ok(), "Small values failed: {:?}", result.err());
}

#[test]
fn test_negative_values() {
    // GIVEN: Negative values
    let x = vec![-5.0, -4.0, -3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![-25.0, -16.0, -9.0, -4.0, -1.0, 0.0, 1.0, 4.0, 9.0, 16.0, 25.0];

    // WHEN: Plotting negative values
    let result = Plot::new()
        .line(&x, &y)
        .title("Negative Values")
        .save("test_output/data_negative_values.png");

    // THEN: Should succeed
    assert!(result.is_ok(), "Negative values failed: {:?}", result.err());
}

#[test]
fn test_zero_values() {
    // GIVEN: Zero values
    let x = vec![0.0, 0.0, 0.0, 0.0];
    let y = vec![1.0, 2.0, 3.0, 4.0];

    // WHEN: Plotting with zero x values
    let result = Plot::new()
        .scatter(&x, &y)
        .marker(MarkerStyle::Circle)
        .title("Zero X Values")
        .save("test_output/data_zero_x.png");

    // THEN: May fail or succeed depending on implementation (zero range is edge case)
    // Just verify it handles gracefully
    println!("Zero x values result: {:?}", result);
}

#[test]
fn test_constant_values() {
    // GIVEN: Constant values
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![5.0, 5.0, 5.0, 5.0, 5.0];

    // WHEN: Plotting constant y values
    let result = Plot::new()
        .line(&x, &y)
        .title("Constant Y Values")
        .save("test_output/data_constant_y.png");

    // THEN: May fail or succeed depending on implementation (constant values is edge case)
    // Just verify it handles gracefully
    println!("Constant y values result: {:?}", result);
}

#[test]
fn test_nan_handling() {
    // GIVEN: Data with NaN
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![1.0, 2.0, f64::NAN, 4.0, 5.0];

    // WHEN: Attempting to plot with NaN
    let result = Plot::new()
        .line(&x, &y)
        .title("NaN Handling")
        .save("test_output/data_nan.png");

    // THEN: Should handle gracefully (either error or filter NaN)
    // Note: Actual behavior depends on implementation
    println!("NaN handling result: {:?}", result);
}

#[test]
fn test_infinity_handling() {
    // GIVEN: Data with infinity
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![1.0, 2.0, f64::INFINITY, 4.0, 5.0];

    // WHEN: Attempting to plot with infinity
    let result = Plot::new()
        .line(&x, &y)
        .title("Infinity Handling")
        .save("test_output/data_infinity.png");

    // THEN: Should handle gracefully (either error or clip)
    // Note: Actual behavior depends on implementation
    println!("Infinity handling result: {:?}", result);
}

#[test]
fn test_unicode_categories() {
    // GIVEN: Unicode category labels
    let categories = ["α", "β", "γ", "δ"];
    let values = vec![10.0, 20.0, 15.0, 25.0];

    // WHEN: Plotting with Unicode categories
    let result = Plot::new()
        .bar(&categories, &values)
        .title("Unicode Categories")
        .save("test_output/data_unicode_categories.png");

    // THEN: Should succeed
    assert!(result.is_ok(), "Unicode categories failed: {:?}", result.err());
}

#[test]
fn test_long_categories() {
    // GIVEN: Long category labels
    let categories = [
        "Very Long Category Name A",
        "Very Long Category Name B",
        "Very Long Category Name C",
    ];
    let values = vec![10.0, 20.0, 15.0];

    // WHEN: Plotting with long categories
    let result = Plot::new()
        .bar(&categories, &values)
        .title("Long Categories")
        .save("test_output/data_long_categories.png");

    // THEN: Should succeed
    assert!(result.is_ok(), "Long categories failed: {:?}", result.err());
}
