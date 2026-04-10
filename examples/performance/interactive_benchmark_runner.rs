use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Result, anyhow};
use ruviz::core::plot::{
    ImageTarget, InteractiveFrame, PlotInputEvent, SurfaceCapability, SurfaceTarget,
};
use ruviz::data::Signal;
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
    hover_stride_px: u32,
    pan_delta_px: [i32; 2],
    time_step_seconds: f64,
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
    hover_stride_px: u32,
    pan_delta_px: (i32, i32),
    time_step_seconds: f64,
    elements: usize,
}

#[derive(Debug)]
enum Dataset {
    TemporalX { x: Vec<f64>, hash: String },
    Heatmap { matrix: Vec<Vec<f64>>, hash: String },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BenchmarkPayload {
    schema_version: u32,
    suite: String,
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
    frame_byte_count: usize,
    frame_diagnostics: FrameDiagnosticsRecord,
    iterations_ms: Vec<f64>,
    summary: Summary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct FrameDiagnosticsRecord {
    target: String,
    surface_capability: String,
    base_dirty: bool,
    overlay_dirty: bool,
    used_incremental_data: bool,
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

#[derive(Debug)]
struct Args {
    manifest: PathBuf,
    mode: String,
    output: PathBuf,
}

impl Args {
    fn parse() -> Result<Self> {
        let mut manifest = None;
        let mut mode = Some("full".to_string());
        let mut output = None;

        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--manifest" => manifest = args.next().map(PathBuf::from),
                "--mode" => mode = args.next(),
                "--output" => output = args.next().map(PathBuf::from),
                _ => return Err(anyhow!("unexpected argument: {arg}")),
            }
        }

        Ok(Self {
            manifest: manifest.ok_or_else(|| anyhow!("missing --manifest"))?,
            mode: mode.ok_or_else(|| anyhow!("missing --mode"))?,
            output: output.ok_or_else(|| anyhow!("missing --output"))?,
        })
    }
}

#[derive(Clone, Copy)]
enum TargetKind {
    Image,
    Surface,
}

impl TargetKind {
    fn label(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Surface => "surface",
        }
    }

    fn output_target(self) -> &'static str {
        match self {
            Self::Image => "image_frame",
            Self::Surface => "surface_frame",
        }
    }
}

#[derive(Clone, Copy)]
enum ActionKind {
    Hover,
    Pan,
    Time,
}

impl ActionKind {
    fn label(self) -> &'static str {
        match self {
            Self::Hover => "hover",
            Self::Pan => "pan",
            Self::Time => "time",
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse()?;
    let manifest: Manifest = serde_json::from_str(&fs::read_to_string(&args.manifest)?)?;
    let runs = scenario_runs(&manifest, &args.mode);
    let mut results = Vec::new();
    for run in &runs {
        results.extend(benchmark_run(run)?);
    }

    let payload = BenchmarkPayload {
        schema_version: 1,
        suite: "interactive".to_string(),
        runtime: "rust".to_string(),
        environment: RuntimeEnvironment {
            rust_version: rust_version(),
            ruviz_version: env!("CARGO_PKG_VERSION").to_string(),
            build_profile: if cfg!(debug_assertions) {
                "debug".to_string()
            } else {
                "release".to_string()
            },
        },
        results,
    };

    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&args.output, serde_json::to_string_pretty(&payload)?)?;
    Ok(())
}

fn rust_version() -> String {
    std::process::Command::new("rustc")
        .arg("-V")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

fn scenario_runs(manifest: &Manifest, mode: &str) -> Vec<ScenarioRun> {
    let defaults = &manifest.defaults;
    let (warmup_iterations, measured_iterations) = if mode == "smoke" {
        (
            defaults.smoke_warmup_iterations,
            defaults.smoke_measured_iterations,
        )
    } else {
        (defaults.warmup_iterations, defaults.measured_iterations)
    };

    let mut runs = Vec::new();
    for scenario in &manifest.scenarios {
        let sizes = if mode == "smoke" {
            &scenario.sizes[..1]
        } else {
            &scenario.sizes[..]
        };
        for size in sizes {
            runs.push(ScenarioRun {
                scenario_id: scenario.id.clone(),
                plot_kind: scenario.plot_kind.clone(),
                dataset_kind: scenario.dataset_kind.clone(),
                canvas: scenario.canvas.clone(),
                size: size.clone(),
                warmup_iterations,
                measured_iterations,
                hover_stride_px: defaults.hover_stride_px,
                pan_delta_px: (defaults.pan_delta_px[0], defaults.pan_delta_px[1]),
                time_step_seconds: defaults.time_step_seconds,
                elements: element_count(size),
            });
        }
    }
    runs
}

fn element_count(size: &SizeSpec) -> usize {
    if let Some(points) = size.points {
        return points;
    }
    size.rows.unwrap_or(0) * size.cols.unwrap_or(0)
}

fn benchmark_run(run: &ScenarioRun) -> Result<Vec<BenchmarkResult>> {
    let dataset = build_dataset(run)?;
    let mut results = Vec::new();

    for target in [TargetKind::Image, TargetKind::Surface] {
        results.push(measure_case(run, &dataset, target, ActionKind::Hover)?);
        results.push(measure_case(run, &dataset, target, ActionKind::Pan)?);
        if supports_time(run) {
            results.push(measure_case(run, &dataset, target, ActionKind::Time)?);
        }
    }

    Ok(results)
}

fn supports_time(run: &ScenarioRun) -> bool {
    run.plot_kind == "line" || run.plot_kind == "scatter"
}

fn build_dataset(run: &ScenarioRun) -> Result<Dataset> {
    match run.dataset_kind.as_str() {
        "line_signal" | "scatter_signal" => {
            let points = run
                .size
                .points
                .ok_or_else(|| anyhow!("signal scenario missing points"))?;
            let x = match run.dataset_kind.as_str() {
                "line_signal" => line_x(points),
                "scatter_signal" => scatter_x(points),
                _ => unreachable!(),
            };
            let hash = temporal_dataset_hash(run.dataset_kind.as_str(), points, &x);
            Ok(Dataset::TemporalX { x, hash })
        }
        "heatmap_field" => {
            let rows = run
                .size
                .rows
                .ok_or_else(|| anyhow!("heatmap scenario missing rows"))?;
            let cols = run
                .size
                .cols
                .ok_or_else(|| anyhow!("heatmap scenario missing cols"))?;
            let matrix = heatmap_field(rows, cols);
            Ok(Dataset::Heatmap {
                hash: heatmap_dataset_hash(&matrix),
                matrix,
            })
        }
        other => Err(anyhow!("unsupported dataset kind: {other}")),
    }
}

fn line_x(points: usize) -> Vec<f64> {
    let divisor = points.saturating_sub(1).max(1) as f64;
    (0..points).map(|i| i as f64 * (200.0 / divisor)).collect()
}

fn scatter_x(points: usize) -> Vec<f64> {
    let modulus = 2_147_483_647_u64 as f64;
    (0..points)
        .map(|i| ((i as u64 * 48_271) % 2_147_483_647_u64) as f64 / modulus)
        .collect()
}

fn heatmap_field(rows: usize, cols: usize) -> Vec<Vec<f64>> {
    (0..rows)
        .map(|row| {
            (0..cols)
                .map(|col| {
                    triangle_wave(row as i64, 79) * triangle_wave(col as i64, 113)
                        + 0.2 * triangle_wave((row as i64 * 3) + (col as i64 * 5), 47)
                })
                .collect()
        })
        .collect()
}

fn triangle_wave(index: i64, period: i64) -> f64 {
    let phase = (index.rem_euclid(period)) as f64 / period as f64;
    1.0 - 4.0 * (phase - 0.5).abs()
}

fn update_f64_hash(hasher: &mut Sha256, values: &[f64]) {
    for value in values {
        hasher.update(value.to_le_bytes());
    }
}

fn hash_f64_sequences<'a>(sequences: impl IntoIterator<Item = &'a [f64]>) -> String {
    let mut hasher = Sha256::new();
    for values in sequences {
        update_f64_hash(&mut hasher, values);
    }
    format!("{:x}", hasher.finalize())
}

fn temporal_dataset_hash(dataset_kind: &str, points: usize, x: &[f64]) -> String {
    let signal_config = match dataset_kind {
        "line_signal" => [points as f64, 0.0, 200.0, 1.0, 6.0, 1.5, 0.0, 0.0],
        "scatter_signal" => [points as f64, 0.0, 1.0, 0.22, 5.0, 1.1, 0.0, 0.52],
        _ => unreachable!("unsupported temporal dataset kind"),
    };
    hash_f64_sequences([x, signal_config.as_slice()])
}

fn heatmap_dataset_hash(matrix: &[Vec<f64>]) -> String {
    hash_f64_sequences(matrix.iter().map(Vec::as_slice))
}

fn build_plot(run: &ScenarioRun, dataset: &Dataset) -> Plot {
    let builder = Plot::new()
        .size_px(run.canvas.width, run.canvas.height)
        .ticks(false);

    match (&dataset, run.plot_kind.as_str()) {
        (Dataset::TemporalX { x, .. }, "line") => {
            let x = Arc::<[f64]>::from(x.clone().into_boxed_slice());
            builder
                .line_source(x.to_vec(), temporal_line_signal(x))
                .into_plot()
        }
        (Dataset::TemporalX { x, .. }, "scatter") => {
            let x = Arc::<[f64]>::from(x.clone().into_boxed_slice());
            builder
                .scatter_source(x.to_vec(), temporal_scatter_signal(x))
                .into_plot()
        }
        (Dataset::Heatmap { matrix, .. }, "heatmap") => builder.heatmap(matrix, None).into_plot(),
        _ => unreachable!("dataset and plot kind should align"),
    }
}

fn temporal_line_signal(x: Arc<[f64]>) -> Signal<Vec<f64>> {
    Signal::new(move |time| {
        x.iter()
            .map(|value| (value * 0.08 + time * 2.0).sin() + 0.35 * (value * 0.02).cos())
            .collect::<Vec<f64>>()
    })
}

fn temporal_scatter_signal(x: Arc<[f64]>) -> Signal<Vec<f64>> {
    Signal::new(move |time| {
        x.iter()
            .enumerate()
            .map(|(index, value)| {
                let band = (index % 11) as f64 / 10.0 - 0.5;
                (0.52 + 0.22 * (value * 6.0 + time * 1.5).sin() + 0.18 * band).clamp(0.0, 1.0)
            })
            .collect::<Vec<f64>>()
    })
}

fn measure_case(
    run: &ScenarioRun,
    dataset: &Dataset,
    target: TargetKind,
    action: ActionKind,
) -> Result<BenchmarkResult> {
    let plot = build_plot(run, dataset);
    let session = plot.prepare_interactive();
    let size_px = (run.canvas.width, run.canvas.height);
    let scale_factor = 1.0_f32;

    let initial_frame = render_frame(&session, target, size_px, scale_factor, 0.0)?;
    let mut last_frame = initial_frame.clone();

    let center = (
        run.canvas.width as f64 * 0.5,
        run.canvas.height as f64 * 0.5,
    );

    let execute = |iteration: usize| -> Result<InteractiveFrame> {
        match action {
            ActionKind::Hover => {
                let x = hover_position(center.0, run.canvas.width, run.hover_stride_px, iteration);
                let y = hover_position(
                    center.1,
                    run.canvas.height,
                    run.hover_stride_px / 2,
                    iteration,
                );
                session.apply_input(PlotInputEvent::Hover {
                    position_px: ruviz::core::plot::ViewportPoint::new(x, y),
                });
                render_frame(&session, target, size_px, scale_factor, 0.0)
            }
            ActionKind::Pan => {
                let sign = if iteration % 2 == 0 { 1.0 } else { -1.0 };
                session.apply_input(PlotInputEvent::Pan {
                    delta_px: ruviz::core::plot::ViewportPoint::new(
                        run.pan_delta_px.0 as f64 * sign,
                        run.pan_delta_px.1 as f64 * sign,
                    ),
                });
                render_frame(&session, target, size_px, scale_factor, 0.0)
            }
            ActionKind::Time => {
                let time_seconds = run.time_step_seconds * (iteration + 1) as f64;
                render_frame(&session, target, size_px, scale_factor, time_seconds)
            }
        }
    };

    for iteration in 0..run.warmup_iterations {
        last_frame = execute(iteration)?;
    }

    let mut iterations_ms = Vec::with_capacity(run.measured_iterations);
    for iteration in 0..run.measured_iterations {
        let start = Instant::now();
        last_frame = execute(run.warmup_iterations + iteration)?;
        iterations_ms.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    Ok(BenchmarkResult {
        implementation: "ruviz".to_string(),
        scenario_id: run.scenario_id.clone(),
        plot_kind: run.plot_kind.clone(),
        size_label: run.size.label.clone(),
        boundary: format!("{}_{}", target.label(), action.label()),
        output_target: target.output_target().to_string(),
        elements: run.elements,
        canvas: run.canvas.clone(),
        dataset_hash: dataset_hash(dataset).to_string(),
        warmup_iterations: run.warmup_iterations,
        measured_iterations: run.measured_iterations,
        frame_byte_count: last_frame.image.pixels.len(),
        frame_diagnostics: FrameDiagnosticsRecord::from_frame(target, &last_frame),
        summary: summarize_iterations(&iterations_ms, run.elements),
        iterations_ms,
    })
}

fn hover_position(center: f64, extent: u32, stride: u32, iteration: usize) -> f64 {
    let stride = stride.max(1) as f64;
    let radius = (extent as f64 * 0.22).max(stride);
    let phase = iteration as f64 * stride;
    let oscillation = ((phase / radius).sin() * radius).round();
    (center + oscillation).clamp(8.0, extent.saturating_sub(8) as f64)
}

fn render_frame(
    session: &ruviz::core::plot::InteractivePlotSession,
    target: TargetKind,
    size_px: (u32, u32),
    scale_factor: f32,
    time_seconds: f64,
) -> Result<InteractiveFrame> {
    match target {
        TargetKind::Image => Ok(session.render_to_image(ImageTarget {
            size_px,
            scale_factor,
            time_seconds,
        })?),
        TargetKind::Surface => Ok(session.render_to_surface(SurfaceTarget {
            size_px,
            scale_factor,
            time_seconds,
        })?),
    }
}

impl FrameDiagnosticsRecord {
    fn from_frame(target: TargetKind, frame: &InteractiveFrame) -> Self {
        Self {
            target: target.label().to_string(),
            surface_capability: match frame.surface_capability {
                SurfaceCapability::Unsupported => "unsupported",
                SurfaceCapability::FallbackImage => "fallback_image",
                SurfaceCapability::FastPath => "fast_path",
            }
            .to_string(),
            base_dirty: frame.layer_state.base_dirty,
            overlay_dirty: frame.layer_state.overlay_dirty,
            used_incremental_data: frame.layer_state.used_incremental_data,
        }
    }
}

fn dataset_hash(dataset: &Dataset) -> &str {
    match dataset {
        Dataset::TemporalX { hash, .. } => hash,
        Dataset::Heatmap { hash, .. } => hash,
    }
}

fn summarize_iterations(iterations_ms: &[f64], elements: usize) -> Summary {
    let mut sorted = iterations_ms.to_vec();
    sorted.sort_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap_or(std::cmp::Ordering::Equal));

    let mean = iterations_ms.iter().sum::<f64>() / iterations_ms.len() as f64;
    let variance = iterations_ms
        .iter()
        .map(|value| (*value - mean).powi(2))
        .sum::<f64>()
        / iterations_ms.len().saturating_sub(1).max(1) as f64;
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

fn percentile(sorted_values: &[f64], ratio: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }
    if sorted_values.len() == 1 {
        return sorted_values[0];
    }

    let position = ratio * (sorted_values.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        return sorted_values[lower];
    }
    let weight = position - lower as f64;
    sorted_values[lower] * (1.0 - weight) + sorted_values[upper] * weight
}
