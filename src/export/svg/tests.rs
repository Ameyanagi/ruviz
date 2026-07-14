use super::*;
use crate::core::TextVAlign;

fn svg_attr_value<'a>(line: &'a str, attr: &str) -> &'a str {
    let marker = format!(r#"{}=""#, attr);
    let start = line
        .find(&marker)
        .unwrap_or_else(|| panic!("missing {} in line: {}", attr, line))
        + marker.len();
    let end = line[start..]
        .find('"')
        .unwrap_or_else(|| panic!("unterminated {} in line: {}", attr, line))
        + start;
    &line[start..end]
}

fn parse_svg_attr(line: &str, attr: &str) -> f32 {
    svg_attr_value(line, attr)
        .parse::<f32>()
        .unwrap_or_else(|_| panic!("invalid {} in line: {}", attr, line))
}

fn svg_element_lines<'a>(svg: &'a str, tag: &str) -> Vec<&'a str> {
    let prefix = format!("<{tag} ");
    svg.lines()
        .filter(|line| line.trim_start().starts_with(&prefix))
        .collect()
}

fn parse_svg_points(line: &str) -> Vec<(f32, f32)> {
    svg_attr_value(line, "points")
        .split_whitespace()
        .map(|point| {
            let (x, y) = point
                .split_once(',')
                .unwrap_or_else(|| panic!("invalid point {point} in line: {line}"));
            (
                x.parse::<f32>()
                    .unwrap_or_else(|_| panic!("invalid point x {x} in line: {line}")),
                y.parse::<f32>()
                    .unwrap_or_else(|_| panic!("invalid point y {y} in line: {line}")),
            )
        })
        .collect()
}

fn assert_approx_eq(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 0.01,
        "expected {expected}, got {actual}"
    );
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
    svg_element_lines(svg, "line").into_iter().any(|line| {
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
fn test_plain_svg_generic_font_families_are_unquoted() {
    let cases = [
        (FontFamily::Serif, "serif"),
        (FontFamily::SansSerif, "sans-serif"),
        (FontFamily::Monospace, "monospace"),
        (FontFamily::Cursive, "cursive"),
        (FontFamily::Fantasy, "fantasy"),
    ];

    for (family, expected) in cases {
        let mut renderer = SvgRenderer::with_font_family(100.0, 100.0, family);
        renderer
            .draw_text("Label", 10.0, 20.0, 12.0, Color::BLACK)
            .unwrap();

        let svg = renderer.to_svg_string();
        assert!(
            svg.contains(&format!(r#"font-family="{expected}""#)),
            "unexpected SVG font family: {svg}"
        );
    }
}

#[test]
fn test_plain_svg_named_font_family_is_css_quoted() {
    let mut renderer = SvgRenderer::with_font_family(
        100.0,
        100.0,
        FontFamily::Name("New Computer Modern Sans".to_string()),
    );
    renderer
        .draw_text("Label", 10.0, 20.0, 12.0, Color::BLACK)
        .unwrap();

    let svg = renderer.to_svg_string();
    assert!(svg.contains(r#"font-family="&quot;New Computer Modern Sans&quot;""#));
}

#[test]
fn test_plain_svg_named_generic_keyword_remains_quoted() {
    let mut renderer =
        SvgRenderer::with_font_family(100.0, 100.0, FontFamily::Name("serif".to_string()));
    renderer
        .draw_text("Label", 10.0, 20.0, 12.0, Color::BLACK)
        .unwrap();

    let svg = renderer.to_svg_string();
    assert!(svg.contains(r#"font-family="&quot;serif&quot;""#));
}

#[test]
fn test_plain_svg_named_font_family_css_and_xml_escaping() {
    let mut renderer = SvgRenderer::with_font_family(
        100.0,
        100.0,
        FontFamily::Name(r#"Font, "A"\B & <C>"#.to_string()),
    );
    renderer
        .draw_text("Label", 10.0, 20.0, 12.0, Color::BLACK)
        .unwrap();

    let svg = renderer.to_svg_string();
    assert!(svg.contains(r#"font-family="&quot;Font, \&quot;A\&quot;\\B &amp; &lt;C&gt;&quot;""#));
}

#[test]
fn test_plain_svg_named_font_family_escapes_control_characters() {
    let mut renderer = SvgRenderer::with_font_family(
        100.0,
        100.0,
        FontFamily::Name(
            "Line\nBreak\rReturn\tTab\u{000C}Form\0Null\u{0001}Unit\u{007F}Delete\u{FFFE}End"
                .to_string(),
        ),
    );
    renderer
        .draw_text("Label", 10.0, 20.0, 12.0, Color::BLACK)
        .unwrap();

    let svg = renderer.to_svg_string();
    let text_line = svg
        .lines()
        .find(|line| line.contains(">Label</text>"))
        .expect("missing SVG text element");
    let family = svg_attr_value(text_line, "font-family");

    assert_eq!(
        family,
        r#"&quot;Line\00000ABreak\00000DReturn\000009Tab\00000CForm�Null\000001Unit\00007FDelete\00FFFEEnd&quot;"#
    );
    assert!(
        family
            .chars()
            .all(|character| character >= ' ' && character != '\u{007F}')
    );
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
fn test_triangle_down_marker_uses_skia_geometry() {
    let mut renderer = SvgRenderer::new(100.0, 100.0);
    renderer.draw_marker(20.0, 30.0, 10.0, MarkerStyle::TriangleDown, Color::RED);

    let svg = renderer.to_svg_string();
    let polygons = svg_element_lines(&svg, "polygon");
    assert_eq!(polygons.len(), 1);
    let polygon = polygons[0];
    let points = parse_svg_points(polygon);
    assert_eq!(points.len(), 3);
    for ((actual_x, actual_y), (expected_x, expected_y)) in
        points
            .into_iter()
            .zip([(20.0, 35.0), (15.67, 27.5), (24.33, 27.5)])
    {
        assert_approx_eq(actual_x, expected_x);
        assert_approx_eq(actual_y, expected_y);
    }
    assert_eq!(svg_attr_value(polygon, "fill"), "rgb(255,0,0)");
    assert!(svg_element_lines(&svg, "circle").is_empty());
}

#[test]
fn test_star_marker_uses_skia_geometry() {
    let mut renderer = SvgRenderer::new(100.0, 100.0);
    renderer.draw_marker(20.0, 30.0, 10.0, MarkerStyle::Star, Color::BLUE);

    let svg = renderer.to_svg_string();
    let lines = svg_element_lines(&svg, "line");

    assert_eq!(lines.len(), 4);
    assert!(has_svg_line(&svg, 15.0, 30.0, 25.0, 30.0));
    assert!(has_svg_line(&svg, 20.0, 25.0, 20.0, 35.0));
    assert!(has_svg_line(&svg, 16.47, 26.47, 23.53, 33.53));
    assert!(has_svg_line(&svg, 16.47, 33.53, 23.53, 26.47));
    assert!(lines.iter().all(|line| {
        (parse_svg_attr(line, "stroke-width") - 2.2).abs() <= 0.01
            && svg_attr_value(line, "stroke-linecap") == "butt"
    }));
    assert!(svg_element_lines(&svg, "circle").is_empty());
    assert!(svg_element_lines(&svg, "polygon").is_empty());
}

#[test]
fn test_marker_rendering_is_exhaustive_and_matches_supported_svg_primitives() {
    for style in [
        MarkerStyle::Circle,
        MarkerStyle::Square,
        MarkerStyle::Triangle,
        MarkerStyle::TriangleDown,
        MarkerStyle::Diamond,
        MarkerStyle::Plus,
        MarkerStyle::Cross,
        MarkerStyle::Star,
        MarkerStyle::CircleOpen,
        MarkerStyle::SquareOpen,
        MarkerStyle::TriangleOpen,
        MarkerStyle::DiamondOpen,
    ] {
        let mut renderer = SvgRenderer::new(100.0, 100.0);
        renderer.draw_marker(20.0, 30.0, 10.0, style, Color::BLACK);
        let svg = renderer.to_svg_string();

        let (tag, expected_count) = match style {
            MarkerStyle::Circle | MarkerStyle::CircleOpen => ("circle", 1),
            MarkerStyle::Square | MarkerStyle::SquareOpen => ("rect", 1),
            MarkerStyle::Triangle
            | MarkerStyle::TriangleDown
            | MarkerStyle::Diamond
            | MarkerStyle::TriangleOpen
            | MarkerStyle::DiamondOpen => ("polygon", 1),
            MarkerStyle::Plus | MarkerStyle::Cross => ("line", 2),
            MarkerStyle::Star => ("line", 4),
        };

        assert_eq!(
            svg_element_lines(&svg, tag).len(),
            expected_count,
            "unexpected SVG primitive for {} marker: {svg}",
            style.name()
        );

        match style {
            MarkerStyle::CircleOpen | MarkerStyle::SquareOpen => {
                let element = svg_element_lines(&svg, tag)[0];
                assert_eq!(svg_attr_value(element, "fill"), "none");
                assert_eq!(svg_attr_value(element, "stroke"), "rgb(0,0,0)");
                assert_approx_eq(parse_svg_attr(element, "stroke-width"), 1.0);
            }
            MarkerStyle::TriangleOpen | MarkerStyle::DiamondOpen => {
                let element = svg_element_lines(&svg, tag)[0];
                assert_eq!(svg_attr_value(element, "fill"), "none");
                assert_eq!(svg_attr_value(element, "stroke"), "rgb(0,0,0)");
                assert_approx_eq(parse_svg_attr(element, "stroke-width"), 1.5);
            }
            MarkerStyle::Circle
            | MarkerStyle::Square
            | MarkerStyle::Triangle
            | MarkerStyle::TriangleDown
            | MarkerStyle::Diamond
            | MarkerStyle::Plus
            | MarkerStyle::Cross
            | MarkerStyle::Star => {}
        }
    }
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
fn test_legend_line_width_and_marker_size_scale_at_2x() {
    fn render_legend_at(scale: f32) -> (Vec<f32>, Vec<f32>) {
        let mut renderer = SvgRenderer::new(400.0, 300.0);
        renderer.set_dpi_scale(scale);
        let items = [
            LegendItem::line("line", Color::BLACK, LineStyle::Solid, 3.6),
            LegendItem::scatter("scatter", Color::BLACK, MarkerStyle::Circle, 7.2),
            LegendItem::line_marker(
                "line-marker",
                Color::BLACK,
                LineStyle::Solid,
                1.8,
                MarkerStyle::Circle,
                3.6,
            ),
        ];
        let legend = Legend::upper_right().style(LegendStyle::invisible());

        renderer
            .draw_legend_full(&items, &legend, (0.0, 0.0, 400.0, 300.0), None)
            .unwrap();

        let svg = renderer.to_svg_string();
        let line_widths = svg_element_lines(&svg, "line")
            .into_iter()
            .map(|line| parse_svg_attr(line, "stroke-width"))
            .collect();
        let marker_sizes = svg_element_lines(&svg, "circle")
            .into_iter()
            .map(|marker| parse_svg_attr(marker, "r") * 2.0)
            .collect();
        (line_widths, marker_sizes)
    }

    let (line_widths_1x, marker_sizes_1x) = render_legend_at(1.0);
    let (line_widths_2x, marker_sizes_2x) = render_legend_at(2.0);

    assert_eq!(line_widths_1x.len(), 2);
    assert_eq!(marker_sizes_1x.len(), 2);
    assert_eq!(line_widths_2x.len(), 2);
    assert_eq!(marker_sizes_2x.len(), 2);
    for (actual, expected) in line_widths_1x.into_iter().zip([5.0, 2.5]) {
        assert_approx_eq(actual, expected);
    }
    for (actual, expected) in marker_sizes_1x.into_iter().zip([10.0, 5.0]) {
        assert_approx_eq(actual, expected);
    }
    for (actual, expected) in line_widths_2x.into_iter().zip([10.0, 5.0]) {
        assert_approx_eq(actual, expected);
    }
    for (actual, expected) in marker_sizes_2x.into_iter().zip([20.0, 10.0]) {
        assert_approx_eq(actual, expected);
    }
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
    let expected_y_tick_x = (35.0 - y_metrics.width).max(0.0);
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
    let expected_y = (35.0 - y_w).max(0.0);
    let expected_y_top = 75.0 - y_h / 2.0;

    assert!((translates[0].0 - expected_x).abs() <= 0.6 && (translates[0].1 - 120.0).abs() <= 0.6);
    assert!(
        (translates[1].0 - expected_y).abs() <= 0.6
            && (translates[1].1 - expected_y_top).abs() <= 0.6
    );
}

#[test]
fn styled_text_defaults_to_center_middle_and_scales_decoration() {
    let render = |dpi| {
        let mut renderer = SvgRenderer::new(240.0, 160.0);
        renderer.set_render_scale(RenderScale::new(dpi));
        let style = TextStyle {
            font_size: 12.0,
            color: Color::BLACK,
            align: TextAlign::Center,
            valign: TextVAlign::Middle,
            rotation: 0.0,
            background: Some(Color::new_rgba(255, 0, 0, 128)),
            padding: 3.0,
            border_color: Some(Color::BLUE),
            border_width: 1.5,
        };
        renderer
            .draw_styled_text("Anchor", 120.0, 80.0, &FontFamily::SansSerif, &style)
            .unwrap();
        renderer.to_svg_string()
    };

    let low = render(72.0);
    let high = render(144.0);
    let low_rect = svg_element_lines(&low, "rect")
        .into_iter()
        .find(|line| line.contains("rgba(255,0,0,0.502)"))
        .unwrap();
    let high_rect = svg_element_lines(&high, "rect")
        .into_iter()
        .find(|line| line.contains("rgba(255,0,0,0.502)"))
        .unwrap();

    let low_x = parse_svg_attr(low_rect, "x");
    let low_y = parse_svg_attr(low_rect, "y");
    let low_width = parse_svg_attr(low_rect, "width");
    let low_height = parse_svg_attr(low_rect, "height");
    let high_width = parse_svg_attr(high_rect, "width");
    let high_height = parse_svg_attr(high_rect, "height");
    assert_approx_eq(low_x + low_width / 2.0, 0.0);
    assert_approx_eq(low_y + low_height / 2.0, 0.0);
    assert!(high_width > low_width * 1.8);
    assert!(high_height > low_height * 1.8);
    assert_eq!(svg_attr_value(low_rect, "stroke-width"), "1.50");
    assert_eq!(svg_attr_value(high_rect, "stroke-width"), "3.00");
    assert!(low.contains(r#"text-anchor="middle""#));
}

#[test]
fn styled_text_honors_alignment_counter_clockwise_rotation_and_font_family() {
    let mut renderer = SvgRenderer::new(240.0, 160.0);
    renderer.set_render_scale(RenderScale::new(144.0));
    let style = TextStyle {
        font_size: 10.0,
        color: Color::new_rgba(10, 20, 30, 200),
        align: TextAlign::Right,
        valign: TextVAlign::Bottom,
        rotation: 30.0,
        background: Some(Color::new(240, 230, 220)),
        padding: 2.0,
        border_color: Some(Color::new_rgba(40, 50, 60, 128)),
        border_width: 2.0,
    };
    renderer
        .draw_styled_text(
            "Styled",
            100.0,
            70.0,
            &FontFamily::Name("New Computer Modern Sans".to_string()),
            &style,
        )
        .unwrap();

    let svg = renderer.to_svg_string();
    assert!(svg.contains(r#"transform="translate(100.00,70.00) rotate(-30.00)""#));
    assert!(svg.contains(r#"text-anchor="end""#));
    assert!(svg.contains(r#"font-family="&quot;New Computer Modern Sans&quot;""#));
    assert!(svg.contains(r#"font-weight="400""#));
    assert!(svg.contains(r#"fill="rgba(10,20,30,0.784)""#));

    let rect = svg_element_lines(&svg, "rect")
        .into_iter()
        .find(|line| line.contains("rgb(240,230,220)"))
        .unwrap();
    let rect_x = parse_svg_attr(rect, "x");
    let rect_y = parse_svg_attr(rect, "y");
    let rect_width = parse_svg_attr(rect, "width");
    let rect_height = parse_svg_attr(rect, "height");
    assert_approx_eq(rect_x + rect_width, 4.0);
    assert_approx_eq(rect_y + rect_height, 4.0);
    assert_eq!(svg_attr_value(rect, "stroke-width"), "4.00");
}

#[test]
fn styled_multiline_text_uses_explicit_tspans_and_block_decoration() {
    let mut renderer = SvgRenderer::new(240.0, 160.0);
    renderer.set_render_scale(RenderScale::new(72.0));
    let style = TextStyle::default()
        .font_size(12.0)
        .color(Color::BLACK)
        .background(Color::new(240, 230, 220))
        .padding(2.0)
        .rotation(20.0);
    renderer
        .draw_styled_text(
            "first\nsecond\nthird",
            120.0,
            80.0,
            &FontFamily::SansSerif,
            &style,
        )
        .unwrap();

    let svg = renderer.to_svg_string();
    assert_eq!(svg.matches("<tspan ").count(), 3);
    assert!(svg.contains(r#"rotate(-20.00)"#));
    let rect = svg_element_lines(&svg, "rect")
        .into_iter()
        .find(|line| line.contains("rgb(240,230,220)"))
        .unwrap();
    assert!(parse_svg_attr(rect, "height") > 40.0);
}

#[test]
fn styled_multiline_text_preserves_per_line_center_and_right_alignment() {
    for (align, anchor) in [(TextAlign::Center, "middle"), (TextAlign::Right, "end")] {
        let mut renderer = SvgRenderer::new(240.0, 160.0);
        let style = TextStyle::default().align(align).font_size(12.0);
        renderer
            .draw_styled_text(
                "a much wider line\nshort",
                120.0,
                80.0,
                &FontFamily::SansSerif,
                &style,
            )
            .unwrap();

        let svg = renderer.to_svg_string();
        let text = svg
            .lines()
            .find(|line| line.contains("a much wider line"))
            .expect("multiline text element");
        assert!(text.contains(&format!(r#"text-anchor="{anchor}""#)));
        assert_eq!(text.matches(r#"<tspan x="0""#).count(), 2);
    }
}

#[test]
fn decorated_empty_and_whitespace_annotations_use_one_line_geometry_without_glyphs() {
    let render = |text: &str| {
        let mut renderer = SvgRenderer::new(200.0, 120.0);
        renderer.set_render_scale(RenderScale::new(72.0));
        let style = TextStyle::default()
            .font_size(12.0)
            .background(Color::RED)
            .padding(2.0)
            .border(Color::BLUE, 1.0);
        renderer
            .draw_styled_text(text, 100.0, 60.0, &FontFamily::SansSerif, &style)
            .unwrap();
        renderer.to_svg_string()
    };

    let empty = render("");
    let whitespace = render(" \t\n  ");
    assert_eq!(empty, whitespace);
    let rect = svg_element_lines(&empty, "rect")
        .into_iter()
        .find(|line| line.contains("rgb(255,0,0)"))
        .expect("decoration rectangle");
    assert_approx_eq(parse_svg_attr(rect, "width"), 4.0);
    assert_approx_eq(parse_svg_attr(rect, "height"), 16.0);
    assert!(!empty.contains("<text "));
    assert!(!empty.contains(r#"data-ruviz-text-engine="typst""#));
}

#[test]
fn styled_annotation_preserves_authored_svg_whitespace_without_formatting_text_nodes() {
    let mut renderer = SvgRenderer::new(200.0, 120.0);
    let style = TextStyle::default().align(TextAlign::Right);
    renderer
        .draw_styled_text(
            "  lead\t  gap\n \tsecond",
            100.0,
            60.0,
            &FontFamily::SansSerif,
            &style,
        )
        .unwrap();

    let svg = renderer.to_svg_string();
    let text = svg
        .lines()
        .find(|line| line.contains("lead\t  gap"))
        .expect("whitespace-preserving annotation text");
    assert!(text.contains(r#"xml:space="preserve""#));
    assert!(text.contains(">  lead\t  gap</tspan><tspan"));
    assert!(text.ends_with("> \tsecond</tspan></text>"));
    assert!(!text.contains("</tspan>      <tspan"));
}

#[test]
fn weighted_multiline_title_uses_matching_metrics_tspans_and_svg_whitespace() {
    let mut renderer = SvgRenderer::with_font_family(240.0, 160.0, FontFamily::Monospace);
    let text = "  wide\tgap\n  x";
    renderer
        .draw_text_centered_with_weight(text, 120.0, 20.0, 12.0, Color::BLACK, FontWeight::Bold)
        .unwrap();

    let config = FontConfig::new(FontFamily::Monospace, 12.0).weight(FontWeight::Bold);
    let metrics = renderer
        .plain_text_metrics_with_config(text, &config)
        .unwrap();
    let svg = renderer.to_svg_string();
    let title = svg
        .lines()
        .find(|line| line.contains("wide\tgap"))
        .expect("multiline title element");
    assert!(title.contains(r#"font-family="monospace""#));
    assert!(title.contains(r#"font-weight="700""#));
    assert!(title.contains(r#"text-anchor="middle""#));
    assert!(title.contains(r#"xml:space="preserve""#));
    assert!(title.contains(r#"><tspan x="120.00" y="#));
    assert!(title.contains(">  wide\tgap</tspan><tspan"));
    assert!(title.ends_with(">  x</tspan></text>"));
    assert_eq!(title.matches("<tspan ").count(), 2);

    let first_tspan = title.find("<tspan ").unwrap();
    let first_y = parse_svg_attr(&title[first_tspan..], "y");
    assert_approx_eq(first_y, top_anchor_to_baseline(20.0, metrics));
    assert!(metrics.height >= 24.0);
}

#[cfg(feature = "typst-math")]
#[test]
fn typst_decorated_whitespace_annotation_uses_plain_one_line_geometry() {
    let mut renderer = SvgRenderer::new(200.0, 120.0);
    renderer.set_render_scale(RenderScale::new(72.0));
    renderer.set_text_engine_mode(TextEngineMode::Typst);
    let style = TextStyle::default()
        .font_size(12.0)
        .background(Color::RED)
        .padding(2.0);
    renderer
        .draw_styled_text(" \t\n ", 100.0, 60.0, &FontFamily::SansSerif, &style)
        .unwrap();

    let svg = renderer.to_svg_string();
    let rect = svg_element_lines(&svg, "rect")
        .into_iter()
        .find(|line| line.contains("rgb(255,0,0)"))
        .expect("Typst-mode decoration rectangle");
    assert_approx_eq(parse_svg_attr(rect, "width"), 4.0);
    assert_approx_eq(parse_svg_attr(rect, "height"), 16.0);
    assert!(!svg.contains(r#"data-ruviz-text-engine="typst""#));
}

#[cfg(feature = "typst-math")]
#[test]
fn embedded_typst_svg_uses_outer_unitless_dimensions() {
    let renderer = SvgRenderer::new(200.0, 150.0);
    let rendered = typst_text::render_svg_with_font_family(
        "Frame",
        12.0,
        Color::BLACK,
        0.0,
        &FontFamily::SansSerif,
        "Typst SVG framing test",
    )
    .unwrap();
    let embedded = renderer.embedded_typst_svg(&rendered);
    let root = embedded
        .lines()
        .find(|line| line.contains("<svg"))
        .expect("embedded Typst SVG root");
    let width = svg_attr_value(root, "width");
    let height = svg_attr_value(root, "height");

    assert!(!width.ends_with("pt"));
    assert!(!height.ends_with("pt"));
    assert_approx_eq(width.parse().unwrap(), rendered.width);
    assert_approx_eq(height.parse().unwrap(), rendered.height);
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
