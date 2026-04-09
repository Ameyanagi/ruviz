use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use anyhow::{Result, anyhow};
use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};
use plotters::coord::Shift;
use plotters::prelude::*;
use ruviz::prelude::{IntoPlot, Plot};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    defaults: ManifestDefaults,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestDefaults {
    warmup_iterations: usize,
    measured_iterations: usize,
    smoke_warmup_iterations: usize,
    smoke_measured_iterations: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Scenario {
    id: String,
    plot_kind: String,
    dataset_kind: String,
    canvas: CanvasSpec,
    sizes: Vec<SizeSpec>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CanvasSpec {
    width: u32,
    height: u32,
    dpi: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SizeSpec {
    label: String,
    points: Option<usize>,
    samples: Option<usize>,
    rows: Option<usize>,
    cols: Option<usize>,
}

#[derive(Debug, Clone)]
struct ScenarioRun {
    scenario_id: String,
    plot_kind: String,
    dataset_kind: String,
    canvas: CanvasSpec,
    size: SizeSpec,
    warmup_iterations: usize,
    measured_iterations: usize,
    elements: usize,
}

#[derive(Debug, Clone, Copy)]
struct NumericRange {
    min: f64,
    max: f64,
}

#[derive(Debug, Clone)]
struct HistogramBin {
    start: f64,
    end: f64,
    count: u32,
}

#[derive(Debug)]
enum Dataset {
    Line {
        x: Vec<f64>,
        y: Vec<f64>,
        hash: String,
        x_range: NumericRange,
        y_range: NumericRange,
    },
    Scatter {
        x: Vec<f64>,
        y: Vec<f64>,
        hash: String,
        x_range: NumericRange,
        y_range: NumericRange,
    },
    Histogram {
        values: Vec<f64>,
        hash: String,
        x_range: NumericRange,
        bins: Vec<HistogramBin>,
        max_bin_count: u32,
    },
    Heatmap {
        matrix: Vec<Vec<f64>>,
        hash: String,
        value_range: NumericRange,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BenchmarkPayload {
    schema_version: u32,
    runtime: String,
    environment: RuntimeEnvironment,
    results: Vec<BenchmarkResult>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeEnvironment {
    rust_version: String,
    ruviz_version: String,
    build_profile: String,
    feature_label: String,
    cargo_features: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BenchmarkResult {
    implementation: String,
    scenario_id: String,
    plot_kind: String,
    size_label: String,
    boundary: String,
    output_target: String,
    elements: usize,
    canvas: CanvasSpec,
    dataset_hash: String,
    warmup_iterations: usize,
    measured_iterations: usize,
    byte_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    actual_backend: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    render_diagnostics: Option<RenderDiagnosticsRecord>,
    iterations_ms: Vec<f64>,
    summary: Summary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct RenderDiagnosticsRecord {
    render_mode: String,
    used_parallel: bool,
    used_auto_datashader: bool,
    used_exact_line_canonicalization: bool,
    used_raster_line_reduction: bool,
    used_marker_path_cache: bool,
    used_marker_sprite_cache: bool,
    used_marker_sprite_compositor: bool,
    used_marker_sprite_fallback: bool,
    used_circle_scanline_blit: bool,
    used_direct_rect_fill: bool,
    used_pixel_aligned_rect_fill: bool,
    used_prepared_geometry_cache: bool,
    rebuilt_prepared_geometry_cache: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Summary {
    mean_ms: f64,
    median_ms: f64,
    p95_ms: f64,
    min_ms: f64,
    max_ms: f64,
    stdev_ms: f64,
    throughput_elements_per_sec: f64,
}

fn main() -> Result<()> {
    let args = Args::parse()?;
    let manifest: Manifest = serde_json::from_str(&fs::read_to_string(&args.manifest)?)?;
    let runs = scenario_runs(&manifest, &args.mode);
    let mut results = Vec::with_capacity(runs.len() * 5);
    for run in &runs {
        results.extend(benchmark_run(run, &args)?);
    }

    let payload = BenchmarkPayload {
        schema_version: 1,
        runtime: "rust".to_string(),
        environment: RuntimeEnvironment {
            rust_version: rust_version(),
            ruviz_version: env!("CARGO_PKG_VERSION").to_string(),
            build_profile: if cfg!(debug_assertions) {
                "debug".to_string()
            } else {
                "release".to_string()
            },
            feature_label: args
                .feature_label
                .clone()
                .unwrap_or_else(detect_feature_label),
            cargo_features: args
                .cargo_features
                .clone()
                .unwrap_or_else(detect_compiled_features),
        },
        results,
    };

    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&args.output, serde_json::to_string_pretty(&payload)?)?;
    Ok(())
}

#[derive(Debug)]
struct Args {
    manifest: PathBuf,
    mode: String,
    output: PathBuf,
    feature_label: Option<String>,
    cargo_features: Option<Vec<String>>,
    include_save_path: bool,
    skip_plotters: bool,
    request_gpu: bool,
}

impl Args {
    fn parse() -> Result<Self> {
        let mut manifest = None;
        let mut mode = Some("full".to_string());
        let mut output = None;
        let mut feature_label = None;
        let mut cargo_features = None;
        let mut include_save_path = false;
        let mut skip_plotters = false;
        let mut request_gpu = false;

        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--manifest" => manifest = args.next().map(PathBuf::from),
                "--mode" => mode = args.next(),
                "--output" => output = args.next().map(PathBuf::from),
                "--feature-label" => feature_label = args.next(),
                "--cargo-features" => {
                    cargo_features = args.next().map(|value| {
                        value
                            .split(',')
                            .filter(|feature| !feature.is_empty())
                            .map(ToString::to_string)
                            .collect()
                    })
                }
                "--include-save-path" => include_save_path = true,
                "--skip-plotters" => skip_plotters = true,
                "--request-gpu" => request_gpu = true,
                other => {
                    return Err(anyhow!(
                        "unexpected argument: {other}. expected --manifest, --mode, --output, --feature-label, --cargo-features, --include-save-path, --skip-plotters, or --request-gpu"
                    ));
                }
            }
        }

        Ok(Self {
            manifest: manifest.ok_or_else(|| anyhow!("missing --manifest"))?,
            mode: mode.ok_or_else(|| anyhow!("missing --mode"))?,
            output: output.ok_or_else(|| anyhow!("missing --output"))?,
            feature_label,
            cargo_features,
            include_save_path,
            skip_plotters,
            request_gpu,
        })
    }
}

fn scenario_runs(manifest: &Manifest, mode: &str) -> Vec<ScenarioRun> {
    let (warmup_iterations, measured_iterations) = if mode == "smoke" {
        (
            manifest.defaults.smoke_warmup_iterations,
            manifest.defaults.smoke_measured_iterations,
        )
    } else {
        (
            manifest.defaults.warmup_iterations,
            manifest.defaults.measured_iterations,
        )
    };

    let mut runs = Vec::new();
    for scenario in &manifest.scenarios {
        let sizes: &[SizeSpec] = if mode == "smoke" {
            &scenario.sizes[..1]
        } else {
            &scenario.sizes
        };
        for size in sizes {
            runs.push(ScenarioRun {
                scenario_id: scenario.id.clone(),
                plot_kind: scenario.plot_kind.clone(),
                dataset_kind: scenario.dataset_kind.clone(),
                canvas: size_canvas(&scenario.canvas),
                size: size.clone(),
                warmup_iterations,
                measured_iterations,
                elements: element_count(size),
            });
        }
    }
    runs
}

fn size_canvas(canvas: &CanvasSpec) -> CanvasSpec {
    CanvasSpec {
        width: canvas.width,
        height: canvas.height,
        dpi: canvas.dpi,
    }
}

fn element_count(size: &SizeSpec) -> usize {
    if let Some(points) = size.points {
        return points;
    }
    if let Some(samples) = size.samples {
        return samples;
    }
    size.rows.unwrap_or(0) * size.cols.unwrap_or(0)
}

fn benchmark_run(run: &ScenarioRun, args: &Args) -> Result<Vec<BenchmarkResult>> {
    let dataset = build_dataset(run)?;
    let built_plot = build_plot(run, &dataset);

    let (render_only_ms, render_only_bytes, render_only_diagnostics) =
        measure_with_diagnostics(run.warmup_iterations, run.measured_iterations, || {
            Ok(built_plot.benchmark_render_png_bytes_with_diagnostics()?)
        })?;

    let (public_ms, public_bytes, public_diagnostics) =
        measure_with_diagnostics(run.warmup_iterations, run.measured_iterations, || {
            Ok(build_plot(run, &dataset).benchmark_render_png_bytes_with_diagnostics()?)
        })?;

    let mut results = vec![
        make_result(
            run,
            &dataset,
            "ruviz",
            "render_only",
            render_only_bytes,
            render_only_ms,
            None,
            Some(render_only_diagnostics),
        ),
        make_result(
            run,
            &dataset,
            "ruviz",
            "public_api_render",
            public_bytes,
            public_ms,
            None,
            Some(public_diagnostics),
        ),
    ];

    if args.include_save_path {
        let built_save_plot = build_plot_with_options(run, &dataset, args.request_gpu);
        let (save_only_ms, save_only_bytes, save_backend, save_diagnostics) =
            measure_with_backend_and_diagnostics(
                run.warmup_iterations,
                run.measured_iterations,
                || Ok(built_save_plot.benchmark_save_png_bytes_with_diagnostics()?),
            )?;
        let (public_save_ms, public_save_bytes, public_save_backend, public_save_diagnostics) =
            measure_with_backend_and_diagnostics(
                run.warmup_iterations,
                run.measured_iterations,
                || {
                    Ok(build_plot_with_options(run, &dataset, args.request_gpu)
                        .benchmark_save_png_bytes_with_diagnostics()?)
                },
            )?;

        results.push(make_result(
            run,
            &dataset,
            "ruviz",
            "save_only",
            save_only_bytes,
            save_only_ms,
            Some(save_backend),
            Some(save_diagnostics),
        ));
        results.push(make_result(
            run,
            &dataset,
            "ruviz",
            "public_api_save",
            public_save_bytes,
            public_save_ms,
            Some(public_save_backend),
            Some(public_save_diagnostics),
        ));
    }

    if !args.skip_plotters {
        let (plotters_ms, plotters_bytes) =
            measure(run.warmup_iterations, run.measured_iterations, || {
                render_plotters_png(run, &dataset)
            })?;
        results.push(make_result(
            run,
            &dataset,
            "plotters",
            "public_api_render",
            plotters_bytes,
            plotters_ms,
            None,
            None,
        ));
    }

    Ok(results)
}

fn make_result(
    run: &ScenarioRun,
    dataset: &Dataset,
    implementation: &str,
    boundary: &str,
    byte_count: usize,
    iterations_ms: Vec<f64>,
    actual_backend: Option<String>,
    render_diagnostics: Option<RenderDiagnosticsRecord>,
) -> BenchmarkResult {
    BenchmarkResult {
        implementation: implementation.to_string(),
        scenario_id: run.scenario_id.clone(),
        plot_kind: run.plot_kind.clone(),
        size_label: run.size.label.clone(),
        boundary: boundary.to_string(),
        output_target: "png_bytes".to_string(),
        elements: run.elements,
        canvas: size_canvas(&run.canvas),
        dataset_hash: dataset_hash(dataset).to_string(),
        warmup_iterations: run.warmup_iterations,
        measured_iterations: run.measured_iterations,
        byte_count,
        actual_backend,
        render_diagnostics,
        summary: summarize_iterations(&iterations_ms, run.elements),
        iterations_ms,
    }
}

fn measure<F>(
    warmup_iterations: usize,
    measured_iterations: usize,
    mut f: F,
) -> Result<(Vec<f64>, usize)>
where
    F: FnMut() -> Result<Vec<u8>>,
{
    for _ in 0..warmup_iterations {
        let _ = f()?;
    }

    let mut iterations_ms = Vec::with_capacity(measured_iterations);
    let mut last_len = 0;
    for _ in 0..measured_iterations {
        let start = Instant::now();
        let bytes = f()?;
        iterations_ms.push(start.elapsed().as_secs_f64() * 1000.0);
        last_len = bytes.len();
    }
    Ok((iterations_ms, last_len))
}

fn measure_with_diagnostics<F>(
    warmup_iterations: usize,
    measured_iterations: usize,
    mut f: F,
) -> Result<(Vec<f64>, usize, RenderDiagnosticsRecord)>
where
    F: FnMut() -> Result<(Vec<u8>, ruviz::core::plot::RenderDiagnostics)>,
{
    let mut diagnostics = None;
    for _ in 0..warmup_iterations {
        let (_, current_diagnostics) = f()?;
        record_diagnostics(
            &mut diagnostics,
            RenderDiagnosticsRecord::from(current_diagnostics),
        )?;
    }

    let mut iterations_ms = Vec::with_capacity(measured_iterations);
    let mut last_len = 0;
    for _ in 0..measured_iterations {
        let start = Instant::now();
        let (bytes, current_diagnostics) = f()?;
        record_diagnostics(
            &mut diagnostics,
            RenderDiagnosticsRecord::from(current_diagnostics),
        )?;
        iterations_ms.push(start.elapsed().as_secs_f64() * 1000.0);
        last_len = bytes.len();
    }

    Ok((
        iterations_ms,
        last_len,
        diagnostics.unwrap_or_else(|| RenderDiagnosticsRecord {
            render_mode: "unknown".to_string(),
            used_parallel: false,
            used_auto_datashader: false,
            used_exact_line_canonicalization: false,
            used_raster_line_reduction: false,
            used_marker_path_cache: false,
            used_marker_sprite_cache: false,
            used_marker_sprite_compositor: false,
            used_marker_sprite_fallback: false,
            used_circle_scanline_blit: false,
            used_direct_rect_fill: false,
            used_pixel_aligned_rect_fill: false,
            used_prepared_geometry_cache: false,
            rebuilt_prepared_geometry_cache: false,
        }),
    ))
}

fn measure_with_backend_and_diagnostics<F>(
    warmup_iterations: usize,
    measured_iterations: usize,
    mut f: F,
) -> Result<(Vec<f64>, usize, String, RenderDiagnosticsRecord)>
where
    F: FnMut() -> Result<(Vec<u8>, &'static str, ruviz::core::plot::RenderDiagnostics)>,
{
    let mut backend = None;
    let mut diagnostics = None;
    for _ in 0..warmup_iterations {
        let (_, current_backend, current_diagnostics) = f()?;
        record_backend(&mut backend, current_backend)?;
        record_diagnostics(
            &mut diagnostics,
            RenderDiagnosticsRecord::from(current_diagnostics),
        )?;
    }

    let mut iterations_ms = Vec::with_capacity(measured_iterations);
    let mut last_len = 0;
    for _ in 0..measured_iterations {
        let start = Instant::now();
        let (bytes, current_backend, current_diagnostics) = f()?;
        record_backend(&mut backend, current_backend)?;
        record_diagnostics(
            &mut diagnostics,
            RenderDiagnosticsRecord::from(current_diagnostics),
        )?;
        iterations_ms.push(start.elapsed().as_secs_f64() * 1000.0);
        last_len = bytes.len();
    }

    Ok((
        iterations_ms,
        last_len,
        backend.unwrap_or_else(|| "unknown".to_string()),
        diagnostics.unwrap_or_else(|| RenderDiagnosticsRecord {
            render_mode: "unknown".to_string(),
            used_parallel: false,
            used_auto_datashader: false,
            used_exact_line_canonicalization: false,
            used_raster_line_reduction: false,
            used_marker_path_cache: false,
            used_marker_sprite_cache: false,
            used_marker_sprite_compositor: false,
            used_marker_sprite_fallback: false,
            used_circle_scanline_blit: false,
            used_direct_rect_fill: false,
            used_pixel_aligned_rect_fill: false,
            used_prepared_geometry_cache: false,
            rebuilt_prepared_geometry_cache: false,
        }),
    ))
}

fn record_backend(seen: &mut Option<String>, current: &str) -> Result<()> {
    match seen {
        Some(previous) if previous != current => Err(anyhow!(
            "save backend changed within a single benchmark case: {previous} -> {current}"
        )),
        Some(_) => Ok(()),
        None => {
            *seen = Some(current.to_string());
            Ok(())
        }
    }
}

fn record_diagnostics(
    seen: &mut Option<RenderDiagnosticsRecord>,
    current: RenderDiagnosticsRecord,
) -> Result<()> {
    match seen {
        Some(previous) if previous != &current => Err(anyhow!(
            "render diagnostics changed within a single benchmark case: {previous:?} -> {current:?}"
        )),
        Some(_) => Ok(()),
        None => {
            *seen = Some(current);
            Ok(())
        }
    }
}

impl From<ruviz::core::plot::RenderDiagnostics> for RenderDiagnosticsRecord {
    fn from(value: ruviz::core::plot::RenderDiagnostics) -> Self {
        Self {
            render_mode: value.render_mode.to_string(),
            used_parallel: value.used_parallel,
            used_auto_datashader: value.used_auto_datashader,
            used_exact_line_canonicalization: value.used_exact_line_canonicalization,
            used_raster_line_reduction: value.used_raster_line_reduction,
            used_marker_path_cache: value.used_marker_path_cache,
            used_marker_sprite_cache: value.used_marker_sprite_cache,
            used_marker_sprite_compositor: value.used_marker_sprite_compositor,
            used_marker_sprite_fallback: value.used_marker_sprite_fallback,
            used_circle_scanline_blit: value.used_circle_scanline_blit,
            used_direct_rect_fill: value.used_direct_rect_fill,
            used_pixel_aligned_rect_fill: value.used_pixel_aligned_rect_fill,
            used_prepared_geometry_cache: value.used_prepared_geometry_cache,
            rebuilt_prepared_geometry_cache: value.rebuilt_prepared_geometry_cache,
        }
    }
}

fn build_plot(run: &ScenarioRun, dataset: &Dataset) -> Plot {
    build_plot_with_options(run, dataset, false)
}

fn build_plot_with_options(run: &ScenarioRun, dataset: &Dataset, request_gpu: bool) -> Plot {
    let base = Plot::new()
        .size_px(run.canvas.width, run.canvas.height)
        .dpi(run.canvas.dpi);
    let base = maybe_enable_gpu(base, request_gpu);

    match dataset {
        Dataset::Line { x, y, .. } => base.line(x, y).into_plot(),
        Dataset::Scatter { x, y, .. } => base.scatter(x, y).into_plot(),
        Dataset::Histogram { values, .. } => base.histogram(values, None).into_plot(),
        Dataset::Heatmap { matrix, .. } => base.heatmap(matrix, None).into_plot(),
    }
}

fn maybe_enable_gpu(plot: Plot, request_gpu: bool) -> Plot {
    #[cfg(feature = "gpu")]
    {
        if request_gpu { plot.gpu(true) } else { plot }
    }

    #[cfg(not(feature = "gpu"))]
    {
        let _ = request_gpu;
        plot
    }
}

fn build_dataset(run: &ScenarioRun) -> Result<Dataset> {
    match run.dataset_kind.as_str() {
        "line_wave" => Ok(line_wave(run.size.points.ok_or_else(|| {
            anyhow!("line scenario is missing points in size entry")
        })?)),
        "scatter_cloud" => {
            Ok(scatter_cloud(run.size.points.ok_or_else(|| {
                anyhow!("scatter scenario is missing points in size entry")
            })?))
        }
        "histogram_signal" => {
            Ok(histogram_signal(run.size.samples.ok_or_else(|| {
                anyhow!("histogram scenario is missing samples in size entry")
            })?))
        }
        "heatmap_field" => Ok(heatmap_field(
            run.size
                .rows
                .ok_or_else(|| anyhow!("heatmap scenario is missing rows in size entry"))?,
            run.size
                .cols
                .ok_or_else(|| anyhow!("heatmap scenario is missing cols in size entry"))?,
        )),
        other => Err(anyhow!("unsupported dataset kind: {other}")),
    }
}

fn dataset_hash(dataset: &Dataset) -> &str {
    match dataset {
        Dataset::Line { hash, .. }
        | Dataset::Scatter { hash, .. }
        | Dataset::Histogram { hash, .. }
        | Dataset::Heatmap { hash, .. } => hash,
    }
}

fn render_plotters_png(run: &ScenarioRun, dataset: &Dataset) -> Result<Vec<u8>> {
    let width = run.canvas.width;
    let height = run.canvas.height;
    let mut rgb = vec![0_u8; width as usize * height as usize * 3];

    {
        let root = BitMapBackend::with_buffer(&mut rgb, (width, height)).into_drawing_area();
        root.fill(&WHITE).map_err(plotters_error)?;

        match dataset {
            Dataset::Line {
                x,
                y,
                x_range,
                y_range,
                ..
            } => draw_plotters_line(&root, x, y, *x_range, *y_range)?,
            Dataset::Scatter {
                x,
                y,
                x_range,
                y_range,
                ..
            } => draw_plotters_scatter(&root, x, y, *x_range, *y_range)?,
            Dataset::Histogram {
                x_range,
                bins,
                max_bin_count,
                ..
            } => draw_plotters_histogram(&root, *x_range, bins, *max_bin_count)?,
            Dataset::Heatmap {
                matrix,
                value_range,
                ..
            } => draw_plotters_heatmap(&root, matrix, *value_range, width, height)?,
        }

        root.present().map_err(plotters_error)?;
    }

    encode_rgb_png(width, height, &rgb)
}

fn draw_plotters_line(
    root: &DrawingArea<BitMapBackend<'_>, Shift>,
    x: &[f64],
    y: &[f64],
    x_range: NumericRange,
    y_range: NumericRange,
) -> Result<()> {
    let mut chart = base_plotters_chart(root, x_range, y_range)?;
    chart
        .draw_series(LineSeries::new(
            x.iter().copied().zip(y.iter().copied()),
            ShapeStyle::from(&BLUE).stroke_width(1),
        ))
        .map_err(plotters_error)?;
    Ok(())
}

fn draw_plotters_scatter(
    root: &DrawingArea<BitMapBackend<'_>, Shift>,
    x: &[f64],
    y: &[f64],
    x_range: NumericRange,
    y_range: NumericRange,
) -> Result<()> {
    let mut chart = base_plotters_chart(root, x_range, y_range)?;
    chart
        .draw_series(
            x.iter()
                .copied()
                .zip(y.iter().copied())
                .map(|point| Circle::new(point, 1, BLUE.filled())),
        )
        .map_err(plotters_error)?;
    Ok(())
}

fn draw_plotters_histogram(
    root: &DrawingArea<BitMapBackend<'_>, Shift>,
    x_range: NumericRange,
    bins: &[HistogramBin],
    max_bin_count: u32,
) -> Result<()> {
    let y_range = 0.0..(max_bin_count.max(1) as f64 * 1.05);
    let mut chart = ChartBuilder::on(root)
        .margin(4)
        .set_all_label_area_size(0)
        .build_cartesian_2d(x_range.min..x_range.max, y_range)
        .map_err(plotters_error)?;
    chart
        .configure_mesh()
        .disable_mesh()
        .x_labels(0)
        .y_labels(0)
        .draw()
        .map_err(plotters_error)?;
    chart
        .draw_series(bins.iter().map(|bin| {
            Rectangle::new(
                [(bin.start, 0.0), (bin.end, bin.count as f64)],
                RGBColor(31, 119, 180).filled(),
            )
        }))
        .map_err(plotters_error)?;
    Ok(())
}

fn draw_plotters_heatmap(
    root: &DrawingArea<BitMapBackend<'_>, Shift>,
    matrix: &[Vec<f64>],
    value_range: NumericRange,
    width: u32,
    height: u32,
) -> Result<()> {
    let rows = matrix.len();
    let cols = matrix.first().map_or(0, Vec::len);
    if rows == 0 || cols == 0 {
        return Ok(());
    }

    for pixel_y in 0..height as usize {
        let source_row = pixel_y * rows / height as usize;
        let row = &matrix[source_row];
        for pixel_x in 0..width as usize {
            let source_col = pixel_x * cols / width as usize;
            let color = heatmap_color(row[source_col], value_range);
            root.draw_pixel((pixel_x as i32, pixel_y as i32), &color)
                .map_err(plotters_error)?;
        }
    }

    Ok(())
}

fn base_plotters_chart<'a, DB: DrawingBackend>(
    root: &'a DrawingArea<DB, Shift>,
    x_range: NumericRange,
    y_range: NumericRange,
) -> Result<
    ChartContext<
        'a,
        DB,
        Cartesian2d<plotters::coord::types::RangedCoordf64, plotters::coord::types::RangedCoordf64>,
    >,
> {
    let mut chart = ChartBuilder::on(root)
        .margin(4)
        .set_all_label_area_size(0)
        .build_cartesian_2d(x_range.min..x_range.max, y_range.min..y_range.max)
        .map_err(plotters_error)?;
    chart
        .configure_mesh()
        .disable_mesh()
        .x_labels(0)
        .y_labels(0)
        .draw()
        .map_err(plotters_error)?;
    Ok(chart)
}

fn encode_rgb_png(width: u32, height: u32, rgb: &[u8]) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes).write_image(rgb, width, height, ColorType::Rgb8.into())?;
    Ok(bytes)
}

fn plotters_error<E: std::fmt::Debug>(error: E) -> anyhow::Error {
    anyhow!("plotters rendering failed: {error:?}")
}

fn triangle_wave(index: usize, period: usize) -> f64 {
    let phase = (index % period) as f64 / period as f64;
    1.0 - 4.0 * (phase - 0.5).abs()
}

fn line_wave(points: usize) -> Dataset {
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
    let hash = hash_f64_arrays([x.as_slice(), y.as_slice()]);
    Dataset::Line {
        x_range: padded_range(extent_from_slice(&x), 0.02),
        y_range: padded_range(extent_from_slice(&y), 0.05),
        x,
        y,
        hash,
    }
}

fn scatter_cloud(points: usize) -> Dataset {
    const MODULUS: u64 = 2_147_483_647;
    let mut x = Vec::with_capacity(points);
    let mut y = Vec::with_capacity(points);
    for index in 0..points {
        let index = index as u64;
        let x_raw = (index * 48_271) % MODULUS;
        let noise_raw = (index * 69_621 + 12_345) % MODULUS;
        let x_value = x_raw as f64 / MODULUS as f64;
        let noise = noise_raw as f64 / MODULUS as f64;
        let band = (index % 11) as f64 / 10.0 - 0.5;
        let y_value = (0.62 * x_value + 0.25 * noise + 0.13 * band).clamp(0.0, 1.0);
        x.push(x_value);
        y.push(y_value);
    }
    let hash = hash_f64_arrays([x.as_slice(), y.as_slice()]);
    Dataset::Scatter {
        x_range: padded_range(extent_from_slice(&x), 0.02),
        y_range: padded_range(extent_from_slice(&y), 0.05),
        x,
        y,
        hash,
    }
}

fn histogram_signal(samples: usize) -> Dataset {
    const MODULUS: u64 = 2_147_483_647;
    let mut values = Vec::with_capacity(samples);
    for index in 0..samples {
        let index = index as u64;
        let a = ((index * 1_103_515_245 + 12_345) % MODULUS) as f64 / MODULUS as f64;
        let b = ((index * 214_013 + 2_531_011) % MODULUS) as f64 / MODULUS as f64;
        let cluster = (index % 17) as f64 / 16.0;
        values.push((0.55 * a + 0.35 * b + 0.10 * cluster) * 10.0 - 5.0);
    }
    let hash = hash_f64_arrays([values.as_slice()]);
    let extent = extent_from_slice(&values);
    let (bins, max_bin_count) = histogram_bins(&values, extent, 256);
    Dataset::Histogram {
        values,
        hash,
        x_range: padded_range(extent, 0.02),
        bins,
        max_bin_count,
    }
}

fn heatmap_field(rows: usize, cols: usize) -> Dataset {
    let mut matrix = vec![vec![0.0; cols]; rows];
    let mut flat = Vec::with_capacity(rows * cols);
    for (row_index, row) in matrix.iter_mut().enumerate() {
        for (col_index, cell) in row.iter_mut().enumerate() {
            let row_wave = triangle_wave(row_index, 79);
            let col_wave = triangle_wave(col_index, 113);
            let diagonal_wave = triangle_wave(row_index * 3 + col_index * 5, 47);
            let value = row_wave * col_wave + 0.2 * diagonal_wave;
            *cell = value;
            flat.push(value);
        }
    }
    let hash = hash_f64_arrays([flat.as_slice()]);
    Dataset::Heatmap {
        value_range: extent_from_slice(&flat),
        matrix,
        hash,
    }
}

fn extent_from_slice(values: &[f64]) -> NumericRange {
    if values.is_empty() {
        return NumericRange {
            min: -1.0,
            max: 1.0,
        };
    }

    let mut min = values[0];
    let mut max = values[0];
    for &value in &values[1..] {
        min = min.min(value);
        max = max.max(value);
    }

    if (max - min).abs() <= f64::EPSILON {
        NumericRange {
            min: min - 1.0,
            max: max + 1.0,
        }
    } else {
        NumericRange { min, max }
    }
}

fn padded_range(range: NumericRange, fraction: f64) -> NumericRange {
    let span = (range.max - range.min).abs();
    let padding = if span <= f64::EPSILON {
        1.0
    } else {
        span * fraction
    };
    NumericRange {
        min: range.min - padding,
        max: range.max + padding,
    }
}

fn histogram_bins(
    values: &[f64],
    range: NumericRange,
    bin_count: usize,
) -> (Vec<HistogramBin>, u32) {
    let span = (range.max - range.min).max(f64::EPSILON);
    let width = span / bin_count as f64;
    let mut counts = vec![0_u32; bin_count];

    for &value in values {
        let mut index = ((value - range.min) / width).floor() as usize;
        if index >= bin_count {
            index = bin_count - 1;
        }
        counts[index] += 1;
    }

    let bins = counts
        .into_iter()
        .enumerate()
        .map(|(index, count)| HistogramBin {
            start: range.min + index as f64 * width,
            end: range.min + (index + 1) as f64 * width,
            count,
        })
        .collect::<Vec<_>>();
    let max_bin_count = bins.iter().map(|bin| bin.count).max().unwrap_or(0);
    (bins, max_bin_count)
}

fn heatmap_color(value: f64, range: NumericRange) -> RGBColor {
    let span = (range.max - range.min).max(f64::EPSILON);
    let t = ((value - range.min) / span).clamp(0.0, 1.0);
    let red = (255.0 * t) as u8;
    let green = (255.0 * (1.0 - (2.0 * (t - 0.5)).abs()).max(0.0)) as u8;
    let blue = (255.0 * (1.0 - t)) as u8;
    RGBColor(red, green, blue)
}

fn hash_f64_arrays<'a, I>(arrays: I) -> String
where
    I: IntoIterator<Item = &'a [f64]>,
{
    let mut digest = Sha256::new();
    for array in arrays {
        for value in array {
            digest.update(value.to_le_bytes());
        }
    }
    format!("{:x}", digest.finalize())
}

fn summarize_iterations(values: &[f64], elements: usize) -> Summary {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| (*value - mean).powi(2))
        .sum::<f64>()
        / (values.len().saturating_sub(1).max(1) as f64);
    let median = percentile(&sorted, 0.5);
    let p95 = percentile(&sorted, 0.95);
    Summary {
        mean_ms: mean,
        median_ms: median,
        p95_ms: p95,
        min_ms: *sorted.first().unwrap_or(&0.0),
        max_ms: *sorted.last().unwrap_or(&0.0),
        stdev_ms: variance.sqrt(),
        throughput_elements_per_sec: if median <= 0.0 {
            0.0
        } else {
            elements as f64 / (median / 1000.0)
        },
    }
}

fn percentile(values: &[f64], ratio: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    if values.len() == 1 {
        return values[0];
    }

    let position = ratio * (values.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        return values[lower];
    }
    let weight = position - lower as f64;
    values[lower] * (1.0 - weight) + values[upper] * weight
}

fn rust_version() -> String {
    String::from_utf8(
        Command::new("rustc")
            .arg("-V")
            .output()
            .expect("rustc should be available")
            .stdout,
    )
    .expect("rustc -V output should be valid utf-8")
    .trim()
    .to_string()
}

fn detect_feature_label() -> String {
    let features = detect_compiled_features();
    if features.is_empty() {
        "baseline_cpu".to_string()
    } else {
        features.join("+")
    }
}

fn detect_compiled_features() -> Vec<String> {
    let mut features = Vec::new();
    if cfg!(feature = "parallel") {
        features.push("parallel".to_string());
    }
    if cfg!(feature = "simd") {
        features.push("simd".to_string());
    }
    if cfg!(feature = "performance") {
        features.push("performance".to_string());
    }
    if cfg!(feature = "gpu") {
        features.push("gpu".to_string());
    }
    if cfg!(feature = "ndarray") {
        features.push("ndarray".to_string());
    }
    if cfg!(feature = "serde") {
        features.push("serde".to_string());
    }
    features
}
