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
}
