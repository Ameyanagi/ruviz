//! Frame-time benchmarks for the interactive presentation path.
//!
//! GUI adapters present an [`InteractivePlotSession`] one of two ways:
//!
//! - **composed** — `render_to_image_stamped`, which alpha-composites the
//!   overlay layer onto the base layer on the CPU and hands back one flat RGBA
//!   buffer.
//! - **layered** — `render_layers_stamped`, which hands back the base and
//!   overlay layers uncomposed so the toolkit/GPU stacks them.
//!
//! The two paths do identical render work; they differ only by the composite.
//! This bench measures the per-frame cost broken into the stages that matter:
//!
//! | bench | stage |
//! |---|---|
//! | `interactive_presentation/base_render/*` | base render (base layer dirty) |
//! | `interactive_presentation/overlay_render/*` | overlay render (overlay-only redraw — the hover case) |
//! | `interactive_presentation_stage/composite` | the full-frame source-over the composed path adds |
//! | `interactive_presentation_stage/pixel_copy` | the full-frame memcpy an adapter pays to upload a layer |
//!
//! `overlay_render/composed` vs `overlay_render/layered` is the headline
//! number: on an overlay-only redraw the base layer is unchanged, so the
//! composite is pure waste and the layered path should shed it entirely.
//!
//! Everything runs at 2800x1800 physical (a ~1400x900 logical window at 2x),
//! which is where the ~5M-pixel scalar composite actually hurts.
//!
//! This bench deliberately depends on `ruviz` only — no GUI adapter — so it
//! stays in the root workspace and builds fast.
//!
//! Run it as
//! `cargo bench --bench interactive_presentation`, and do **not** trust
//! `-- --quick` here: a frame costs tens of milliseconds and the composite it
//! is measuring is a ~6 ms slice of that, which is inside quick mode's noise.
//! Observed on an M-series laptop, quick mode moved individual results by
//! ±60% and twice reported the layered path as the *slower* one, while the
//! full run reproduces to within ~1 ms across invocations.

use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};

use criterion::{Criterion, criterion_group, criterion_main};
use ruviz::core::{
    AlphaMode, Image, ImageTarget, InteractivePlotSession, PlotInputEvent, ViewportPoint,
    source_over_straight_rgba,
};
use ruviz::prelude::Plot;

/// Physical backing size of a ~1400x900 logical window on a 2x display.
const FRAME_SIZE_PX: (u32, u32) = (2800, 1800);
const SCALE_FACTOR: f32 = 2.0;

const TARGET: ImageTarget = ImageTarget {
    size_px: FRAME_SIZE_PX,
    scale_factor: SCALE_FACTOR,
    time_seconds: 0.0,
};

/// One realistic interactive plot. Kept modest in point count so the numbers
/// are dominated by per-frame presentation cost, not by data reduction.
fn interactive_session() -> InteractivePlotSession {
    const POINTS: usize = 20_000;
    let x: Vec<f64> = (0..POINTS).map(|i| i as f64 * 0.005).collect();
    let y: Vec<f64> = x.iter().map(|v| (v * 0.7).sin() * 40.0 + v * 0.2).collect();
    let plot: Plot = Plot::new()
        .line(&x, &y)
        .title("interactive presentation")
        .xlabel("t")
        .ylabel("value")
        .into();
    let session = plot.prepare_interactive();
    session.resize(FRAME_SIZE_PX, SCALE_FACTOR);
    session
}

/// The two frame kinds an interactive adapter actually renders.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scenario {
    /// The view moved, so the base layer must be re-rendered.
    BaseDirty,
    /// Only the pointer moved: the base layer is reused, the overlay redraws.
    OverlayOnly,
}

impl Scenario {
    fn name(self) -> &'static str {
        match self {
            Scenario::BaseDirty => "base_render",
            Scenario::OverlayOnly => "overlay_render",
        }
    }

    fn expects_base_dirty(self) -> bool {
        self == Scenario::BaseDirty
    }

    /// Applies the input event that dirties the next frame the way this
    /// scenario describes. Always called outside the measured region.
    fn mutate(self, session: &InteractivePlotSession, step: u64) {
        match self {
            // Alternating pan keeps the view in place over many iterations
            // while still invalidating the base layer on every frame.
            Scenario::BaseDirty => {
                let dx = if step.is_multiple_of(2) { 12.0 } else { -12.0 };
                session.apply_input(PlotInputEvent::Pan {
                    delta_px: ViewportPoint::new(dx, 0.0),
                });
            }
            // A tooltip tracking the cursor: overlay content changes, the
            // plotted data does not.
            Scenario::OverlayOnly => {
                session.apply_input(PlotInputEvent::ShowTooltip {
                    content: "t = 41.2, value = 18.7".to_string(),
                    position_px: ViewportPoint::new(600.0 + (step % 128) as f64, 900.0),
                });
            }
        }
    }
}

/// Builds a session that has already committed a frame with a live overlay, and
/// proves the scenario produces the layer-dirty pattern it claims to.
///
/// Without this check a mis-specified scenario would quietly benchmark a frame
/// where nothing was dirty, and report a meaningless speed-up.
fn prepared_session(scenario: Scenario) -> InteractivePlotSession {
    let session = interactive_session();
    session
        .render_layers_stamped(TARGET)
        .expect("initial frame should render");
    Scenario::OverlayOnly.mutate(&session, 0);
    session
        .render_layers_stamped(TARGET)
        .expect("overlay warm-up frame should render");

    scenario.mutate(&session, 1);
    let probe = session
        .render_layers_stamped(TARGET)
        .expect("probe frame should render");
    assert_eq!(
        probe.layer_state.base_dirty,
        scenario.expects_base_dirty(),
        "{} scenario must produce base_dirty == {}",
        scenario.name(),
        scenario.expects_base_dirty()
    );
    assert!(
        probe.layer_state.overlay_dirty,
        "{} scenario must re-render the overlay",
        scenario.name()
    );
    assert!(
        probe.overlay.is_some(),
        "{} scenario must have a live overlay layer",
        scenario.name()
    );
    session
}

/// Mirrors the crate-private `compose_images`
/// (`src/core/plot/interactive_session/helpers.rs`) that the composed path
/// runs: a fresh full-frame allocation plus a scalar per-pixel source-over.
fn compose_straight(base: &Image, overlay: &Image) -> Image {
    let mut pixels = base.pixels.clone();
    for (dst, src) in pixels
        .chunks_exact_mut(4)
        .zip(overlay.pixels.chunks_exact(4))
    {
        let destination = [dst[0], dst[1], dst[2], dst[3]];
        let source = [src[0], src[1], src[2], src[3]];
        dst.copy_from_slice(&source_over_straight_rgba(destination, source));
    }
    Image::new(base.width, base.height, pixels)
}

/// Composed vs layered, for a base-dirty frame and for an overlay-only frame.
///
/// `iter_custom` is used rather than `iter_batched` because the dirtying input
/// must be applied immediately before each render: `iter_batched` runs a whole
/// batch of setup calls up front, which would coalesce into a single dirty
/// frame and leave the rest of the batch measuring a no-op.
fn bench_frame_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("interactive_presentation");
    // A frame here costs tens of milliseconds, so the sample count is the
    // minimum criterion allows and the time budget is spent on iterations
    // instead. The long warm-up is load-bearing: the composed path allocates a
    // fresh ~20 MB frame every iteration, and with a short warm-up the
    // allocator is still cold enough to hide most of the composite cost.
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(10));

    for scenario in [Scenario::BaseDirty, Scenario::OverlayOnly] {
        let session = prepared_session(scenario);
        let mut step = 1u64;

        group.bench_function(format!("{}/composed", scenario.name()), |b| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    step += 1;
                    scenario.mutate(&session, step);
                    let start = Instant::now();
                    let frame = session
                        .render_to_image_stamped(TARGET)
                        .expect("composed frame should render");
                    black_box(&frame);
                    // Dropped inside the measured region: freeing the freshly
                    // composed full-frame buffer is a cost this path really pays.
                    drop(frame);
                    total += start.elapsed();
                }
                total
            });
        });

        group.bench_function(format!("{}/layered", scenario.name()), |b| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    step += 1;
                    scenario.mutate(&session, step);
                    let start = Instant::now();
                    let frame = session
                        .render_layers_stamped(TARGET)
                        .expect("layered frame should render");
                    black_box(&frame);
                    drop(frame);
                    total += start.elapsed();
                }
                total
            });
        });
    }

    group.finish();
}

/// The two full-frame pixel passes, isolated from any render work.
fn bench_presentation_stages(c: &mut Criterion) {
    let session = prepared_session(Scenario::OverlayOnly);
    Scenario::OverlayOnly.mutate(&session, 99);
    let layers = session
        .render_layers_stamped(TARGET)
        .expect("stage frame should render");
    let base = Arc::clone(layers.base.image());
    let overlay = Arc::clone(
        layers
            .overlay
            .as_ref()
            .expect("stage frame needs an overlay")
            .image(),
    );
    assert_eq!(base.alpha_mode(), AlphaMode::Straight);
    assert_eq!(overlay.alpha_mode(), AlphaMode::Straight);
    assert_eq!((base.width, base.height), (overlay.width, overlay.height));

    let mut group = c.benchmark_group("interactive_presentation_stage");
    group.sample_size(20);
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("composite", |b| {
        b.iter(|| black_box(compose_straight(black_box(&base), black_box(&overlay))));
    });

    // A texture upload's full-frame memcpy, without the allocation the
    // composite already accounts for.
    let mut scratch = vec![0u8; base.pixels.len()];
    group.bench_function("pixel_copy", |b| {
        b.iter(|| {
            scratch.copy_from_slice(black_box(&base.pixels));
            black_box(&scratch);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_frame_paths, bench_presentation_stages);
criterion_main!(benches);
