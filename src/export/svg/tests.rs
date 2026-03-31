use super::*;

fn parse_svg_attr(line: &str, attr: &str) -> f32 {
    let marker = format!(r#"{}=""#, attr);
    let start = line
        .find(&marker)
        .unwrap_or_else(|| panic!("missing {} in line: {}", attr, line))
        + marker.len();
    let end = line[start..]
        .find('"')
        .unwrap_or_else(|| panic!("unterminated {} in line: {}", attr, line))
        + start;
    line[start..end]
        .parse::<f32>()
        .unwrap_or_else(|_| panic!("invalid {} in line: {}", attr, line))
}

fn extract_svg_text_xy(svg: &str, text: &str) -> (f32, f32) {
    let marker = format!(">{}</text>", text);
    let line = svg
        .lines()
        .find(|line| line.contains(&marker))
        .unwrap_or_else(|| panic!("missing text node for {}", text));
    (parse_svg_attr(line, "x"), parse_svg_attr(line, "y"))
}

fn has_svg_line(svg: &str, x1: f32, y1: f32, x2: f32, y2: f32) -> bool {
    svg.lines()
        .filter(|line| line.contains("<line"))
        .any(|line| {
            (parse_svg_attr(line, "x1") - x1).abs() <= 0.01
                && (parse_svg_attr(line, "y1") - y1).abs() <= 0.01
                && (parse_svg_attr(line, "x2") - x2).abs() <= 0.01
                && (parse_svg_attr(line, "y2") - y2).abs() <= 0.01
        })
}

fn extract_typst_group_translates(svg: &str) -> Vec<(f32, f32)> {
    svg.lines()
        .filter(|line| line.contains(r#"data-ruviz-text-engine="typst""#))
        .map(|line| {
            let marker = r#"transform="translate("#;
            let start = line
                .find(marker)
                .unwrap_or_else(|| panic!("missing translate transform in line: {}", line))
                + marker.len();
            let end = line[start..]
                .find(')')
                .unwrap_or_else(|| panic!("unterminated translate transform in line: {}", line))
                + start;
            let coords = &line[start..end];
            let mut parts = coords.split(',');
            let x = parts
                .next()
                .unwrap_or_else(|| panic!("missing translate x in line: {}", line))
                .parse::<f32>()
                .unwrap_or_else(|_| panic!("invalid translate x in line: {}", line));
            let y = parts
                .next()
                .unwrap_or_else(|| panic!("missing translate y in line: {}", line))
                .parse::<f32>()
                .unwrap_or_else(|_| panic!("invalid translate y in line: {}", line));
            (x, y)
        })
        .collect()
}

#[test]
fn test_svg_renderer_creation() {
    let renderer = SvgRenderer::new(800.0, 600.0);
    assert_eq!(renderer.width(), 800.0);
    assert_eq!(renderer.height(), 600.0);
}

#[test]
fn test_color_conversion() {
    let renderer = SvgRenderer::new(100.0, 100.0);
    assert_eq!(renderer.color_to_svg(Color::new(255, 0, 0)), "rgb(255,0,0)");
    assert_eq!(
        renderer.color_to_svg(Color::new_rgba(255, 0, 0, 128)),
        "rgba(255,0,0,0.502)"
    );
}

#[test]
fn test_line_style_conversion() {
    let renderer = SvgRenderer::new(100.0, 100.0);
    assert_eq!(renderer.line_style_to_dasharray(&LineStyle::Solid), None);
    assert_eq!(
        renderer.line_style_to_dasharray(&LineStyle::Dashed),
        Some("5,5".to_string())
    );
}

#[test]
fn test_line_style_conversion_scales_with_dpi() {
    let mut renderer = SvgRenderer::new(100.0, 100.0);
    renderer.set_dpi_scale(2.0);

    assert_eq!(
        renderer.line_style_to_dasharray(&LineStyle::Dashed),
        Some("10,10".to_string())
    );
    assert_eq!(
        renderer.line_style_to_dasharray(&LineStyle::Dotted),
        Some("2,4".to_string())
    );
    assert_eq!(
        renderer.line_style_to_dasharray(&LineStyle::Custom(vec![1.5, 2.0])),
        Some("3,4".to_string())
    );
}

#[test]
fn test_set_dpi_scale_sanitizes_invalid_values() {
    let mut renderer = SvgRenderer::new(100.0, 100.0);

    renderer.set_dpi_scale(2.5);
    assert!((renderer.dpi_scale() - 2.5).abs() < f32::EPSILON);

    renderer.set_dpi_scale(0.0);
    assert!((renderer.dpi_scale() - 1.0).abs() < f32::EPSILON);
    assert_eq!(
        renderer.line_style_to_dasharray(&LineStyle::Dashed),
        Some("5,5".to_string())
    );

    renderer.set_dpi_scale(-3.0);
    assert!((renderer.dpi_scale() - 1.0).abs() < f32::EPSILON);

    renderer.set_dpi_scale(f32::NAN);
    assert!((renderer.dpi_scale() - 1.0).abs() < f32::EPSILON);

    renderer.set_dpi_scale(f32::INFINITY);
    assert!((renderer.dpi_scale() - 1.0).abs() < f32::EPSILON);
}

#[test]
fn test_xml_escaping() {
    let renderer = SvgRenderer::new(100.0, 100.0);
    assert_eq!(
        renderer.escape_xml("a < b & c > d"),
        "a &lt; b &amp; c &gt; d"
    );
}

#[test]
fn test_svg_output() {
    let mut renderer = SvgRenderer::new(200.0, 150.0);
    renderer.draw_rectangle(0.0, 0.0, 200.0, 150.0, Color::WHITE, true);
    renderer.draw_line(
        10.0,
        10.0,
        190.0,
        140.0,
        Color::BLACK,
        2.0,
        LineStyle::Solid,
    );

    let svg = renderer.to_svg_string();
    assert!(svg.contains("svg"));
    assert!(svg.contains("rect"));
    assert!(svg.contains("line"));
}

#[test]
fn test_polygon_outline_requires_three_points() {
    let mut renderer = SvgRenderer::new(200.0, 150.0);
    renderer.draw_polygon_outline(&[(10.0, 10.0), (20.0, 20.0)], Color::BLACK, 2.0);

    let svg = renderer.to_svg_string();
    assert!(
        !svg.contains("<polygon"),
        "two-point polygon outlines should be ignored to match raster rendering"
    );
}

#[test]
fn test_legend_line_handle_attribute_spacing() {
    let mut renderer = SvgRenderer::new(200.0, 150.0);

    renderer.draw_legend_line_handle(10.0, 20.0, 30.0, Color::BLACK, &LineStyle::Solid, 1.5);
    renderer.draw_legend_line_handle(10.0, 30.0, 30.0, Color::BLACK, &LineStyle::Dashed, 1.5);

    let svg = renderer.to_svg_string();
    let line_nodes: Vec<&str> = svg.lines().filter(|line| line.contains("<line")).collect();

    assert!(
        line_nodes
            .iter()
            .any(|line| line.contains(r#"stroke-width="1.5"/>"#)),
        "solid legend handle should not include dangling whitespace before '/>'"
    );
    assert!(
        line_nodes
            .iter()
            .any(|line| { line.contains(r#"stroke-width="1.5" stroke-dasharray="5,5""#) }),
        "dashed legend handle should include dash attribute with one leading space"
    );
    assert!(
        !line_nodes
            .iter()
            .any(|line| line.contains(r#"stroke-width="1.5" />"#)),
        "legend handle should not emit dangling whitespace before '/>'"
    );
}

#[test]
fn test_plain_text_uses_top_origin_baseline() {
    let mut renderer = SvgRenderer::new(200.0, 150.0);
    renderer
        .draw_text("Top Origin", 10.0, 20.0, 12.0, Color::BLACK)
        .unwrap();
    renderer
        .draw_text_centered("Centered Top", 100.0, 25.0, 12.0, Color::BLACK)
        .unwrap();

    let svg = renderer.to_svg_string();
    assert!(!svg.contains("dominant-baseline=\"text-before-edge\""));

    let (x1, y1) = extract_svg_text_xy(&svg, "Top Origin");
    let (x2, y2) = extract_svg_text_xy(&svg, "Centered Top");
    let metrics1 = renderer.plain_text_metrics("Top Origin", 12.0).unwrap();
    let metrics2 = renderer.plain_text_metrics("Centered Top", 12.0).unwrap();

    assert!((x1 - 10.0).abs() <= 0.01);
    assert!((x2 - 100.0).abs() <= 0.01);
    assert!((y1 - top_anchor_to_baseline(20.0, metrics1)).abs() <= 0.6);
    assert!((y2 - top_anchor_to_baseline(25.0, metrics2)).abs() <= 0.6);
}

#[test]
fn test_tick_labels_use_layout_positions() {
    let mut renderer = SvgRenderer::new(200.0, 150.0);
    let x_ticks = vec![100.0];
    let x_labels = vec!["1.0".to_string()];
    let y_ticks = vec![75.0];
    let y_labels = vec!["2.0".to_string()];

    renderer
        .draw_tick_labels(
            &x_ticks,
            &x_labels,
            &y_ticks,
            &y_labels,
            40.0,
            160.0,
            20.0,
            120.0,
            120.0,
            35.0,
            Color::BLACK,
            10.0,
        )
        .unwrap();

    let svg = renderer.to_svg_string();
    let (x_tick_x, x_tick_y) = extract_svg_text_xy(&svg, "1.0");
    let (y_tick_x, y_tick_y) = extract_svg_text_xy(&svg, "2.0");
    let x_metrics = renderer.plain_text_metrics("1.0", 10.0).unwrap();
    let y_metrics = renderer.plain_text_metrics("2.0", 10.0).unwrap();

    let x_top = 120.0;
    let expected_x_tick_x = (100.0 - x_metrics.width / 2.0)
        .max(0.0)
        .min(renderer.width - x_metrics.width);
    let expected_x_tick_y = top_anchor_to_baseline(x_top, x_metrics);

    let y_top = 75.0 - y_metrics.height / 2.0;
    let expected_y_tick_x = (35.0 - y_metrics.width - 5.0).max(5.0);
    let expected_y_tick_y = top_anchor_to_baseline(y_top, y_metrics);

    assert!(
        (x_tick_x - expected_x_tick_x).abs() <= 0.6 && (x_tick_y - expected_x_tick_y).abs() <= 0.6,
        "x-axis tick label should use layout xtick baseline"
    );
    assert!(
        (y_tick_x - expected_y_tick_x).abs() <= 0.6 && (y_tick_y - expected_y_tick_y).abs() <= 0.6,
        "y-axis tick label should use layout ytick anchor and centered y"
    );
}

#[test]
fn test_draw_axes_renders_ticks_on_all_sides() {
    let mut renderer = SvgRenderer::new(200.0, 150.0);
    renderer.draw_axes(
        40.0,
        160.0,
        20.0,
        120.0,
        &[100.0],
        &[75.0],
        &TickDirection::Inside,
        &TickSides::all(),
        Color::BLACK,
    );

    let svg = renderer.to_svg_string();
    assert!(has_svg_line(&svg, 40.0, 20.0, 160.0, 20.0));
    assert!(has_svg_line(&svg, 160.0, 20.0, 160.0, 120.0));
    assert!(has_svg_line(&svg, 100.0, 120.0, 100.0, 114.0));
    assert!(has_svg_line(&svg, 100.0, 20.0, 100.0, 26.0));
    assert!(has_svg_line(&svg, 40.0, 75.0, 46.0, 75.0));
    assert!(has_svg_line(&svg, 160.0, 75.0, 154.0, 75.0));
}

#[test]
fn test_draw_axes_respects_bottom_left_tick_selection() {
    let mut renderer = SvgRenderer::new(200.0, 150.0);
    renderer.draw_axes(
        40.0,
        160.0,
        20.0,
        120.0,
        &[100.0],
        &[75.0],
        &TickDirection::Inside,
        &TickSides::bottom_left(),
        Color::BLACK,
    );

    let svg = renderer.to_svg_string();
    assert!(has_svg_line(&svg, 40.0, 20.0, 160.0, 20.0));
    assert!(has_svg_line(&svg, 160.0, 20.0, 160.0, 120.0));
    assert!(has_svg_line(&svg, 100.0, 120.0, 100.0, 114.0));
    assert!(has_svg_line(&svg, 40.0, 75.0, 46.0, 75.0));
    assert!(!has_svg_line(&svg, 100.0, 20.0, 100.0, 26.0));
    assert!(!has_svg_line(&svg, 160.0, 75.0, 154.0, 75.0));
}

#[cfg(feature = "typst-math")]
#[test]
fn test_typst_tick_labels_follow_plain_anchor_math() {
    let mut renderer = SvgRenderer::new(200.0, 150.0);
    renderer.set_text_engine_mode(TextEngineMode::Typst);
    let x_ticks = vec![100.0];
    let x_labels = vec!["1.0".to_string()];
    let y_ticks = vec![75.0];
    let y_labels = vec!["2.0".to_string()];

    renderer
        .draw_tick_labels(
            &x_ticks,
            &x_labels,
            &y_ticks,
            &y_labels,
            40.0,
            160.0,
            20.0,
            120.0,
            120.0,
            35.0,
            Color::BLACK,
            10.0,
        )
        .unwrap();

    let svg = renderer.to_svg_string();
    assert!(svg.contains("data-ruviz-text-engine=\"typst\""));

    let translates = extract_typst_group_translates(&svg);
    assert_eq!(translates.len(), 2, "expected two typst text groups");

    let x_snippet = typst_text::literal_text_snippet("1.0");
    let y_snippet = typst_text::literal_text_snippet("2.0");
    let (x_w, _x_h) = typst_text::measure_text(
        &x_snippet,
        10.0,
        Color::BLACK,
        0.0,
        TypstBackendKind::Svg,
        "typst tick test",
    )
    .unwrap();
    let (y_w, y_h) = typst_text::measure_text(
        &y_snippet,
        10.0,
        Color::BLACK,
        0.0,
        TypstBackendKind::Svg,
        "typst tick test",
    )
    .unwrap();

    let expected_x = (100.0 - x_w / 2.0).max(0.0).min(renderer.width - x_w);
    let expected_y = (35.0 - y_w - 5.0).max(5.0);
    let expected_y_top = 75.0 - y_h / 2.0;

    assert!((translates[0].0 - expected_x).abs() <= 0.6 && (translates[0].1 - 120.0).abs() <= 0.6);
    assert!(
        (translates[1].0 - expected_y).abs() <= 0.6
            && (translates[1].1 - expected_y_top).abs() <= 0.6
    );
}

#[cfg(feature = "typst-math")]
#[test]
fn test_typst_rotated_text_uses_typst_rotation_path() {
    let mut renderer = SvgRenderer::new(200.0, 150.0);
    renderer.set_text_engine_mode(TextEngineMode::Typst);
    renderer
        .draw_text_rotated("Y Axis", 100.0, 75.0, 12.0, Color::BLACK, -90.0)
        .unwrap();

    let svg = renderer.to_svg_string();
    assert!(svg.contains("data-ruviz-text-engine=\"typst\""));
    assert!(!svg.contains("data-ruviz-text-engine=\"typst\" transform=\"rotate("));
}
