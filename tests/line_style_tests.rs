//! Contract for `LineStyle`: each variant has to reach the pixels.
//!
//! Every assertion in this file used to be `assert!(result.is_ok())` on a
//! `save()`, so a renderer that drew Dashed, Dotted, DashDot, DashDotDot and
//! Custom all as solid lines passed the whole suite. The tests below render in
//! memory and compare the actual output: a patterned line must cover strictly
//! less of the canvas than a solid one, and no two styles may produce the same
//! image.

use ruviz::prelude::*;

/// Every style the crate offers, in a stable order.
fn all_line_styles() -> Vec<(&'static str, LineStyle)> {
    vec![
        ("Solid", LineStyle::Solid),
        ("Dashed", LineStyle::Dashed),
        ("Dotted", LineStyle::Dotted),
        ("DashDot", LineStyle::DashDot),
        ("DashDotDot", LineStyle::DashDotDot),
        ("Custom", LineStyle::Custom(vec![20.0, 5.0, 10.0, 5.0])),
    ]
}

/// Render one long diagonal stroke with nothing else on the canvas, so the
/// only thing that can differ between renders is the stroke itself.
fn render_styled_line(style: LineStyle, line_width: f32) -> Image {
    let x: Vec<f64> = (0..=20).map(|i| i as f64).collect();
    let y = x.clone();

    Plot::new()
        .size_px(400, 300)
        .line(&x, &y)
        .ticks(false)
        .grid(false)
        .line_style(style)
        .line_width(line_width)
        .color(Color::from_rgb(0, 0, 0))
        .render()
        .expect("a styled line must render")
}

/// Pixels that differ from the canvas background.
fn ink_pixels(image: &Image) -> usize {
    let background = &image.pixels[..4];
    image
        .pixels
        .chunks_exact(4)
        .filter(|pixel| *pixel != background)
        .count()
}

#[test]
fn no_two_line_styles_render_the_same_image() {
    let rendered: Vec<(&str, Image)> = all_line_styles()
        .into_iter()
        .map(|(name, style)| (name, render_styled_line(style, 2.0)))
        .collect();

    for (index, (name, image)) in rendered.iter().enumerate() {
        assert!(ink_pixels(image) > 0, "{name} drew nothing onto the canvas");
        for (other_name, other) in rendered.iter().skip(index + 1) {
            assert!(
                image.pixels != other.pixels,
                "{name} and {other_name} produced identical pixels, so at least one \
                 of them is not being applied to the stroke"
            );
        }
    }
}

#[test]
fn every_patterned_style_covers_less_than_a_solid_line() {
    let solid = ink_pixels(&render_styled_line(LineStyle::Solid, 2.0));
    assert!(solid > 0, "the solid reference stroke drew nothing");

    for (name, style) in all_line_styles().into_iter().skip(1) {
        let patterned = ink_pixels(&render_styled_line(style, 2.0));
        assert!(patterned > 0, "{name} drew nothing onto the canvas");
        assert!(
            patterned < solid,
            "{name} inked {patterned} pixels and a solid line inked {solid}; a gapped \
             pattern must cover strictly less, so {name} is rendering solid"
        );
    }
}

#[test]
fn line_width_scales_a_patterned_stroke() {
    let thin = ink_pixels(&render_styled_line(LineStyle::Dashed, 1.0));
    let thick = ink_pixels(&render_styled_line(LineStyle::Dashed, 4.0));

    assert!(
        thick > thin,
        "a 4pt dashed stroke inked {thick} pixels and a 1pt one inked {thin}; \
         line width is not reaching the dashed geometry"
    );
}

#[test]
fn per_series_styles_survive_a_multi_series_plot() {
    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let y1 = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let y2 = vec![6.0, 5.0, 4.0, 3.0, 2.0, 1.0];
    let y3 = vec![3.0, 3.5, 3.2, 3.8, 3.6, 4.0];

    let render = |style_2: LineStyle, style_3: LineStyle| {
        Plot::new()
            .size_px(400, 300)
            .line(&x, &y1)
            .ticks(false)
            .grid(false)
            .line_style(LineStyle::Solid)
            .color(Color::from_rgb(255, 0, 0))
            .label("Solid")
            .line(&x, &y2)
            .line_style(style_2)
            .color(Color::from_rgb(0, 255, 0))
            .label("Second")
            .line(&x, &y3)
            .line_style(style_3)
            .color(Color::from_rgb(0, 0, 255))
            .label("Third")
            .legend(LegendPosition::UpperRight)
            .render()
            .expect("a multi-series plot must render")
    };

    let mixed = render(LineStyle::Dashed, LineStyle::Dotted);
    let all_solid = render(LineStyle::Solid, LineStyle::Solid);

    assert!(
        mixed.pixels != all_solid.pixels,
        "a plot whose second and third series are Dashed and Dotted must not \
         match the same plot drawn entirely with solid lines"
    );
    assert!(
        ink_pixels(&mixed) < ink_pixels(&all_solid),
        "the dashed and dotted series must cover less than their solid counterparts"
    );
}
