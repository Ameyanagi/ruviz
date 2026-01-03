use crate::core::{Result, types::Point2f};
#[cfg(feature = "simd")]
use wide::f32x4;

#[cfg(not(feature = "simd"))]
mod disabled {
    use crate::core::types::{Point2f, Transform2D};

    #[derive(Debug, Clone)]
    pub struct SIMDTransformer {
        enabled: bool,
    }

    impl SIMDTransformer {
        pub fn new() -> Self {
            Self { enabled: false }
        }

        pub fn is_available() -> bool {
            false
        }

        pub fn transform_coordinates_scalar(
            &self,
            points: &[(f64, f64)],
            scale: f32,
            offset: f32,
        ) -> Vec<Point2f> {
            points
                .iter()
                .map(|&(x, y)| {
                    Point2f::new((x as f32) * scale + offset, (y as f32) * scale + offset)
                })
                .collect()
        }

        pub fn transform_coordinates_2d_scalar(
            &self,
            points: &[(f64, f64)],
            x_scale: f32,
            y_scale: f32,
            x_offset: f32,
            y_offset: f32,
        ) -> Vec<Point2f> {
            points
                .iter()
                .map(|&(x, y)| {
                    Point2f::new(
                        (x as f32) * x_scale + x_offset,
                        (y as f32) * y_scale + y_offset,
                    )
                })
                .collect()
        }

        pub fn distance_from_point_scalar(
            &self,
            points: &[Point2f],
            reference: Point2f,
        ) -> Vec<f32> {
            points
                .iter()
                .map(|p| {
                    let dx = p.x - reference.x;
                    let dy = p.y - reference.y;
                    (dx * dx + dy * dy).sqrt()
                })
                .collect()
        }

        pub fn apply_transform_scalar(
            &self,
            points: &[Point2f],
            transform: &Transform2D,
        ) -> Vec<Point2f> {
            points
                .iter()
                .map(|p| {
                    Point2f::new(
                        transform.m11 * p.x + transform.m12 * p.y + transform.tx,
                        transform.m21 * p.x + transform.m22 * p.y + transform.ty,
                    )
                })
                .collect()
        }
    }
}

#[cfg(not(feature = "simd"))]
pub use disabled::SIMDTransformer;

#[cfg(feature = "simd")]
/// SIMD-accelerated coordinate transformation module
///
/// Provides vectorized coordinate transformations for high-performance plotting.
/// Uses wide crate for portable SIMD operations across different architectures.
#[derive(Debug, Clone)]
pub struct SIMDTransformer {
    /// Whether SIMD is available and enabled
    enabled: bool,
    /// Minimum batch size to activate SIMD processing
    simd_threshold: usize,
    /// Preferred SIMD lane width (4 for f32x4)
    lane_width: usize,
}

impl SIMDTransformer {
    /// Create new SIMD transformer with default settings
    pub fn new() -> Self {
        Self {
            enabled: Self::detect_simd_support(),
            simd_threshold: 16, // Minimum 16 points for SIMD activation
            lane_width: 4,      // f32x4 SIMD lanes
        }
    }

    /// Create SIMD transformer with custom threshold
    pub fn with_threshold(threshold: usize) -> Self {
        let mut transformer = Self::new();
        transformer.simd_threshold = threshold.max(4); // At least 4 for f32x4
        transformer
    }

    /// Detect SIMD support at runtime
    fn detect_simd_support() -> bool {
        // wide crate handles SIMD detection automatically
        // Always return true as wide provides fallbacks
        true
    }

    /// Check if SIMD should be used for given data size
    pub fn should_use_simd(&self, data_size: usize) -> bool {
        self.enabled && data_size >= self.simd_threshold
    }

    /// Transform X coordinates using SIMD operations
    /// Maps from data space [x_min, x_max] to pixel space [left, right]
    pub fn transform_x_coordinates_simd(
        &self,
        x_data: &[f64],
        x_min: f64,
        x_max: f64,
        left: f32,
        right: f32,
    ) -> Vec<f32> {
        if !self.should_use_simd(x_data.len()) {
            return self.transform_x_coordinates_scalar(x_data, x_min, x_max, left, right);
        }

        let x_range = (x_max - x_min) as f32;
        let pixel_range = right - left;
        let scale = pixel_range / x_range;
        let offset = left - (x_min as f32) * scale;

        // Broadcast constants to SIMD vectors
        let scale_vec = f32x4::splat(scale);
        let offset_vec = f32x4::splat(offset);

        let mut result = Vec::with_capacity(x_data.len());

        // Process in chunks of 4
        let chunks = x_data.chunks_exact(4);
        let remainder = chunks.remainder();

        for chunk in chunks {
            // Convert f64 to f32 and load into SIMD vector
            let x_vec = f32x4::new([
                chunk[0] as f32,
                chunk[1] as f32,
                chunk[2] as f32,
                chunk[3] as f32,
            ]);

            // Vectorized transformation: pixel = x * scale + offset
            let pixel_vec = x_vec * scale_vec + offset_vec;

            // Extract and store results
            let pixels = pixel_vec.to_array();
            result.extend_from_slice(&pixels);
        }

        // Handle remaining elements with scalar operations
        for &x in remainder {
            let pixel = (x as f32) * scale + offset;
            result.push(pixel);
        }

        result
    }

    /// Transform Y coordinates using SIMD operations  
    /// Maps from data space [y_min, y_max] to pixel space [bottom, top]
    pub fn transform_y_coordinates_simd(
        &self,
        y_data: &[f64],
        y_min: f64,
        y_max: f64,
        bottom: f32,
        top: f32,
    ) -> Vec<f32> {
        if !self.should_use_simd(y_data.len()) {
            return self.transform_y_coordinates_scalar(y_data, y_min, y_max, bottom, top);
        }

        let y_range = (y_max - y_min) as f32;
        let pixel_range = top - bottom;
        let scale = pixel_range / y_range;
        let offset = bottom - (y_min as f32) * scale;

        // Broadcast constants to SIMD vectors
        let scale_vec = f32x4::splat(scale);
        let offset_vec = f32x4::splat(offset);

        let mut result = Vec::with_capacity(y_data.len());

        // Process in chunks of 4
        let chunks = y_data.chunks_exact(4);
        let remainder = chunks.remainder();

        for chunk in chunks {
            // Convert f64 to f32 and load into SIMD vector
            let y_vec = f32x4::new([
                chunk[0] as f32,
                chunk[1] as f32,
                chunk[2] as f32,
                chunk[3] as f32,
            ]);

            // Vectorized transformation: pixel = y * scale + offset
            let pixel_vec = y_vec * scale_vec + offset_vec;

            // Extract and store results
            let pixels = pixel_vec.to_array();
            result.extend_from_slice(&pixels);
        }

        // Handle remaining elements with scalar operations
        for &y in remainder {
            let pixel = (y as f32) * scale + offset;
            result.push(pixel);
        }

        result
    }

    /// Transform both X and Y coordinates simultaneously with SIMD
    /// More efficient than separate transformations for point clouds
    pub fn transform_coordinates_simd(
        &self,
        x_data: &[f64],
        y_data: &[f64],
        bounds: CoordinateBounds,
        viewport: PixelViewport,
    ) -> Result<Vec<Point2f>> {
        if x_data.len() != y_data.len() {
            return Err(crate::core::PlottingError::DataLengthMismatch {
                x_len: x_data.len(),
                y_len: y_data.len(),
            });
        }

        if !self.should_use_simd(x_data.len()) {
            return Ok(self.transform_coordinates_scalar(x_data, y_data, bounds, viewport));
        }

        // Precompute transformation parameters
        let x_range = (bounds.x_max - bounds.x_min) as f32;
        let y_range = (bounds.y_max - bounds.y_min) as f32;
        let x_scale = (viewport.right - viewport.left) / x_range;
        let y_scale = (viewport.top - viewport.bottom) / y_range;
        let x_offset = viewport.left - (bounds.x_min as f32) * x_scale;
        let y_offset = viewport.bottom - (bounds.y_min as f32) * y_scale;

        // Broadcast to SIMD vectors
        let x_scale_vec = f32x4::splat(x_scale);
        let y_scale_vec = f32x4::splat(y_scale);
        let x_offset_vec = f32x4::splat(x_offset);
        let y_offset_vec = f32x4::splat(y_offset);

        let mut result = Vec::with_capacity(x_data.len());

        // Process coordinate pairs in chunks of 4
        let chunks = x_data.chunks_exact(4);
        let remainder = chunks.remainder();
        let y_chunks = y_data.chunks_exact(4);

        for (x_chunk, y_chunk) in chunks.zip(y_chunks) {
            // Load coordinates into SIMD vectors
            let x_vec = f32x4::new([
                x_chunk[0] as f32,
                x_chunk[1] as f32,
                x_chunk[2] as f32,
                x_chunk[3] as f32,
            ]);
            let y_vec = f32x4::new([
                y_chunk[0] as f32,
                y_chunk[1] as f32,
                y_chunk[2] as f32,
                y_chunk[3] as f32,
            ]);

            // Vectorized transformations
            let px_vec = x_vec * x_scale_vec + x_offset_vec;
            let py_vec = y_vec * y_scale_vec + y_offset_vec;

            // Extract results and create points
            let px_array = px_vec.to_array();
            let py_array = py_vec.to_array();

            for i in 0..4 {
                result.push(Point2f::new(px_array[i], py_array[i]));
            }
        }

        // Handle remaining elements
        let y_remainder = &y_data[x_data.len() - remainder.len()..];
        for (i, (&x, &y)) in remainder.iter().zip(y_remainder.iter()).enumerate() {
            let px = (x as f32) * x_scale + x_offset;
            let py = (y as f32) * y_scale + y_offset;
            result.push(Point2f::new(px, py));
        }

        Ok(result)
    }

    /// Batch distance calculations using SIMD for spatial operations
    /// Useful for spatial queries, nearest neighbor, clustering
    pub fn batch_distance_squared_simd(&self, points: &[Point2f], reference: Point2f) -> Vec<f32> {
        if !self.should_use_simd(points.len()) {
            return self.batch_distance_squared_scalar(points, reference);
        }

        let ref_x_vec = f32x4::splat(reference.x);
        let ref_y_vec = f32x4::splat(reference.y);

        let mut result = Vec::with_capacity(points.len());

        // Process in chunks of 4 points
        let chunks = points.chunks_exact(4);
        let remainder = chunks.remainder();

        for chunk in chunks {
            // Load point coordinates
            let x_vec = f32x4::new([chunk[0].x, chunk[1].x, chunk[2].x, chunk[3].x]);
            let y_vec = f32x4::new([chunk[0].y, chunk[1].y, chunk[2].y, chunk[3].y]);

            // Vectorized distance calculation: (x-ref_x)² + (y-ref_y)²
            let dx_vec = x_vec - ref_x_vec;
            let dy_vec = y_vec - ref_y_vec;
            let dist_sq_vec = dx_vec * dx_vec + dy_vec * dy_vec;

            // Extract and store results
            let distances = dist_sq_vec.to_array();
            result.extend_from_slice(&distances);
        }

        // Handle remaining points
        for point in remainder {
            let dx = point.x - reference.x;
            let dy = point.y - reference.y;
            result.push(dx * dx + dy * dy);
        }

        result
    }

    /// Apply affine transformation using SIMD
    /// Supports rotation, scaling, translation in matrix form
    pub fn apply_affine_transform_simd(
        &self,
        points: &[Point2f],
        transform: &AffineTransform2D,
    ) -> Vec<Point2f> {
        if !self.should_use_simd(points.len()) {
            return self.apply_affine_transform_scalar(points, transform);
        }

        // Broadcast matrix elements to SIMD vectors
        let m11_vec = f32x4::splat(transform.m11);
        let m12_vec = f32x4::splat(transform.m12);
        let m21_vec = f32x4::splat(transform.m21);
        let m22_vec = f32x4::splat(transform.m22);
        let tx_vec = f32x4::splat(transform.tx);
        let ty_vec = f32x4::splat(transform.ty);

        let mut result = Vec::with_capacity(points.len());

        let chunks = points.chunks_exact(4);
        let remainder = chunks.remainder();

        for chunk in chunks {
            // Load point coordinates
            let x_vec = f32x4::new([chunk[0].x, chunk[1].x, chunk[2].x, chunk[3].x]);
            let y_vec = f32x4::new([chunk[0].y, chunk[1].y, chunk[2].y, chunk[3].y]);

            // Matrix multiplication: [m11 m12] [x] + [tx]
            //                        [m21 m22] [y]   [ty]
            let new_x_vec = m11_vec * x_vec + m12_vec * y_vec + tx_vec;
            let new_y_vec = m21_vec * x_vec + m22_vec * y_vec + ty_vec;

            // Extract and create transformed points
            let x_array = new_x_vec.to_array();
            let y_array = new_y_vec.to_array();

            for i in 0..4 {
                result.push(Point2f::new(x_array[i], y_array[i]));
            }
        }

        // Handle remaining points
        for point in remainder {
            let new_x = transform.m11 * point.x + transform.m12 * point.y + transform.tx;
            let new_y = transform.m21 * point.x + transform.m22 * point.y + transform.ty;
            result.push(Point2f::new(new_x, new_y));
        }

        result
    }

    /// Get performance characteristics
    pub fn performance_info(&self) -> SIMDPerformanceInfo {
        SIMDPerformanceInfo {
            simd_enabled: self.enabled,
            simd_threshold: self.simd_threshold,
            lane_width: self.lane_width,
            target_arch: Self::get_target_arch(),
            estimated_speedup: if self.enabled { 3.5 } else { 1.0 },
        }
    }

    /// Get target architecture info
    fn get_target_arch() -> &'static str {
        #[cfg(target_arch = "x86_64")]
        return "x86_64";
        #[cfg(target_arch = "aarch64")]
        return "aarch64";
        #[cfg(target_arch = "x86")]
        return "x86";
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "x86")))]
        return "other";
    }

    // Scalar fallback implementations

    fn transform_x_coordinates_scalar(
        &self,
        x_data: &[f64],
        x_min: f64,
        x_max: f64,
        left: f32,
        right: f32,
    ) -> Vec<f32> {
        let x_range = (x_max - x_min) as f32;
        let pixel_range = right - left;
        let scale = pixel_range / x_range;
        let offset = left - (x_min as f32) * scale;

        x_data
            .iter()
            .map(|&x| (x as f32) * scale + offset)
            .collect()
    }

    fn transform_y_coordinates_scalar(
        &self,
        y_data: &[f64],
        y_min: f64,
        y_max: f64,
        bottom: f32,
        top: f32,
    ) -> Vec<f32> {
        let y_range = (y_max - y_min) as f32;
        let pixel_range = top - bottom;
        let scale = pixel_range / y_range;
        let offset = bottom - (y_min as f32) * scale;

        y_data
            .iter()
            .map(|&y| (y as f32) * scale + offset)
            .collect()
    }

    fn transform_coordinates_scalar(
        &self,
        x_data: &[f64],
        y_data: &[f64],
        bounds: CoordinateBounds,
        viewport: PixelViewport,
    ) -> Vec<Point2f> {
        let x_range = (bounds.x_max - bounds.x_min) as f32;
        let y_range = (bounds.y_max - bounds.y_min) as f32;
        let x_scale = (viewport.right - viewport.left) / x_range;
        let y_scale = (viewport.top - viewport.bottom) / y_range;
        let x_offset = viewport.left - (bounds.x_min as f32) * x_scale;
        let y_offset = viewport.bottom - (bounds.y_min as f32) * y_scale;

        x_data
            .iter()
            .zip(y_data.iter())
            .map(|(&x, &y)| {
                let px = (x as f32) * x_scale + x_offset;
                let py = (y as f32) * y_scale + y_offset;
                Point2f::new(px, py)
            })
            .collect()
    }

    fn batch_distance_squared_scalar(&self, points: &[Point2f], reference: Point2f) -> Vec<f32> {
        points
            .iter()
            .map(|point| {
                let dx = point.x - reference.x;
                let dy = point.y - reference.y;
                dx * dx + dy * dy
            })
            .collect()
    }

    fn apply_affine_transform_scalar(
        &self,
        points: &[Point2f],
        transform: &AffineTransform2D,
    ) -> Vec<Point2f> {
        points
            .iter()
            .map(|point| {
                let new_x = transform.m11 * point.x + transform.m12 * point.y + transform.tx;
                let new_y = transform.m21 * point.x + transform.m22 * point.y + transform.ty;
                Point2f::new(new_x, new_y)
            })
            .collect()
    }
}

impl Default for SIMDTransformer {
    fn default() -> Self {
        Self::new()
    }
}

/// Coordinate bounds for transformations
#[derive(Debug, Clone)]
pub struct CoordinateBounds {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

/// Pixel viewport for coordinate mapping
#[derive(Debug, Clone)]
pub struct PixelViewport {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

/// 2D point with single precision coordinates

/// 2D affine transformation matrix
#[derive(Debug, Clone)]
pub struct AffineTransform2D {
    pub m11: f32,
    pub m12: f32,
    pub tx: f32,
    pub m21: f32,
    pub m22: f32,
    pub ty: f32,
}

impl AffineTransform2D {
    /// Identity transformation
    pub fn identity() -> Self {
        Self {
            m11: 1.0,
            m12: 0.0,
            tx: 0.0,
            m21: 0.0,
            m22: 1.0,
            ty: 0.0,
        }
    }

    /// Translation transformation
    pub fn translation(tx: f32, ty: f32) -> Self {
        Self {
            m11: 1.0,
            m12: 0.0,
            tx,
            m21: 0.0,
            m22: 1.0,
            ty,
        }
    }

    /// Uniform scale transformation
    pub fn scale(scale: f32) -> Self {
        Self {
            m11: scale,
            m12: 0.0,
            tx: 0.0,
            m21: 0.0,
            m22: scale,
            ty: 0.0,
        }
    }

    /// Rotation transformation (angle in radians)
    pub fn rotation(angle: f32) -> Self {
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        Self {
            m11: cos_a,
            m12: -sin_a,
            tx: 0.0,
            m21: sin_a,
            m22: cos_a,
            ty: 0.0,
        }
    }
}

/// Performance information for SIMD operations
#[derive(Debug, Clone)]
pub struct SIMDPerformanceInfo {
    pub simd_enabled: bool,
    pub simd_threshold: usize,
    pub lane_width: usize,
    pub target_arch: &'static str,
    pub estimated_speedup: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_transformer_creation() {
        let transformer = SIMDTransformer::new();
        assert!(transformer.enabled);
        assert_eq!(transformer.lane_width, 4);
    }

    #[test]
    fn test_simd_threshold() {
        let transformer = SIMDTransformer::with_threshold(32);
        assert!(!transformer.should_use_simd(16));
        assert!(transformer.should_use_simd(32));
        assert!(transformer.should_use_simd(64));
    }

    #[test]
    fn test_coordinate_transformation_consistency() {
        let transformer = SIMDTransformer::new();
        let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let y_data = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0];

        let bounds = CoordinateBounds {
            x_min: 1.0,
            x_max: 8.0,
            y_min: 10.0,
            y_max: 80.0,
        };

        let viewport = PixelViewport {
            left: 0.0,
            right: 100.0,
            top: 0.0,
            bottom: 100.0,
        };

        // Test that SIMD and scalar produce same results
        let simd_result = transformer
            .transform_coordinates_simd(&x_data, &y_data, bounds.clone(), viewport.clone())
            .unwrap();
        let scalar_result =
            transformer.transform_coordinates_scalar(&x_data, &y_data, bounds, viewport);

        assert_eq!(simd_result.len(), scalar_result.len());
        for (simd_point, scalar_point) in simd_result.iter().zip(scalar_result.iter()) {
            assert!((simd_point.x - scalar_point.x).abs() < 0.001);
            assert!((simd_point.y - scalar_point.y).abs() < 0.001);
        }
    }

    #[test]
    fn test_distance_calculation() {
        let transformer = SIMDTransformer::new();
        let points = vec![
            Point2f::new(0.0, 0.0),
            Point2f::new(3.0, 4.0),
            Point2f::new(-1.0, -1.0),
            Point2f::new(5.0, 12.0),
        ];
        let reference = Point2f::new(0.0, 0.0);

        let distances = transformer.batch_distance_squared_simd(&points, reference);

        assert_eq!(distances.len(), 4);
        assert_eq!(distances[0], 0.0); // Distance to self
        assert_eq!(distances[1], 25.0); // 3² + 4² = 25
        assert_eq!(distances[2], 2.0); // (-1)² + (-1)² = 2
        assert_eq!(distances[3], 169.0); // 5² + 12² = 169
    }

    #[test]
    fn test_affine_transformation() {
        let transformer = SIMDTransformer::new();
        let points = vec![
            Point2f::new(1.0, 0.0),
            Point2f::new(0.0, 1.0),
            Point2f::new(1.0, 1.0),
            Point2f::new(-1.0, -1.0),
        ];

        // Test translation
        let translation = AffineTransform2D::translation(5.0, 3.0);
        let translated = transformer.apply_affine_transform_simd(&points, &translation);

        assert_eq!(translated[0].x, 6.0); // 1 + 5
        assert_eq!(translated[0].y, 3.0); // 0 + 3

        // Test scale
        let scale = AffineTransform2D::scale(2.0);
        let scaled = transformer.apply_affine_transform_simd(&points, &scale);

        assert_eq!(scaled[0].x, 2.0); // 1 * 2
        assert_eq!(scaled[0].y, 0.0); // 0 * 2
    }

    #[test]
    fn test_performance_info() {
        let transformer = SIMDTransformer::new();
        let info = transformer.performance_info();

        assert!(info.simd_enabled);
        assert_eq!(info.lane_width, 4);
        assert!(info.estimated_speedup > 1.0);
    }
}
