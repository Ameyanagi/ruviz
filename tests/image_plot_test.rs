//! Public-contract tests for RGBA image series and eager function sampling.

use base64::Engine as _;
use ruviz::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::OnceLock;

const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const YELLOW: [u8; 4] = [255, 255, 0, 255];

fn test_plot() -> Plot {
    Plot::with_theme(
        Theme::builder()
            .background(Color::WHITE)
            .frame(false)
            .build(),
    )
    .size_px(240, 200)
    .grid(false)
    .ticks(false)
    .xlim(0.0, 2.0)
    .ylim(0.0, 2.0)
}

fn solid_image(color: [u8; 4]) -> Image {
    Image::new(1, 1, color.to_vec())
}

fn quadrants() -> Image {
    Image::new(2, 2, [RED, GREEN, BLUE, YELLOW].concat())
}

fn pixel(image: &Image, x: u32, y: u32) -> [u8; 4] {
    let offset = ((y * image.width + x) * 4) as usize;
    image.pixels[offset..offset + 4].try_into().unwrap()
}

#[derive(Clone, Copy, Debug)]
struct Rect {
    left: u32,
    right: u32,
    top: u32,
    bottom: u32,
}

impl Rect {
    fn sample(self, image: &Image, x_fraction: f64, y_fraction: f64) -> [u8; 4] {
        // Fractions are screen coordinates: y increases downward.
        let x = self.left + ((self.right - self.left) as f64 * x_fraction).round() as u32;
        let y = self.top + ((self.bottom - self.top) as f64 * y_fraction).round() as u32;
        pixel(image, x, y)
    }
}

fn color_bounds(image: &Image, color: [u8; 4]) -> Rect {
    let mut rect = Rect {
        left: image.width,
        right: 0,
        top: image.height,
        bottom: 0,
    };
    for y in 0..image.height {
        for x in 0..image.width {
            if pixel(image, x, y) == color {
                rect.left = rect.left.min(x);
                rect.right = rect.right.max(x);
                rect.top = rect.top.min(y);
                rect.bottom = rect.bottom.max(y);
            }
        }
    }
    assert!(
        rect.right > rect.left && rect.bottom > rect.top,
        "color {color:?} should cover a rectangle"
    );
    rect
}

fn panel() -> Rect {
    static PANEL: OnceLock<Rect> = OnceLock::new();
    *PANEL.get_or_init(|| {
        let calibration_color = [33, 211, 177, 255];
        let image = test_plot()
            .image(&solid_image(calibration_color))
            .extent(0.0, 2.0, 0.0, 2.0)
            .render()
            .expect("calibration image should render");
        color_bounds(&image, calibration_color)
    })
}

#[test]
fn invalid_images_and_extents_return_deferred_errors() {
    for image in [
        Image::new(0, 1, vec![]),
        Image::new(1, 0, vec![]),
        Image::new(1, 1, vec![0; 3]),
        Image::new(1, 1, vec![0; 5]),
        Image::new(u32::MAX, u32::MAX, vec![]),
    ] {
        // Constructing and finalizing the fluent builder must not panic.
        let plot = Plot::new().image(&image).into_plot();
        assert!(plot.render().is_err());
        assert!(plot.render_to_svg().is_err());
    }

    for extent in [
        (0.0, 0.0, 0.0, 1.0),
        (1.0, 0.0, 0.0, 1.0),
        (0.0, 1.0, 2.0, 1.0),
        (f64::NAN, 1.0, 0.0, 1.0),
        (0.0, f64::INFINITY, 0.0, 1.0),
        (-f64::MAX, f64::MAX, 0.0, 1.0),
    ] {
        let plot = Plot::new()
            .image(&solid_image(RED))
            .extent(extent.0, extent.1, extent.2, extent.3)
            .into_plot();
        assert!(plot.render().is_err(), "invalid extent {extent:?}");
        assert!(
            plot.render_to_svg().is_err(),
            "invalid SVG extent {extent:?}"
        );
    }
}

#[test]
fn source_rgba_is_preserved_and_series_keeps_its_pixels() {
    let original = vec![255, 0, 0, 128, 18, 219, 71, 0];
    let mut source = Image::new(2, 1, original.clone());
    let plot = test_plot()
        .image(&source)
        .extent(0.0, 2.0, 0.0, 2.0)
        .into_plot();
    assert_eq!(source.pixels, original);
    source.pixels.fill(0);

    let rendered = plot.render().unwrap();
    let translucent_red = panel().sample(&rendered, 0.25, 0.5);
    assert_eq!(translucent_red[0], 255);
    assert!(
        (126..=128).contains(&translucent_red[1]),
        "{translucent_red:?}"
    );
    assert_eq!(translucent_red[1], translucent_red[2]);
    assert_eq!(panel().sample(&rendered, 0.75, 0.5), [255; 4]);
}

#[test]
fn origin_and_axis_direction_are_independent() {
    for origin in [ImageOrigin::Upper, ImageOrigin::Lower] {
        for reverse_x in [false, true] {
            for reverse_y in [false, true] {
                let rendered = test_plot()
                    .xlim(
                        if reverse_x { 2.0 } else { 0.0 },
                        if reverse_x { 0.0 } else { 2.0 },
                    )
                    .ylim(
                        if reverse_y { 2.0 } else { 0.0 },
                        if reverse_y { 0.0 } else { 2.0 },
                    )
                    .image(&quadrants())
                    .origin(origin)
                    .render()
                    .unwrap();
                for (column, row, expected) in
                    [(0, 0, RED), (1, 0, GREEN), (0, 1, BLUE), (1, 1, YELLOW)]
                {
                    let x_fraction = if (column == 0) ^ reverse_x {
                        0.25
                    } else {
                        0.75
                    };
                    let physical_upper = (row == 0) == (origin == ImageOrigin::Upper);
                    let y_fraction = if physical_upper ^ reverse_y {
                        0.25
                    } else {
                        0.75
                    };
                    assert_eq!(
                        panel().sample(&rendered, x_fraction, y_fraction),
                        expected,
                        "origin={origin:?}, reverse_x={reverse_x}, reverse_y={reverse_y}, cell=({column}, {row})"
                    );
                }
            }
        }
    }
}

#[test]
fn extents_use_outer_edges_and_images_clip_to_axes() {
    let rendered = test_plot()
        .image_with(
            &solid_image(BLUE),
            ImageConfig::new().extent(0.5, 1.5, 0.5, 1.5),
        )
        .render()
        .unwrap();
    assert_eq!(panel().sample(&rendered, 0.5, 0.5), BLUE);
    for (x, y) in [(0.1, 0.5), (0.9, 0.5), (0.5, 0.1), (0.5, 0.9)] {
        assert_eq!(panel().sample(&rendered, x, y), [255; 4]);
    }

    let clipped = test_plot()
        .image(&solid_image(BLUE))
        .extent(-2.0, 4.0, -2.0, 4.0)
        .render()
        .unwrap();
    assert_eq!(panel().sample(&clipped, 0.05, 0.05), BLUE);
    assert_eq!(panel().sample(&clipped, 0.95, 0.95), BLUE);
    let rect = panel();
    assert_eq!(
        pixel(
            &clipped,
            rect.left.saturating_sub(8),
            (rect.top + rect.bottom) / 2
        ),
        [255; 4]
    );
    assert_eq!(
        pixel(
            &clipped,
            (rect.left + rect.right) / 2,
            rect.top.saturating_sub(8)
        ),
        [255; 4]
    );
}

#[test]
fn bilinear_sampling_uses_premultiplied_alpha_and_series_opacity() {
    // Hidden blue at alpha zero must not make a purple fringe beside red.
    let source = Image::new(2, 1, vec![255, 0, 0, 255, 0, 0, 255, 0]);
    let rendered = test_plot()
        .image(&source)
        .extent(0.0, 2.0, 0.0, 2.0)
        .interpolation(ImageInterpolation::Bilinear)
        .render()
        .unwrap();
    let middle = panel().sample(&rendered, 0.5, 0.5);
    assert!(
        middle[0] >= 253,
        "transparent color should not darken red: {middle:?}"
    );
    assert!(
        (110..=145).contains(&middle[1]),
        "half coverage: {middle:?}"
    );
    assert!(
        middle[1].abs_diff(middle[2]) <= 2,
        "transparent blue should not tint interpolation: {middle:?}"
    );

    let half_opacity = test_plot()
        .image(&solid_image(RED))
        .extent(0.0, 2.0, 0.0, 2.0)
        .alpha(0.5)
        .render()
        .unwrap();
    let middle = panel().sample(&half_opacity, 0.5, 0.5);
    assert_eq!(middle[0], 255);
    assert!((127..=128).contains(&middle[1]), "{middle:?}");
    assert_eq!(middle[1], middle[2]);
}

#[test]
fn image_line_and_scatter_respect_insertion_order() {
    let rendered = test_plot()
        .image(&solid_image(BLUE))
        .extent(0.0, 2.0, 0.0, 2.0)
        .line(&[0.0, 2.0], &[1.0, 1.0])
        .color(Color::RED)
        .line_width(6.0)
        .scatter(&[1.0], &[1.0])
        .color(Color::from_rgb(0, 255, 0))
        .marker_size(14.0)
        .render()
        .unwrap();
    assert_eq!(panel().sample(&rendered, 0.25, 0.5), RED);
    assert_eq!(panel().sample(&rendered, 0.5, 0.5), GREEN);
    assert_eq!(panel().sample(&rendered, 0.5, 0.75), BLUE);

    let covered = test_plot()
        .line(&[0.0, 2.0], &[1.0, 1.0])
        .color(Color::RED)
        .line_width(6.0)
        .image(&solid_image(BLUE))
        .extent(0.0, 2.0, 0.0, 2.0)
        .render()
        .unwrap();
    assert_eq!(panel().sample(&covered, 0.25, 0.5), BLUE);
}

#[test]
fn nonlinear_axes_sample_in_data_coordinates() {
    let source = Image::new(2, 1, [RED, BLUE].concat());
    let plot = test_plot().xlim(1.0, 100.0).xscale(AxisScale::Log);
    let calibration = plot
        .clone()
        .image(&solid_image(GREEN))
        .extent(1.0, 100.0, 0.0, 2.0)
        .render()
        .unwrap();
    let rect = color_bounds(&calibration, GREEN);
    let rendered = plot
        .image(&source)
        .extent(1.0, 100.0, 0.0, 2.0)
        .render()
        .unwrap();
    // The source-cell boundary is x=50.5, or about 85% across a log axis.
    assert_eq!(rect.sample(&rendered, 0.75, 0.5), RED);
    assert_eq!(rect.sample(&rendered, 0.95, 0.5), BLUE);
}

#[test]
fn helpers_sample_lower_origin_cell_centers_eagerly_and_once() {
    let expected = [(0.5, 0.5), (1.5, 0.5), (0.5, 1.5), (1.5, 1.5)];
    let calls = Rc::new(RefCell::new(Vec::new()));
    let callback_calls = Rc::clone(&calls);
    let plot = test_plot()
        .image_fn(2, 2, (0.0, 2.0, 0.0, 2.0), move |x, y| {
            callback_calls.borrow_mut().push((x, y));
            if y < 1.0 { Color::RED } else { Color::BLUE }
        })
        .into_plot();
    assert_eq!(&*calls.borrow(), &expected);
    let rendered = plot.render().unwrap();
    assert_eq!(panel().sample(&rendered, 0.25, 0.75), RED);
    assert_eq!(panel().sample(&rendered, 0.25, 0.25), BLUE);
    plot.clone().dpi(200).render().unwrap();
    plot.render_to_svg().unwrap();
    plot.prepare().render_frame((320, 240), 1.0, 0.0).unwrap();
    assert_eq!(
        &*calls.borrow(),
        &expected,
        "exports and viewport changes must reuse the original samples"
    );

    let scalar_calls = Rc::new(RefCell::new(Vec::new()));
    let callback_calls = Rc::clone(&scalar_calls);
    let scalar = test_plot()
        .heatmap_fn(2, 2, (0.0, 2.0, 0.0, 2.0), move |x, y| {
            callback_calls.borrow_mut().push((x, y));
            y
        })
        .cmap("gray")
        .vmin(0.0)
        .vmax(2.0)
        .colorbar(false)
        .into_plot();
    assert_eq!(&*scalar_calls.borrow(), &expected);
    let rendered = scalar.render().unwrap();
    let lower = panel().sample(&rendered, 0.25, 0.75);
    let upper = panel().sample(&rendered, 0.25, 0.25);
    assert!(
        upper[0] > lower[0] + 100,
        "lower-origin scalar helper: {lower:?}, {upper:?}"
    );
    scalar.clone().dpi(150).render().unwrap();
    assert_eq!(&*scalar_calls.borrow(), &expected);
}

#[test]
fn invalid_function_grids_do_not_invoke_callbacks() {
    let calls = std::cell::Cell::new(0);
    let image = Plot::new().image_fn(0, 2, (0.0, 1.0, 0.0, 1.0), |_, _| {
        calls.set(calls.get() + 1);
        Color::RED
    });
    assert_eq!(calls.get(), 0);
    assert!(image.render().is_err());
    let scalar = Plot::new().heatmap_fn(2, 2, (1.0, 0.0, 0.0, 1.0), |_, _| {
        calls.set(calls.get() + 1);
        0.5
    });
    assert_eq!(calls.get(), 0);
    assert!(scalar.render().is_err());
}

#[test]
fn large_finite_extents_keep_function_sample_coordinates_finite() {
    let mut image_x = Vec::new();
    let image = Plot::new().image_fn(2, 1, (0.0, f64::MAX, 0.0, 1.0), |x, _| {
        image_x.push(x);
        Color::RED
    });
    assert_eq!(image_x.len(), 2);
    assert!(image_x.iter().all(|x| x.is_finite()));
    assert_eq!(image_x[0], f64::MAX * 0.25);
    assert_eq!(image_x[1], f64::MAX * 0.75);
    drop(image);

    let mut scalar_x = Vec::new();
    let scalar = Plot::new().heatmap_fn(2, 1, (0.0, f64::MAX, 0.0, 1.0), |x, _| {
        scalar_x.push(x);
        0.5
    });
    assert_eq!(scalar_x, image_x);
    drop(scalar);
}

#[test]
fn svg_embeds_a_png_with_matching_image_orientation_without_3d() {
    let plot = test_plot()
        .image(&quadrants())
        .origin(ImageOrigin::Lower)
        .into_plot();
    let raster = plot.render().unwrap();
    let svg = plot.render_to_svg().unwrap();
    let marker = "data:image/png;base64,";
    let start = svg.find(marker).expect("2D SVG must embed image pixels") + marker.len();
    let end = start + svg[start..].find('"').expect("quoted image URL");
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&svg[start..end])
        .unwrap();
    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    let decoded = ::image::load_from_memory(&bytes).unwrap().to_rgba8();
    let embedded = Image::new(decoded.width(), decoded.height(), decoded.into_raw());
    let rect = Rect {
        left: 0,
        top: 0,
        right: embedded.width - 1,
        bottom: embedded.height - 1,
    };
    for (x, y) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
        assert_eq!(rect.sample(&embedded, x, y), panel().sample(&raster, x, y));
    }
}

#[cfg(feature = "pdf")]
#[test]
fn pdf_exports_the_image_as_a_raster_object() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("image.pdf");
    test_plot()
        .image(&quadrants())
        .save_pdf(&path)
        .expect("PDF should support 2D image series");
    let bytes = std::fs::read(path).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
    assert!(bytes.len() > 500);
    let pdf = String::from_utf8_lossy(&bytes);
    assert!(
        pdf.contains("/Subtype /Image"),
        "PDF should contain an embedded image object"
    );
}
