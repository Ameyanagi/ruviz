use super::*;
use crate::core::types::Point2f;

#[derive(Clone, Copy, Debug)]
struct BucketPoint {
    index: usize,
    point: Point2f,
}

#[derive(Clone, Copy, Debug)]
struct ColumnBucket {
    column: i32,
    first: BucketPoint,
    last: BucketPoint,
    min_y: BucketPoint,
    max_y: BucketPoint,
}

pub(super) fn should_reduce_line_series(
    series: &PlotSeries,
    point_count: usize,
    plot_width: f32,
) -> bool {
    if !matches!(series.series_type, SeriesType::Line { .. }) {
        return false;
    }

    if series.marker_style.is_some() || series.x_errors.is_some() || series.y_errors.is_some() {
        return false;
    }

    if series
        .line_style
        .as_ref()
        .is_some_and(|style| !matches!(style, LineStyle::Solid))
    {
        return false;
    }

    point_count > (plot_width.max(1.0).ceil() as usize * 4)
}

pub(super) fn canonicalize_line_points_exact(points: &[Point2f]) -> Option<Vec<Point2f>> {
    if points.len() < 3 || !points.iter().all(is_finite_point) {
        return None;
    }

    let mut canonical = Vec::with_capacity(points.len());
    for point in points.iter().copied() {
        if canonical
            .last()
            .is_some_and(|last: &Point2f| last.x == point.x && last.y == point.y)
        {
            continue;
        }

        while canonical.len() >= 2 {
            let previous = canonical[canonical.len() - 2];
            let current = canonical[canonical.len() - 1];
            if !is_exactly_redundant_line_point(previous, current, point) {
                break;
            }
            canonical.pop();
        }

        canonical.push(point);
    }

    if canonical.len() >= points.len() {
        None
    } else {
        Some(canonical)
    }
}

pub(super) fn reduce_line_points_for_raster(
    points: &[Point2f],
    plot_left: f32,
    plot_width: f32,
) -> Option<Vec<Point2f>> {
    let column_count = plot_width.max(1.0).ceil() as usize;
    if points.len() <= column_count * 4 || !points.iter().all(is_finite_point) {
        return None;
    }

    if !is_monotonic_x(points) {
        return None;
    }

    let max_column = column_count.saturating_sub(1) as i32;
    let mut reduced = Vec::with_capacity(column_count.saturating_mul(4));
    let mut active_bucket: Option<ColumnBucket> = None;

    for (index, point) in points.iter().copied().enumerate() {
        let column = ((point.x - plot_left).floor() as i32).clamp(0, max_column);
        let bucket_point = BucketPoint { index, point };

        match active_bucket.as_mut() {
            Some(bucket) if bucket.column == column => update_bucket(bucket, bucket_point),
            Some(bucket) => {
                flush_bucket(bucket, &mut reduced);
                active_bucket = Some(ColumnBucket {
                    column,
                    first: bucket_point,
                    last: bucket_point,
                    min_y: bucket_point,
                    max_y: bucket_point,
                });
            }
            None => {
                active_bucket = Some(ColumnBucket {
                    column,
                    first: bucket_point,
                    last: bucket_point,
                    min_y: bucket_point,
                    max_y: bucket_point,
                });
            }
        }
    }

    if let Some(bucket) = active_bucket {
        flush_bucket(&bucket, &mut reduced);
    }

    if reduced.len() >= points.len() {
        None
    } else {
        Some(reduced)
    }
}

fn is_finite_point(point: &Point2f) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

fn is_exactly_redundant_line_point(previous: Point2f, current: Point2f, next: Point2f) -> bool {
    let ab_x = current.x - previous.x;
    let ab_y = current.y - previous.y;
    let bc_x = next.x - current.x;
    let bc_y = next.y - current.y;
    if (ab_x == 0.0 && ab_y == 0.0) || (bc_x == 0.0 && bc_y == 0.0) {
        return false;
    }

    let cross = (ab_x as f64) * (bc_y as f64) - (ab_y as f64) * (bc_x as f64);
    if cross != 0.0 {
        return false;
    }

    let dot = (ab_x as f64) * (bc_x as f64) + (ab_y as f64) * (bc_y as f64);
    dot >= 0.0
}

// Points are expected to be in pixel-space after coordinate projection.
// f32::EPSILON is appropriate here; do not reuse this monotonicity check on
// data-space coordinates where adjacent x-values may legitimately differ by
// less than 1e-7.
fn is_monotonic_x(points: &[Point2f]) -> bool {
    if points.len() < 2 {
        return true;
    }

    let mut direction = 0_i8;
    for window in points.windows(2) {
        let delta = window[1].x - window[0].x;
        if delta.abs() <= f32::EPSILON {
            continue;
        }

        let current = if delta.is_sign_positive() { 1 } else { -1 };
        if direction == 0 {
            direction = current;
        } else if direction != current {
            return false;
        }
    }

    true
}

fn update_bucket(bucket: &mut ColumnBucket, point: BucketPoint) {
    bucket.last = point;
    if point.point.y < bucket.min_y.point.y {
        bucket.min_y = point;
    }
    if point.point.y > bucket.max_y.point.y {
        bucket.max_y = point;
    }
}

fn flush_bucket(bucket: &ColumnBucket, output: &mut Vec<Point2f>) {
    let mut candidates = [bucket.first, bucket.min_y, bucket.max_y, bucket.last];
    candidates.sort_by_key(|candidate| candidate.index);

    let mut last_index = None;
    for candidate in candidates {
        if last_index == Some(candidate.index) {
            continue;
        }
        if output
            .last()
            .is_some_and(|last| last.x == candidate.point.x && last.y == candidate.point.y)
        {
            last_index = Some(candidate.index);
            continue;
        }
        output.push(candidate.point);
        last_index = Some(candidate.index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{Color, LineStyle, SkiaRenderer, Theme};

    fn render_polyline(points: &[Point2f], clip_rect: (f32, f32, f32, f32)) -> Image {
        let width = 680;
        let height = 220;
        let mut renderer = SkiaRenderer::new(width, height, Theme::default()).expect("renderer");
        renderer.clear();
        renderer
            .draw_polyline_points_clipped(points, Color::BLACK, 2.0, LineStyle::Solid, clip_rect)
            .expect("polyline render");
        renderer.into_image()
    }

    fn mean_normalized_channel_diff(lhs: &Image, rhs: &Image) -> f64 {
        assert_eq!(lhs.width, rhs.width);
        assert_eq!(lhs.height, rhs.height);

        lhs.pixels
            .iter()
            .zip(&rhs.pixels)
            .map(|(left, right)| (*left as f64 - *right as f64).abs() / 255.0)
            .sum::<f64>()
            / lhs.pixels.len() as f64
    }

    fn image_has_dark_pixel_near(image: &Image, x: u32, y: u32, radius: u32) -> bool {
        let x_start = x.saturating_sub(radius);
        let x_end = (x + radius).min(image.width.saturating_sub(1));
        let y_start = y.saturating_sub(radius);
        let y_end = (y + radius).min(image.height.saturating_sub(1));

        for sample_y in y_start..=y_end {
            for sample_x in x_start..=x_end {
                let idx = ((sample_y * image.width + sample_x) * 4) as usize;
                let pixel = &image.pixels[idx..idx + 4];
                if pixel[3] > 0 && pixel[0] < 80 && pixel[1] < 80 && pixel[2] < 80 {
                    return true;
                }
            }
        }

        false
    }

    #[test]
    fn test_reduce_line_points_preserves_monotonic_envelope() {
        let points = vec![
            Point2f::new(0.1, 10.0),
            Point2f::new(0.2, 2.0),
            Point2f::new(0.3, 8.0),
            Point2f::new(0.4, 4.0),
            Point2f::new(1.2, 5.0),
            Point2f::new(1.3, 1.0),
            Point2f::new(1.4, 9.0),
            Point2f::new(1.5, 6.0),
            Point2f::new(2.2, 3.0),
            Point2f::new(2.3, 7.0),
            Point2f::new(2.4, 2.0),
            Point2f::new(2.5, 8.0),
        ];

        let reduced = reduce_line_points_for_raster(&points, 0.0, 2.0).expect("expected reduction");

        assert!(reduced.len() < points.len());
        assert_eq!(reduced.first().copied(), points.first().copied());
        assert_eq!(reduced.last().copied(), points.last().copied());
        assert!(
            reduced
                .iter()
                .any(|point| (point.y - 1.0).abs() < f32::EPSILON)
        );
        assert!(
            reduced
                .iter()
                .any(|point| (point.y - 9.0).abs() < f32::EPSILON)
        );
    }

    #[test]
    fn test_reduce_line_points_rejects_non_monotonic_x() {
        let points = vec![
            Point2f::new(0.0, 0.0),
            Point2f::new(1.0, 1.0),
            Point2f::new(0.5, 2.0),
            Point2f::new(1.5, 3.0),
            Point2f::new(2.0, 4.0),
            Point2f::new(2.5, 5.0),
            Point2f::new(3.0, 6.0),
            Point2f::new(3.5, 7.0),
            Point2f::new(4.0, 8.0),
        ];

        assert!(reduce_line_points_for_raster(&points, 0.0, 1.0).is_none());
    }

    #[test]
    fn test_exact_line_canonicalization_removes_duplicate_and_collinear_points() {
        let points = vec![
            Point2f::new(0.0, 0.0),
            Point2f::new(1.0, 1.0),
            Point2f::new(2.0, 2.0),
            Point2f::new(2.0, 2.0),
            Point2f::new(3.0, 3.0),
            Point2f::new(3.0, 4.0),
        ];

        let canonical = canonicalize_line_points_exact(&points).expect("expected canonicalization");

        assert_eq!(
            canonical,
            vec![
                Point2f::new(0.0, 0.0),
                Point2f::new(3.0, 3.0),
                Point2f::new(3.0, 4.0),
            ]
        );
    }

    #[test]
    fn test_exact_line_canonicalization_preserves_turns() {
        let points = vec![
            Point2f::new(0.0, 0.0),
            Point2f::new(1.0, 1.0),
            Point2f::new(1.0, 1.0),
            Point2f::new(2.0, 1.0),
            Point2f::new(3.0, 1.0),
        ];

        let canonical = canonicalize_line_points_exact(&points).expect("expected dedupe");

        assert_eq!(
            canonical,
            vec![
                Point2f::new(0.0, 0.0),
                Point2f::new(1.0, 1.0),
                Point2f::new(3.0, 1.0),
            ]
        );
    }

    #[test]
    fn test_reduce_line_points_matches_original_raster_with_small_deviation() {
        let plot_left = 20.0;
        let plot_top = 20.0;
        let plot_width = 640.0;
        let plot_height = 180.0;
        let samples_per_column = 10usize;
        let clip_rect = (plot_left, plot_top, plot_width, plot_height);
        let mut points = Vec::with_capacity(plot_width as usize * samples_per_column);

        for column in 0..plot_width as usize {
            for sample in 0..samples_per_column {
                let phase = (column * samples_per_column + sample) as f32;
                let x = plot_left + column as f32 + sample as f32 / samples_per_column as f32;
                let mut y =
                    plot_top + plot_height * 0.52 + phase.sin() * 18.0 + (phase / 7.0).cos() * 11.0;

                if column % 61 == 17 && sample == samples_per_column / 2 {
                    y = plot_top + 8.0;
                } else if column % 79 == 41 && sample == samples_per_column / 3 {
                    y = plot_top + plot_height - 8.0;
                }

                y = y.clamp(plot_top + 2.0, plot_top + plot_height - 2.0);
                points.push(Point2f::new(x, y));
            }
        }

        let reduced = reduce_line_points_for_raster(&points, plot_left, plot_width)
            .expect("expected point reduction for dense monotonic line");

        let original_image = render_polyline(&points, clip_rect);
        let reduced_image = render_polyline(&reduced, clip_rect);
        let diff = mean_normalized_channel_diff(&original_image, &reduced_image);

        assert!(
            diff <= 0.01,
            "reduced line render deviated too much from original: {diff:.6}"
        );
        assert!(
            image_has_dark_pixel_near(
                &reduced_image,
                (plot_left + 17.0).round() as u32,
                (plot_top + 8.0).round() as u32,
                4,
            ),
            "reduced line should preserve top spike detail"
        );
        assert!(
            image_has_dark_pixel_near(
                &reduced_image,
                (plot_left + 41.0).round() as u32,
                (plot_top + plot_height - 8.0).round() as u32,
                4,
            ),
            "reduced line should preserve bottom spike detail"
        );
    }
}
