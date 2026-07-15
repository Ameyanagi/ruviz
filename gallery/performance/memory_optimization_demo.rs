use ruviz::prelude::*;
use ruviz::render::PooledRenderer;

fn main() -> Result<()> {
    const POINTS: usize = 100_000;
    let x_data: Vec<f64> = (0..POINTS).map(|i| i as f64 * 0.001).collect();
    let y_data: Vec<f64> = x_data.iter().map(|value| (value * 0.5).sin()).collect();

    println!("=== Explicit pooled coordinate transformation ===");
    let renderer = PooledRenderer::with_pool_sizes(POINTS, 16, 16);

    let first_capacity = transform_once(&renderer, &x_data, &y_data, "first")?;
    let after_first = renderer.get_pool_stats();
    println!(
        "after return: {} f32 buffers available, {} values of retained capacity",
        after_first.f32_pool_stats.available_count, after_first.f32_pool_stats.total_capacity
    );
    assert_eq!(after_first.f32_pool_stats.in_use_count, 0);
    assert_eq!(after_first.f32_pool_stats.available_count, 2);

    let second_capacity = transform_once(&renderer, &x_data, &y_data, "second")?;
    assert_eq!(
        first_capacity, second_capacity,
        "the second pass should reuse the retained coordinate buffers"
    );

    // This normal Plot render is useful output, but it is deliberately reported
    // separately: its internal allocations are not the PooledRenderer statistics above.
    let image = Plot::new()
        .line(&x_data, &y_data)
        .title("High-level render (separate from pooled transform stats)")
        .xlabel("X")
        .ylabel("sin(X / 2)")
        .render()?;
    println!(
        "high-level Plot render produced a {}x{} image",
        image.width, image.height
    );

    Ok(())
}

fn transform_once(
    renderer: &PooledRenderer,
    x_data: &[f64],
    y_data: &[f64],
    label: &str,
) -> Result<usize> {
    let started = std::time::Instant::now();
    let (screen_x, screen_y) = renderer.transform_coordinates_pooled(
        &x_data,
        &y_data,
        x_data[0],
        x_data[x_data.len() - 1],
        -1.0,
        1.0,
        60.0,
        20.0,
        780.0,
        580.0,
    )?;

    assert_eq!(screen_x.len(), x_data.len());
    assert_eq!(screen_y.len(), y_data.len());
    std::hint::black_box((&screen_x, &screen_y));

    let stats = renderer.get_pool_stats();
    println!(
        "{label} pass: transformed {} points in {:?}; {} f32 buffers in use",
        x_data.len(),
        started.elapsed(),
        stats.f32_pool_stats.in_use_count
    );
    assert_eq!(stats.f32_pool_stats.in_use_count, 2);

    Ok(stats.f32_pool_stats.total_capacity)
}
