use std::env;
use std::hint::black_box;
use std::time::Instant;

use ruviz::prelude::*;

enum Case {
    Line(usize),
    Scatter(usize),
    Histogram(usize),
    Heatmap(usize, usize),
}

enum Operation {
    Render,
    Png,
}

fn parse_args() -> (Case, Operation, usize) {
    let mut args = env::args().skip(1);
    let case = match args.next().as_deref() {
        Some("line") => Case::Line(args.next().and_then(|v| v.parse().ok()).unwrap_or(100_000)),
        Some("scatter") => {
            Case::Scatter(args.next().and_then(|v| v.parse().ok()).unwrap_or(100_000))
        }
        Some("histogram") => Case::Histogram(
            args.next()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1_000_000),
        ),
        Some("heatmap") => {
            let rows = args.next().and_then(|v| v.parse().ok()).unwrap_or(1024);
            let cols = args.next().and_then(|v| v.parse().ok()).unwrap_or(rows);
            Case::Heatmap(rows, cols)
        }
        _ => Case::Scatter(100_000),
    };

    let operation = match args.next().as_deref() {
        Some("render") => Operation::Render,
        Some("png") => Operation::Png,
        _ => Operation::Png,
    };

    let iterations = args.next().and_then(|v| v.parse().ok()).unwrap_or(40);
    (case, operation, iterations)
}

fn triangle_wave(index: usize, period: usize) -> f64 {
    let phase = (index % period) as f64 / period as f64;
    1.0 - 4.0 * (phase - 0.5).abs()
}

fn build_line(points: usize) -> Plot {
    let divisor = points.saturating_sub(1).max(1) as f64;
    let mut x = Vec::with_capacity(points);
    let mut y = Vec::with_capacity(points);
    for index in 0..points {
        x.push(index as f64 * (200.0 / divisor));
        y.push(
            triangle_wave(index, 1024)
                + 0.35 * triangle_wave(index, 257)
                + 0.1 * triangle_wave(index, 61),
        );
    }
    Plot::new()
        .size_px(640, 480)
        .dpi(100)
        .line(&x, &y)
        .into_plot()
}

fn build_scatter(points: usize) -> Plot {
    let modulus = 2_147_483_647_u64;
    let mut x = Vec::with_capacity(points);
    let mut y = Vec::with_capacity(points);
    for index in 0..points {
        let x_raw = (index as u64 * 48_271) % modulus;
        let noise_raw = (index as u64 * 69_621 + 12_345) % modulus;
        let x_value = x_raw as f64 / modulus as f64;
        let noise = noise_raw as f64 / modulus as f64;
        let band = (index % 11) as f64 / 10.0 - 0.5;
        x.push(x_value);
        y.push((0.62 * x_value + 0.25 * noise + 0.13 * band).clamp(0.0, 1.0));
    }
    Plot::new()
        .size_px(640, 480)
        .dpi(100)
        .scatter(&x, &y)
        .into_plot()
}

fn build_histogram(samples: usize) -> Plot {
    let modulus = 2_147_483_647_u64;
    let mut values = Vec::with_capacity(samples);
    for index in 0..samples {
        let a = ((index as u64 * 1_103_515_245 + 12_345) % modulus) as f64 / modulus as f64;
        let b = ((index as u64 * 214_013 + 2_531_011) % modulus) as f64 / modulus as f64;
        let cluster = (index % 17) as f64 / 16.0;
        values.push((0.55 * a + 0.35 * b + 0.10 * cluster) * 10.0 - 5.0);
    }
    Plot::new()
        .size_px(640, 480)
        .dpi(100)
        .histogram(&values, None)
        .into_plot()
}

fn build_heatmap(rows: usize, cols: usize) -> Plot {
    let mut matrix = vec![vec![0.0; cols]; rows];
    for (row_index, row) in matrix.iter_mut().enumerate() {
        for (col_index, value) in row.iter_mut().enumerate() {
            let row_wave = triangle_wave(row_index, 79);
            let col_wave = triangle_wave(col_index, 113);
            let diagonal_wave = triangle_wave(row_index * 3 + col_index * 5, 47);
            *value = row_wave * col_wave + 0.2 * diagonal_wave;
        }
    }
    Plot::new()
        .size_px(640, 640)
        .dpi(100)
        .heatmap(&matrix, None)
        .into_plot()
}

fn main() -> Result<()> {
    let (case, operation, iterations) = parse_args();
    let plot = match case {
        Case::Line(points) => {
            println!("Profiling line with {points} points");
            build_line(points)
        }
        Case::Scatter(points) => {
            println!("Profiling scatter with {points} points");
            build_scatter(points)
        }
        Case::Histogram(samples) => {
            println!("Profiling histogram with {samples} samples");
            build_histogram(samples)
        }
        Case::Heatmap(rows, cols) => {
            println!("Profiling heatmap with {rows}x{cols} cells");
            build_heatmap(rows, cols)
        }
    };

    match operation {
        Operation::Render => println!("Operation: render()"),
        Operation::Png => println!("Operation: render_png_bytes()"),
    }
    println!("Iterations: {iterations}");

    match operation {
        Operation::Render => {
            let _ = plot.render()?;
        }
        Operation::Png => {
            let _ = plot.render_png_bytes()?;
        }
    }

    let start = Instant::now();
    for _ in 0..iterations {
        match operation {
            Operation::Render => {
                black_box(plot.render()?);
            }
            Operation::Png => {
                black_box(plot.render_png_bytes()?);
            }
        }
    }

    let elapsed = start.elapsed();
    println!(
        "Total elapsed: {:.2} ms ({:.2} ms / iteration)",
        elapsed.as_secs_f64() * 1000.0,
        elapsed.as_secs_f64() * 1000.0 / iterations as f64
    );
    Ok(())
}
