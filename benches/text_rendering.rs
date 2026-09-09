//! Font discovery is warmed once; measure label work without PNG encoding or I/O.
use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use ruviz::render::{Color, FontConfig, FontFamily, TextRenderer};
use tiny_skia::Pixmap;

fn labels(c: &mut Criterion) {
    let renderer = TextRenderer::new();
    let config = FontConfig::new(FontFamily::SansSerif, 16.0);
    let mut pixels = Pixmap::new(600, 80).expect("label canvas");
    let mut group = c.benchmark_group("text");
    group.sample_size(50);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(2));
    for (name, text) in [
        ("latin", "Scattering intensity (Å⁻²)"),
        ("mixed", "散乱強度 / Intensity (Å⁻²)"),
    ] {
        renderer.measure_text(text, &config).expect("warm fonts");
        group.bench_function(format!("measure/{name}"), |b| {
            b.iter(|| black_box(renderer.measure_text(black_box(text), &config).unwrap()));
        });
        group.bench_function(format!("raster/{name}"), |b| {
            b.iter(|| {
                renderer
                    .render_text(
                        &mut pixels,
                        black_box(text),
                        4.0,
                        4.0,
                        &config,
                        Color::BLACK,
                    )
                    .unwrap()
            });
        });
    }
    // A fixed named family also separates code overhead from changes to the
    // generic-family preference policy.
    let named = FontConfig::new(FontFamily::from("Arial"), 16.0);
    for (name, text) in [
        ("latin", "Scattering intensity (Å⁻²)"),
        ("mixed", "散乱強度 / Intensity (Å⁻²)"),
    ] {
        renderer
            .measure_text(text, &named)
            .expect("warm named font");
        group.bench_function(format!("named_measure/{name}"), |b| {
            b.iter(|| black_box(renderer.measure_text(black_box(text), &named).unwrap()));
        });
        group.bench_function(format!("named_raster/{name}"), |b| {
            b.iter(|| {
                renderer
                    .render_text(&mut pixels, black_box(text), 4.0, 4.0, &named, Color::BLACK)
                    .unwrap()
            });
        });
    }
    #[cfg(feature = "typst-math")]
    {
        use ruviz::render::typst_text;
        for (name, text) in [
            ("math", "$q (Å^(-1))$"),
            ("mixed", "散乱強度 / $q (Å^(-1))$"),
        ] {
            typst_text::render_raster(text, 16.0, Color::BLACK, 0.0, "warm Typst").unwrap();
            group.bench_function(format!("typst_cached/{name}"), |b| {
                b.iter(|| {
                    black_box(
                        typst_text::render_raster(text, 16.0, Color::BLACK, 0.0, "bench").unwrap(),
                    )
                });
            });
        }
    }
    let international = named.clone().text_options(
        ruviz::render::TextOptions::new()
            .font_fallbacks(["Helvetica", "Noto Sans CJK JP"])
            .language("ja"),
    );
    let mixed = "散乱強度 / Intensity (Å⁻²)";
    renderer
        .measure_text(mixed, &international)
        .expect("warm international context");
    group.bench_function("configured_measure/mixed", |b| {
        b.iter(|| {
            black_box(
                renderer
                    .measure_text(black_box(mixed), &international)
                    .unwrap(),
            )
        });
    });
    group.bench_function("configured_raster/mixed", |b| {
        b.iter(|| {
            renderer
                .render_text(
                    &mut pixels,
                    black_box(mixed),
                    4.0,
                    4.0,
                    &international,
                    Color::BLACK,
                )
                .unwrap()
        });
    });
    #[cfg(feature = "typst-math")]
    group.bench_function("configured_typst_cached/mixed", |b| {
        b.iter(|| {
            black_box(
                ruviz::render::typst_text::render_raster_with_options(
                    mixed,
                    16.0,
                    Color::BLACK,
                    0.0,
                    &international.family,
                    &international.text_options,
                    "bench",
                )
                .unwrap(),
            )
        });
    });
    group.finish();
}

criterion_group!(benches, labels);
criterion_main!(benches);
