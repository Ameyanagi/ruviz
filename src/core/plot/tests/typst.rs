use super::*;

#[cfg(feature = "typst-math")]
#[test]
fn test_typst_toggle_mode_switch() {
    let plot = Plot::new().typst(true);
    assert_eq!(plot.display.text_engine, TextEngineMode::Typst);

    let plot = plot.typst(false);
    assert_eq!(plot.display.text_engine, TextEngineMode::Plain);
}

#[cfg(feature = "typst-math")]
#[test]
fn test_plot_builder_typst_forwarding() {
    let x = vec![0.0, 1.0, 2.0];
    let y = vec![1.0, 2.0, 3.0];

    let plot: Plot = Plot::new().line(&x, &y).typst(true).into();
    assert_eq!(plot.display.text_engine, TextEngineMode::Typst);
}

#[cfg(feature = "typst-math")]
#[test]
fn test_plot_series_builder_typst_forwarding() {
    let x = vec![0.0, 1.0, 2.0];
    let y = vec![1.0, 2.0, 3.0];

    let plot = Plot::new()
        .line(&x, &y)
        .label("Series")
        .typst(true)
        .end_series();
    assert_eq!(plot.display.text_engine, TextEngineMode::Typst);
}

#[cfg(feature = "typst-math")]
#[test]
fn test_invalid_typst_snippet_returns_typst_error() {
    let x = vec![0.0, 1.0, 2.0];
    let y = vec![1.0, 2.0, 3.0];

    let result = Plot::new()
        .line(&x, &y)
        .title("#let broken =")
        .typst(true)
        .render();

    assert!(matches!(result, Err(PlottingError::TypstError(_))));
}

#[cfg(feature = "typst-math")]
#[test]
fn test_plot_to_typst_svg_applies_font_family() {
    let x = vec![0.0, 1.0];
    let y = vec![0.0, 1.0];
    let render = |font_family| {
        Plot::new()
            .line(&x, &y)
            .title("iiiiiiiiWWWW")
            .font_family(font_family)
            .typst(true)
            .render_to_svg()
            .expect("Typst SVG plot render should succeed")
    };

    let serif = render(crate::render::FontFamily::Name(
        "Libertinus Serif".to_string(),
    ));
    let mono = render(crate::render::FontFamily::Name(
        "DejaVu Sans Mono".to_string(),
    ));

    assert!(serif.contains(r#"data-ruviz-text-engine="typst""#));
    assert!(mono.contains(r#"data-ruviz-text-engine="typst""#));
    assert_ne!(serif, mono);
}

#[cfg(feature = "typst-math")]
#[test]
fn test_typst_title_honors_resolved_title_weight() {
    let x = vec![0.0, 1.0];
    let y = vec![0.0, 1.0];
    let render = |weight| {
        let config = PlotConfig::builder()
            .typography(|typography| typography.title_weight(weight))
            .build();
        Plot::new()
            .plot_config(config)
            .line(&x, &y)
            .title("Weighted title")
            .typst(true)
            .render_to_svg()
            .expect("Typst SVG plot render should succeed")
    };

    let normal = render(crate::render::FontWeight::Normal);
    let bold = render(crate::render::FontWeight::Bold);
    assert_ne!(normal, bold);
}

#[cfg(feature = "typst-math")]
#[test]
fn test_typst_annotation_ignores_title_weight() {
    let render = |weight| {
        let config = PlotConfig::builder()
            .typography(|typography| typography.title_weight(weight))
            .build();
        Plot::new()
            .plot_config(config)
            .line(&[0.0, 1.0], &[0.0, 1.0])
            .color(Color::RED)
            .text_styled(
                0.5,
                0.5,
                "Normal annotation",
                crate::core::TextStyle::default().color(Color::BLUE),
            )
            .typst(true)
            .render_to_svg()
            .expect("Typst SVG annotation render should succeed")
    };

    assert_eq!(
        render(crate::render::FontWeight::Normal),
        render(crate::render::FontWeight::Bold)
    );
}

#[cfg(feature = "typst-math")]
#[test]
fn test_typst_raster_title_honors_weight_without_changing_annotation_weight() {
    let normal = plot_with_weighted_title(crate::render::FontWeight::Normal)
        .typst(true)
        .render()
        .expect("normal-weight Typst raster");
    let bold = plot_with_weighted_title(crate::render::FontWeight::Bold)
        .typst(true)
        .render()
        .expect("bold Typst raster");

    assert!(differing_pixel_count(&normal, &bold) > 20);
    assert_eq!(
        blue_dominant_pixel_indices(&normal),
        blue_dominant_pixel_indices(&bold),
        "title weight must not leak into Typst annotation text"
    );
}

#[cfg(feature = "typst-math")]
#[test]
fn public_typst_title_plain_newline_renders_as_multiline_in_svg_and_raster() {
    let make_plot = |title: &str| {
        Plot::new()
            .line(&[0.0, 1.0], &[0.0, 1.0])
            .title(title)
            .end_series()
            .set_output_pixels(360, 280)
            .typst(true)
    };

    let multiline_plot = make_plot("Bold Title\nShort");
    let single_line_plot = make_plot("Bold Title Short");
    let multiline_svg = multiline_plot.render_to_svg().unwrap();
    let single_line_svg = single_line_plot.render_to_svg().unwrap();
    let multiline_title = extract_typst_group_boxes(&multiline_svg)
        .last()
        .copied()
        .expect("multiline Typst title group");
    let single_line_title = extract_typst_group_boxes(&single_line_svg)
        .last()
        .copied()
        .expect("single-line Typst title group");
    assert!(
        multiline_title.3 > single_line_title.3 * 1.5,
        "authored title newline must increase rendered height: multiline={multiline_title:?}, single={single_line_title:?}"
    );

    let multiline = multiline_plot.render().unwrap();
    let single_line = single_line_plot.render().unwrap();
    assert!(
        differing_pixel_count(&multiline, &single_line) > 100,
        "public Typst raster title must not collapse an authored newline to a space"
    );
}

#[cfg(feature = "typst-math")]
#[test]
fn public_typst_annotation_plain_newline_renders_multiline_with_alignment_and_rotation() {
    let render = |text: &str, align, rotation| {
        let style = crate::core::TextStyle::default()
            .font_size(16.0)
            .align(align)
            .rotation(rotation)
            .background(Color::new(240, 200, 180))
            .padding(3.0);
        Plot::new()
            .line(&[0.0, 1.0], &[0.0, 1.0])
            .color(Color::BLUE)
            .text_styled(0.5, 0.5, text, style)
            .end_series()
            .set_output_pixels(360, 280)
            .typst(true)
            .render()
            .unwrap()
    };

    for (align, rotation) in [
        (crate::core::TextAlign::Center, 0.0),
        (crate::core::TextAlign::Right, 27.0),
    ] {
        let multiline = render("WIDE CENTER\nx", align, rotation);
        let single_line = render("WIDE CENTER x", align, rotation);
        assert!(
            differing_pixel_count(&multiline, &single_line) > 100,
            "public Typst annotation newline collapsed for {align:?} at {rotation} degrees"
        );
    }
}

#[cfg(feature = "typst-math")]
#[test]
fn public_typst_multiline_title_preserves_inline_markup() {
    Plot::new()
        .line(&[0.0, 1.0], &[0.0, 1.0])
        .title("#emph[wide]\n$ x + 1 $")
        .typst(true)
        .render()
        .expect("multiline Typst markup should remain compilable");
}
