//! Backwards-compatible release benchmark for comparing flat markers to 0.12.2.
//! Copy this file to the baseline checkout's examples/ and run with 3d,gpu.
use ruviz::prelude::*;
use std::time::Instant;

fn main() -> PlotResult<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let backend = args.first().map(String::as_str).unwrap_or("cpu");
    let count: usize = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(1000);
    let coordinate = |step: f64| {
        (0..count)
            .map(|i| ((i as f64 * step).fract() - 0.5) * 16.0)
            .collect::<Vec<_>>()
    };
    let x = coordinate(0.61803398875);
    let y = coordinate(0.41421356237);
    let z = coordinate(0.73205080757);
    let mut session = scatter3d(&x, &y, &z)
        .marker_size(8.0)
        .camera(Camera3D::default().axis_aspect(AxisAspect3D::Equal))
        .xlim(-9.0, 9.0)
        .ylim(-9.0, 9.0)
        .zlim(-9.0, 9.0)
        .size_px(800, 600)
        .dpi(96)
        .interactive_session()?;
    let mut worker = BackgroundRenderer3D::new(match backend {
        "cpu" => BackgroundRenderBackend3D::Cpu,
        #[cfg(feature = "gpu")]
        "gpu" => BackgroundRenderBackend3D::GpuReadback,
        _ => panic!("backend must be cpu or gpu"),
    });
    let frames: usize = std::env::var("RUVIZ_SPHERE_BENCH_FRAMES")
        .ok()
        .map(|n| n.parse().unwrap())
        .unwrap_or(60)
        .max(2);
    let mut times = Vec::new();
    let mut checksum = 0_u64;
    for i in 0..frames + 5 {
        let start = Instant::now();
        session.orbit(0.8, 0.15)?;
        let frame = worker.render(session.background_render_job()?)?;
        if i >= 5 {
            times.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        if i == frames + 4 {
            checksum = frame
                .image
                .pixels
                .iter()
                .fold(0xcbf29ce484222325_u64, |hash, b| {
                    (hash ^ u64::from(*b)).wrapping_mul(0x100000001b3)
                });
        }
    }
    times.sort_by(f64::total_cmp);
    println!(
        "{backend},{count},{:.3},{:.3},{checksum:016x}",
        times[((times.len() - 1) as f64 * 0.5).ceil() as usize],
        times[((times.len() - 1) as f64 * 0.95).ceil() as usize]
    );
    Ok(())
}
