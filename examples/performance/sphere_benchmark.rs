//! `cargo run --release --example sphere_benchmark --features 3d,gpu -- gpu shaded 1000`
//! Modes: flat / unlit / shaded. Datasets: 100 / 1000 / 10000 / ru / ruo2.
//! Timings include image readback and composition, as used by the GPUI worker.
#[path = "../support/molecules/mod.rs"]
mod molecules;

use ruviz::prelude::*;
use std::time::Instant;

fn session(mode: &str, atoms: &[Sphere3D]) -> PlotResult<InteractivePlot3DSession> {
    let camera = Camera3D::default().axis_aspect(AxisAspect3D::Data);
    if mode == "flat" {
        let x: Vec<_> = atoms.iter().map(|a| a.center.x).collect();
        let y: Vec<_> = atoms.iter().map(|a| a.center.y).collect();
        let z: Vec<_> = atoms.iter().map(|a| a.center.z).collect();
        scatter3d(&x, &y, &z)
            .marker_size(8.0)
            .camera(camera)
            .xlim(-9.0, 9.0)
            .ylim(-9.0, 9.0)
            .zlim(-9.0, 9.0)
            .size_px(800, 600)
            .dpi(96)
            .interactive_session()
    } else {
        spheres3d(atoms)
            .shading(mode == "shaded")
            .camera(camera)
            .xlim(-9.0, 9.0)
            .ylim(-9.0, 9.0)
            .zlim(-9.0, 9.0)
            .size_px(800, 600)
            .dpi(96)
            .interactive_session()
    }
}

fn percentile(samples: &mut [f64], q: f64) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[((samples.len() - 1) as f64 * q).ceil() as usize]
}

fn main() -> PlotResult<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let backend = args.first().map(String::as_str).unwrap_or("cpu");
    let mode = args.get(1).map(String::as_str).unwrap_or("shaded");
    let dataset = args.get(2).map(String::as_str).unwrap_or("1000");
    assert!(matches!(mode, "flat" | "unlit" | "shaded"));
    let mut atoms = match dataset {
        "ru" => {
            let a = molecules::cluster(false, 8.0);
            assert_eq!(a.len(), 147);
            a
        }
        "ruo2" => {
            let a = molecules::cluster(true, 8.0);
            assert_eq!(a.len(), 209);
            a
        }
        _ => synthetic(dataset.parse().expect("atom count")),
    };
    let frames: usize = std::env::var("RUVIZ_SPHERE_BENCH_FRAMES")
        .ok()
        .map(|n| n.parse().expect("frame count"))
        .unwrap_or(60)
        .max(2);
    let start = Instant::now();
    let mut session = session(mode, &atoms)?;
    let render_backend = match backend {
        "cpu" => BackgroundRenderBackend3D::Cpu,
        #[cfg(feature = "gpu")]
        "gpu" => BackgroundRenderBackend3D::GpuReadback,
        _ => panic!("backend must be cpu or gpu (with the gpu feature)"),
    };
    let mut worker = BackgroundRenderer3D::new(render_backend);
    let (_, first) = worker.render_with_diagnostics(session.background_render_job()?)?;
    let first_ms = start.elapsed().as_secs_f64() * 1000.0;
    eprintln!(
        "backend={} adapter={:?} viewport=800x600 dpi=96 samples={} first_upload_bytes={}",
        first.actual_backend,
        first.adapter_name,
        first.sample_count,
        first.vertex_upload_bytes + first.index_upload_bytes + first.texture_upload_bytes
    );
    let mut orbit = Vec::new();
    let mut update = Vec::new();
    for i in 0..frames + 5 {
        let start = Instant::now();
        session.orbit(0.8, 0.15)?;
        worker.render(session.background_render_job()?)?;
        if i >= 5 {
            orbit.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    for i in 0..frames {
        let start = Instant::now();
        atoms[0].center.x += if i % 2 == 0 { 0.01 } else { -0.01 };
        session.replace_keep_camera(crate::session(mode, &atoms)?)?;
        worker.render(session.background_render_job()?)?;
        update.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    println!(
        "backend,mode,dataset,atoms,first_ms,orbit_p50_ms,orbit_p95_ms,update_p50_ms,update_p95_ms,first_upload_bytes"
    );
    println!(
        "{backend},{mode},{dataset},{},{first_ms:.3},{:.3},{:.3},{:.3},{:.3},{}",
        atoms.len(),
        percentile(&mut orbit, 0.5),
        percentile(&mut orbit, 0.95),
        percentile(&mut update, 0.5),
        percentile(&mut update, 0.95),
        first.vertex_upload_bytes + first.index_upload_bytes + first.texture_upload_bytes
    );
    Ok(())
}

pub fn synthetic(count: usize) -> Vec<Sphere3D> {
    // Irrational increments give a deterministic volume-filling cloud.
    (0..count)
        .map(|i| {
            let coordinate = |step: f64| ((i as f64 * step).fract() - 0.5) * 16.0;
            Sphere3D::new(
                i as u32,
                Point3D::new(
                    coordinate(0.61803398875),
                    coordinate(0.41421356237),
                    coordinate(0.73205080757),
                ),
                0.35,
                Color::from_palette(i % 3),
            )
        })
        .collect()
}
