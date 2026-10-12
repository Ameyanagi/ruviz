use ruviz::prelude::*;
use ruviz::render::Color;

fn attribute(tag: &str, name: &str) -> f64 {
    let marker = format!("{name}=\"");
    tag.split_once(&marker)
        .expect("SVG attribute")
        .1
        .split_once('"')
        .expect("attribute terminator")
        .0
        .parse()
        .expect("numeric SVG attribute")
}

fn svg_plot_area(plot: &Plot) -> ViewportRect {
    let svg = plot.render_to_svg().expect("render SVG");
    let clip = svg.split_once("<clipPath ").expect("series clip").1;
    let rect = clip.split_once("<rect ").expect("clip rectangle").1;
    let rect = rect.split_once("/>").expect("rectangle terminator").0;
    let x = attribute(rect, "x");
    let y = attribute(rect, "y");
    ViewportRect {
        min: ViewportPoint::new(x, y),
        max: ViewportPoint::new(x + attribute(rect, "width"), y + attribute(rect, "height")),
    }
}

fn assert_ratio(area: ViewportRect, expected: f64) {
    let actual = area.width() / area.height();
    assert!(
        (actual - expected).abs() < expected * 0.0002,
        "axes ratio {actual} differs from {expected}: {area:?}"
    );
}

fn circle_plot() -> Plot {
    let angles: Vec<f64> = (0..=256)
        .map(|index| index as f64 * std::f64::consts::TAU / 256.0)
        .collect();
    let x: Vec<f64> = angles.iter().map(|angle| angle.cos()).collect();
    let y: Vec<f64> = angles.iter().map(|angle| angle.sin()).collect();
    Plot::new()
        .size(6.0, 3.0)
        .xlim(-2.0, 2.0)
        .ylim(-1.5, 1.5)
        .aspect_equal()
        .grid(false)
        .line(&x, &y)
        .color(Color::RED)
        .line_width(2.0)
        .into()
}

fn assert_red_circle(pixels: &[u8], width: u32) {
    let mut left = u32::MAX;
    let mut top = u32::MAX;
    let mut right = 0;
    let mut bottom = 0;
    for (index, pixel) in pixels.chunks_exact(4).enumerate() {
        if pixel[0] > 180 && pixel[1] < 100 && pixel[2] < 100 && pixel[3] > 200 {
            let x = index as u32 % width;
            let y = index as u32 / width;
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
        }
    }
    assert!(left != u32::MAX, "circle has visible red pixels");
    assert!(
        (right - left).abs_diff(bottom - top) <= 2,
        "rendered circle measures {} by {} pixels",
        right - left,
        bottom - top
    );
}

#[test]
fn equal_aspect_keeps_circles_round_in_raster_and_svg_at_multiple_dpis() {
    for dpi in [100, 200] {
        let plot = circle_plot().dpi(dpi);
        assert_ratio(svg_plot_area(&plot), 4.0 / 3.0);
        let image = plot.render().expect("render circle");
        assert_red_circle(&image.pixels, image.width);
    }
}

#[test]
fn aspect_ratio_preserves_resolved_limits_and_axis_direction() {
    let plot: Plot = Plot::new()
        .size_px(640, 360)
        .xlim(4.0, -4.0)
        .ylim(-2.0, 2.0)
        .invert_y()
        .data_aspect(2.0)
        .line(&[-4.0, 4.0], &[-2.0, 2.0])
        .into();
    let session = plot.prepare_interactive();
    session.resize((640, 360), 1.0);
    let snapshot = session.viewport_snapshot().expect("interactive geometry");
    assert_eq!(snapshot.visible_bounds.min, ViewportPoint::new(4.0, 2.0));
    assert_eq!(snapshot.visible_bounds.max, ViewportPoint::new(-4.0, -2.0));
    assert_ratio(snapshot.plot_area, 1.0);
    assert_ratio(svg_plot_area(&plot), 1.0);
}

#[test]
fn aspect_uses_log_and_symlog_coordinates() {
    let logarithmic: Plot = Plot::new()
        .xlim(1.0, 1000.0)
        .ylim(0.0, 2.0)
        .xscale(AxisScale::Log)
        .aspect_equal()
        .line(&[1.0, 1000.0], &[0.0, 2.0])
        .into();
    let symmetric: Plot = Plot::new()
        .xlim(100.0, -100.0)
        .ylim(-1.0, 1.0)
        .xscale(AxisScale::symlog(1.0))
        .data_aspect(2.0)
        .line(&[-100.0, 100.0], &[-1.0, 1.0])
        .into();
    for plot in [logarithmic, symmetric] {
        assert_ratio(svg_plot_area(&plot), 1.5);
        let session = plot.prepare_interactive();
        session.resize((640, 480), 1.0);
        assert_ratio(
            session
                .viewport_snapshot()
                .expect("scaled interactive geometry")
                .plot_area,
            1.5,
        );
    }
}

#[test]
fn interactive_aspect_uses_the_current_viewport() {
    let plot: Plot = Plot::new()
        .size_px(600, 400)
        .xlim(0.0, 10.0)
        .ylim(0.0, 10.0)
        .aspect_equal()
        .line(&[0.0, 10.0], &[0.0, 10.0])
        .into();
    let session = plot.prepare_interactive();
    let target = ImageTarget {
        size_px: (600, 400),
        scale_factor: 1.0,
        time_seconds: 0.0,
    };
    session.render_to_image(target).expect("initial frame");
    assert_ratio(
        session
            .viewport_snapshot()
            .expect("initial geometry")
            .plot_area,
        1.0,
    );
    let visible = ViewportRect {
        min: ViewportPoint::new(1.0, 2.0),
        max: ViewportPoint::new(3.0, 6.0),
    };
    assert!(session.restore_visible_bounds(visible));
    session.render_to_image(target).expect("zoomed frame");
    let snapshot = session.viewport_snapshot().expect("zoomed geometry");
    assert!((snapshot.visible_bounds.min.x - visible.min.x).abs() < 1.0e-12);
    assert!((snapshot.visible_bounds.min.y - visible.min.y).abs() < 1.0e-12);
    assert!((snapshot.visible_bounds.max.x - visible.max.x).abs() < 1.0e-12);
    assert!((snapshot.visible_bounds.max.y - visible.max.y).abs() < 1.0e-12);
    assert_ratio(snapshot.plot_area, 0.5);
}

#[test]
fn outside_legends_and_labels_preserve_equal_aspect() {
    for position in [
        LegendPosition::OutsideLeft,
        LegendPosition::OutsideRight,
        LegendPosition::OutsideUpper,
        LegendPosition::OutsideLower,
    ] {
        let plot: Plot = circle_plot()
            .title("Unit circle")
            .xlabel("Real coordinate")
            .ylabel("Imaginary coordinate")
            .legend(position)
            .line(&[-0.5, 0.5], &[0.0, 0.0])
            .color(Color::BLUE)
            .label("Reference")
            .into();
        let svg = plot.render_to_svg().expect("decorated SVG");
        for text in [
            "Unit circle",
            "Real coordinate",
            "Imaginary coordinate",
            "Reference",
        ] {
            assert!(svg.contains(text), "SVG keeps {text}");
        }
        assert_ratio(svg_plot_area(&plot), 4.0 / 3.0);
        let image = plot.render().expect("decorated raster");
        assert_red_circle(&image.pixels, image.width);
    }
}

#[test]
fn equal_aspect_applies_inside_a_subplot() {
    let output = tempfile::tempdir().expect("temporary output");
    let path = output.path().join("aspect-subplot.png");
    subplots(1, 1, 700, 320)
        .expect("subplot figure")
        .subplot(0, 0, circle_plot())
        .expect("circle subplot")
        .save(&path)
        .expect("render subplot");
    let image = image::open(path).expect("read subplot").into_rgba8();
    assert_red_circle(image.as_raw(), image.width());
}

#[test]
fn invalid_aspects_fail_when_rendered() {
    for ratio in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let plot = circle_plot().data_aspect(ratio);
        for error in [
            plot.render().expect_err("invalid raster aspect"),
            plot.render_to_svg().expect_err("invalid SVG aspect"),
        ] {
            assert!(
                error
                    .to_string()
                    .contains("Data aspect ratio must be finite and positive")
            );
        }
    }
}

#[test]
fn finite_extreme_ranges_do_not_overflow_aspect_calculation() {
    let plot: Plot = Plot::new()
        .ticks(false)
        .grid(false)
        .xlim(-1.0e308, 1.0e308)
        .ylim(-1.0e308, 1.0e308)
        .aspect_equal()
        .line(&[-1.0e308, 1.0e308], &[-1.0e308, 1.0e308])
        .into();
    assert_ratio(svg_plot_area(&plot), 1.0);
    let unrepresentable = circle_plot().data_aspect(f64::MAX);
    assert!(unrepresentable.render().is_err());
    assert!(unrepresentable.render_to_svg().is_err());
}
