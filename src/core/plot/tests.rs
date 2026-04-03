use super::*;
use crate::core::FigureConfig;

#[derive(Debug)]
struct FailingIngestionData;

impl crate::data::NumericData1D for FailingIngestionData {
    fn len(&self) -> usize {
        3
    }

    fn try_collect_f64_with_policy(
        &self,
        _null_policy: crate::data::NullPolicy,
    ) -> crate::core::Result<Vec<f64>> {
        Err(PlottingError::DataExtractionFailed {
            source: "test::failing-ingestion".to_string(),
            message: "forced ingestion failure".to_string(),
        })
    }
}

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
        .unwrap_or_else(|_| panic!("invalid {} value in line: {}", attr, line))
}

fn extract_svg_text_xy(svg: &str, text: &str) -> (f32, f32) {
    let marker = format!(">{}</text>", text);
    let line = svg
        .lines()
        .find(|line| line.contains(&marker))
        .unwrap_or_else(|| panic!("missing text node for {}", text));
    (parse_svg_attr(line, "x"), parse_svg_attr(line, "y"))
}

fn extract_svg_group_translate_xy(svg: &str, text: &str) -> (f32, f32) {
    let marker = format!(">{}</text>", text);
    let line = svg
        .lines()
        .find(|line| line.contains(&marker))
        .unwrap_or_else(|| panic!("missing grouped text node for {}", text));
    let transform_marker = r#"transform="translate("#;
    let start = line
        .find(transform_marker)
        .unwrap_or_else(|| panic!("missing translate transform for {}", text))
        + transform_marker.len();
    let end = line[start..]
        .find(')')
        .unwrap_or_else(|| panic!("unterminated translate transform for {}", text))
        + start;
    let coords = &line[start..end];
    let mut parts = coords.split(',');
    let x = parts
        .next()
        .unwrap_or_else(|| panic!("missing translate x for {}", text))
        .parse::<f32>()
        .unwrap_or_else(|_| panic!("invalid translate x for {}", text));
    let y = parts
        .next()
        .unwrap_or_else(|| panic!("missing translate y for {}", text))
        .parse::<f32>()
        .unwrap_or_else(|_| panic!("invalid translate y for {}", text));
    (x, y)
}

#[test]
fn test_plot_series_static_source_helpers_materialize_values() {
    let mut series = PlotSeries {
        series_type: SeriesType::Line {
            x_data: PlotData::Static(vec![0.0, 1.0]),
            y_data: PlotData::Static(vec![1.0, 2.0]),
        },
        streaming_source: None,
        label: None,
        color: None,
        color_source: None,
        line_width: None,
        line_width_source: None,
        line_style: None,
        line_style_source: None,
        marker_style: None,
        marker_style_source: None,
        marker_size: None,
        marker_size_source: None,
        alpha: None,
        alpha_source: None,
        y_errors: None,
        x_errors: None,
        error_config: None,
        inset_layout: None,
        group_id: None,
    };

    series.set_color_source_value(Color::RED.into());
    series.set_line_width_source_value(0.01_f32.into());
    series.set_line_style_source_value(LineStyle::Dashed.into());
    series.set_marker_style_source_value(MarkerStyle::Square.into());
    series.set_marker_size_source_value(0.01_f32.into());
    series.set_alpha_source_value(1.5_f32.into());

    assert_eq!(series.color, Some(Color::RED));
    assert!(series.color_source.is_none());
    assert_eq!(series.line_width, Some(0.1));
    assert!(series.line_width_source.is_none());
    assert_eq!(series.line_style, Some(LineStyle::Dashed));
    assert!(series.line_style_source.is_none());
    assert_eq!(series.marker_style, Some(MarkerStyle::Square));
    assert!(series.marker_style_source.is_none());
    assert_eq!(series.marker_size, Some(0.1));
    assert!(series.marker_size_source.is_none());
    assert_eq!(series.alpha, Some(1.0));
    assert!(series.alpha_source.is_none());
}

#[test]
fn test_series_group_builder_static_source_setters_materialize_values() {
    let plot = Plot::new().group(|group| {
        group
            .color_source(Color::RED)
            .line_width_source(0.01_f32)
            .line_style_source(LineStyle::Dashed)
            .alpha_source(1.5_f32)
            .line(&[0.0, 1.0], &[1.0, 2.0])
    });

    let series = &plot.series_mgr.series[0];
    assert_eq!(series.color, Some(Color::RED));
    assert!(series.color_source.is_none());
    assert_eq!(series.line_width, Some(0.1));
    assert!(series.line_width_source.is_none());
    assert_eq!(series.line_style, Some(LineStyle::Dashed));
    assert!(series.line_style_source.is_none());
    assert_eq!(series.alpha, Some(1.0));
    assert!(series.alpha_source.is_none());
}

fn extract_svg_root_attr(svg: &str, attr: &str) -> f32 {
    let line = svg
        .lines()
        .find(|line| line.contains("<svg"))
        .unwrap_or_else(|| panic!("missing svg root"));
    parse_svg_attr(line, attr)
}

fn extract_first_svg_polyline_stroke_width(svg: &str) -> f32 {
    let line = svg
        .lines()
        .find(|line| line.contains("<polyline"))
        .unwrap_or_else(|| panic!("missing polyline element"));
    parse_svg_attr(line, "stroke-width")
}

fn extract_first_svg_line_stroke_width(svg: &str) -> f32 {
    let line = svg
        .lines()
        .find(|line| line.contains("<line"))
        .unwrap_or_else(|| panic!("missing line element"));
    parse_svg_attr(line, "stroke-width")
}

fn extract_first_stroked_svg_polygon_stroke_width(svg: &str) -> f32 {
    let line = svg
        .lines()
        .find(|line| line.contains("<polygon") && line.contains("stroke-width"))
        .unwrap_or_else(|| panic!("missing stroked polygon element"));
    parse_svg_attr(line, "stroke-width")
}

fn image_pixel_is_dark(image: &Image, x: u32, y: u32) -> bool {
    let idx = ((y * image.width + x) * 4) as usize;
    image.pixels[idx..idx + 3]
        .iter()
        .all(|channel| *channel < 220)
}

fn image_has_dark_pixel_near(image: &Image, x: u32, y: u32, radius: u32) -> bool {
    let x_start = x.saturating_sub(radius);
    let x_end = (x + radius).min(image.width.saturating_sub(1));
    let y_start = y.saturating_sub(radius);
    let y_end = (y + radius).min(image.height.saturating_sub(1));

    for sample_y in y_start..=y_end {
        for sample_x in x_start..=x_end {
            if image_pixel_is_dark(image, sample_x, sample_y) {
                return true;
            }
        }
    }

    false
}

fn compute_render_tick_probe_points(plot: &Plot) -> ((u32, u32), (u32, u32)) {
    let (x_min, x_max, y_min, y_max) = plot
        .calculate_data_bounds()
        .expect("data bounds should be available");
    let content = plot.create_plot_content(y_min, y_max);
    let layout_config = match &plot.display.config.margins {
        MarginConfig::ContentDriven {
            edge_buffer,
            center_plot,
        } => LayoutConfig {
            edge_buffer_pt: *edge_buffer,
            center_plot: *center_plot,
            ..Default::default()
        },
        _ => LayoutConfig::default(),
    };
    let calculator = LayoutCalculator::new(layout_config);
    let mut measurement_renderer = crate::render::SkiaRenderer::new(
        plot.display.dimensions.0,
        plot.display.dimensions.1,
        plot.display.theme.clone(),
    )
    .expect("measurement renderer");
    measurement_renderer.set_render_scale(plot.render_scale());
    measurement_renderer.set_text_engine_mode(plot.display.text_engine);
    let measured_dimensions = plot
        .measure_layout_text(
            &measurement_renderer,
            &content,
            plot.display.config.figure.dpi,
        )
        .expect("layout text measurements");
    let layout = calculator.compute(
        plot.display.dimensions,
        &content,
        &plot.display.config.typography,
        &plot.display.config.spacing,
        plot.display.config.figure.dpi,
        measured_dimensions.as_ref(),
    );
    let plot_area = tiny_skia::Rect::from_ltrb(
        layout.plot_area.left,
        layout.plot_area.top,
        layout.plot_area.right,
        layout.plot_area.bottom,
    )
    .expect("valid plot area");

    let x_tick_count = ((plot_area.width() / 100.0) as usize).clamp(2, 10);
    let y_tick_count = ((plot_area.height() / 60.0) as usize).clamp(2, 8);
    let x_ticks = generate_ticks(x_min, x_max, x_tick_count);
    let y_ticks = generate_ticks(y_min, y_max, y_tick_count);
    let x_tick_pixels: Vec<f32> = x_ticks
        .iter()
        .map(|&tick| {
            crate::render::skia::map_data_to_pixels(
                tick, 0.0, x_min, x_max, y_min, y_max, plot_area,
            )
            .0
        })
        .collect();
    let y_tick_pixels: Vec<f32> = y_ticks
        .iter()
        .map(|&tick| {
            crate::render::skia::map_data_to_pixels(
                0.0, tick, x_min, x_max, y_min, y_max, plot_area,
            )
            .1
        })
        .collect();

    let x_probe = x_tick_pixels[x_tick_pixels.len() / 2].round() as u32;
    let y_probe = y_tick_pixels[y_tick_pixels.len() / 2].round() as u32;
    let top_probe = (x_probe, (plot_area.top() + 2.0).round() as u32);
    let right_probe = ((plot_area.right() - 2.0).round() as u32, y_probe);
    (top_probe, right_probe)
}

fn parse_svg_attr_pt(line: &str, attr: &str) -> f32 {
    let marker = format!(r#"{}=""#, attr);
    let start = line
        .find(&marker)
        .unwrap_or_else(|| panic!("missing {} in line: {}", attr, line))
        + marker.len();
    let end = line[start..]
        .find('"')
        .unwrap_or_else(|| panic!("unterminated {} in line: {}", attr, line))
        + start;
    let value = line[start..end].trim_end_matches("pt");
    value
        .parse::<f32>()
        .unwrap_or_else(|_| panic!("invalid {} value in line: {}", attr, line))
}

fn extract_typst_group_boxes(svg: &str) -> Vec<(f32, f32, f32, f32)> {
    svg.lines()
        .filter(|line| line.contains(r#"data-ruviz-text-engine="typst""#))
        .map(|line| {
            let transform_marker = r#"transform="translate("#;
            let start = line
                .find(transform_marker)
                .unwrap_or_else(|| panic!("missing translate transform in line: {}", line))
                + transform_marker.len();
            let end = line[start..]
                .find(')')
                .unwrap_or_else(|| panic!("unterminated translate transform in line: {}", line))
                + start;
            let coords = &line[start..end];
            let mut parts = coords.split(',');
            let tx = parts
                .next()
                .unwrap_or_else(|| panic!("missing translate x in line: {}", line))
                .parse::<f32>()
                .unwrap_or_else(|_| panic!("invalid translate x in line: {}", line));
            let ty = parts
                .next()
                .unwrap_or_else(|| panic!("missing translate y in line: {}", line))
                .parse::<f32>()
                .unwrap_or_else(|_| panic!("invalid translate y in line: {}", line));
            let width = parse_svg_attr_pt(line, "width");
            let height = parse_svg_attr_pt(line, "height");
            (tx, ty, width, height)
        })
        .collect()
}

#[test]
fn test_get_theme_method() {
    use crate::render::Theme;

    let plot = Plot::new();
    let theme = plot.get_theme();

    // Should return a valid theme (can't compare directly since Theme doesn't implement PartialEq)
    // Just ensure the method works without panicking

    // Test with custom theme - just ensure get/set works
    let custom_theme = Theme::dark();
    let plot = Plot::new().theme(custom_theme);
    let _retrieved_theme = plot.get_theme();
    // Test passes if no panic occurs
}

#[test]
fn test_pending_ingestion_error_preserves_single_error_shape() {
    let bad = FailingIngestionData;
    let y = vec![1.0, 2.0, 3.0];

    let err = Plot::new().line(&bad, &y).render().unwrap_err();
    match err {
        PlottingError::DataExtractionFailed { source, message } => {
            assert_eq!(source, "test::failing-ingestion");
            assert_eq!(message, "forced ingestion failure");
        }
        other => panic!("expected DataExtractionFailed, got {other:?}"),
    }
}

#[test]
fn test_snapshot_validation_isolated_from_later_reactive_mutation() {
    let x = crate::data::Observable::new(vec![0.0, 1.0]);
    let plot = Plot::new().add_line_series(
        PlotData::Reactive(x.clone()),
        PlotData::Static(vec![1.0, 2.0]),
        &crate::plots::basic::LineConfig::default(),
        crate::core::plot::builder::SeriesStyle::default(),
    );

    let snapshot_series = plot.snapshot_series(0.0);
    x.set(vec![0.0, f64::NAN]);

    assert!(matches!(
        plot.validate_runtime_inputs(),
        Err(PlottingError::InvalidData { .. })
    ));
    plot.validate_runtime_inputs_for_series(&snapshot_series)
        .expect("snapshot validation should ignore later live mutations");
}

#[test]
fn test_snapshot_bounds_cover_heatmap_and_pie_series() {
    let heatmap = Plot::new()
        .heatmap(&vec![vec![1.0, 2.0], vec![3.0, 4.0]], None)
        .end_series();
    let heatmap_bounds = heatmap
        .calculate_data_bounds_for_series(&heatmap.snapshot_series(0.0))
        .expect("heatmap bounds should resolve");
    assert!(heatmap_bounds.0.is_finite());
    assert!(heatmap_bounds.1.is_finite());
    assert!(heatmap_bounds.2.is_finite());
    assert!(heatmap_bounds.3.is_finite());

    let pie = Plot::new().pie(&[2.0, 3.0, 5.0]).end_series();
    let pie_bounds = pie
        .calculate_data_bounds_for_series(&pie.snapshot_series(0.0))
        .expect("pie bounds should resolve");
    assert_eq!(pie_bounds, (0.0, 1.0, 0.0, 1.0));
}

#[test]
fn test_auto_datashader_policy_excludes_large_line_series() {
    let x: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|x| x.sin()).collect();
    let plot = Plot::new().line(&x, &y).end_series();
    let snapshot_series = plot.snapshot_series(0.0);
    let total_points = Plot::calculate_total_points_for_series(&snapshot_series);

    assert!(DataShader::should_activate(total_points));
    assert!(!Plot::should_auto_use_datashader(
        &snapshot_series,
        total_points
    ));
}

#[test]
fn test_auto_datashader_policy_keeps_large_scatter_series_eligible() {
    let x: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|x| x.sin()).collect();
    let plot = Plot::new().scatter(&x, &y).end_series();
    let snapshot_series = plot.snapshot_series(0.0);
    let total_points = Plot::calculate_total_points_for_series(&snapshot_series);

    assert!(DataShader::should_activate(total_points));
    assert!(Plot::should_auto_use_datashader(
        &snapshot_series,
        total_points
    ));
}

#[test]
fn test_prepared_frame_large_line_stays_off_auto_datashader() {
    let x: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|x| x.sin()).collect();
    let prepared = Plot::new()
        .line(&x, &y)
        .end_series()
        .prepared_frame_plot((1280, 720), 1.0, 0.0);
    let snapshot_series = prepared.snapshot_series(0.0);
    let total_points = Plot::calculate_total_points_for_series(&snapshot_series);

    assert!(DataShader::should_activate(total_points));
    assert!(!Plot::should_auto_use_datashader(
        &snapshot_series,
        total_points
    ));
}

#[test]
fn test_render_datashader_path_still_validates_mismatched_scatter_series() {
    let x: Vec<f64> = (0..100_001).map(|i| i as f64).collect();
    let y: Vec<f64> = (0..100_000).map(|i| i as f64).collect();

    let err = Plot::new()
        .scatter(&x, &y)
        .render()
        .expect_err("datashader path should reject mismatched inputs");

    assert!(matches!(err, PlottingError::DataLengthMismatch { .. }));
}

#[test]
fn test_render_with_datashader_uses_captured_snapshot_for_reactive_series() {
    let x = crate::data::Observable::new((0..100_001).map(|i| i as f64).collect::<Vec<_>>());
    let y = crate::data::Observable::new((0..100_001).map(|i| i as f64).collect::<Vec<_>>());
    let plot = Plot::new().add_line_series(
        PlotData::Reactive(x.clone()),
        PlotData::Reactive(y.clone()),
        &crate::plots::basic::LineConfig::default(),
        crate::core::plot::builder::SeriesStyle::default(),
    );

    let snapshot_series = plot.snapshot_series(0.0);
    x.set(vec![0.0, f64::NAN]);
    y.set(vec![1.0]);

    assert!(plot.validate_runtime_inputs().is_err());

    let image = plot
        .render_with_datashader(&snapshot_series)
        .expect("datashader helper should render the captured snapshot");
    assert!(image.width > 0);
    assert!(image.height > 0);
}

#[cfg(feature = "parallel")]
#[test]
fn test_render_parallel_path_still_validates_empty_series() {
    let empty: Vec<f64> = Vec::new();

    let err = Plot::new()
        .line(&empty, &empty)
        .end_series()
        .line(&[0.0, 1.0], &[1.0, 2.0])
        .render()
        .expect_err("parallel path should reject empty inputs");

    assert!(matches!(err, PlottingError::EmptyDataSet));
}

#[test]
fn test_pending_ingestion_error_reports_additional_failures() {
    let bad = FailingIngestionData;

    let err = Plot::new().line(&bad, &bad).render().unwrap_err();
    match err {
        PlottingError::DataExtractionFailed { source, message } => {
            assert_eq!(source, "ruviz::plot-ingestion");
            assert!(message.contains("forced ingestion failure"));
            assert!(message.contains("1 additional ingestion error"));
        }
        other => panic!("expected DataExtractionFailed, got {other:?}"),
    }
}

#[test]
fn test_group_scopes_shared_style_and_does_not_leak() {
    let x = vec![0.0, 1.0, 2.0];
    let y1 = vec![0.0, 1.0, 2.0];
    let y2 = vec![0.0, 2.0, 4.0];
    let y3 = vec![0.0, 3.0, 6.0];

    let plot = Plot::new()
        .group(|g| {
            g.color(Color::RED)
                .line_width(3.0)
                .line_style(LineStyle::Dashed)
                .alpha(0.35)
                .line(&x, &y1)
                .line(&x, &y2)
        })
        .line(&x, &y3)
        .end_series();

    assert_eq!(plot.series_mgr.series.len(), 3);
    assert_eq!(plot.series_mgr.series[0].color, Some(Color::RED));
    assert_eq!(plot.series_mgr.series[1].color, Some(Color::RED));
    assert_eq!(plot.series_mgr.series[0].line_width, Some(3.0));
    assert_eq!(plot.series_mgr.series[1].line_width, Some(3.0));
    assert!(matches!(
        plot.series_mgr.series[0].line_style,
        Some(LineStyle::Dashed)
    ));
    assert!(matches!(
        plot.series_mgr.series[1].line_style,
        Some(LineStyle::Dashed)
    ));
    assert_eq!(plot.series_mgr.series[0].alpha, Some(0.35));
    assert_eq!(plot.series_mgr.series[1].alpha, Some(0.35));

    // Outside the group, styles should fall back to normal per-series defaults.
    assert_ne!(plot.series_mgr.series[2].color, Some(Color::RED));
    assert_ne!(plot.series_mgr.series[2].line_width, Some(3.0));
    assert!(matches!(
        plot.series_mgr.series[2].line_style,
        Some(LineStyle::Solid)
    ));
    assert_eq!(plot.series_mgr.series[2].alpha, Some(1.0));
}

#[test]
fn test_group_label_collapses_legend_to_single_item() {
    let x = vec![0.0, 1.0, 2.0];
    let y1 = vec![0.0, 1.0, 2.0];
    let y2 = vec![0.0, 2.0, 4.0];
    let y3 = vec![0.0, 3.0, 6.0];

    let plot = Plot::new()
        .group(|g| {
            g.group_label("Grouped")
                .line_style(LineStyle::Dashed)
                .line(&x, &y1)
                .line(&x, &y2)
        })
        .line(&x, &y3)
        .label("Solo")
        .end_series();

    let legend_items = plot.collect_legend_items();
    assert_eq!(legend_items.len(), 2);
    assert!(legend_items.iter().any(|item| item.label == "Grouped"));
    assert!(legend_items.iter().any(|item| item.label == "Solo"));

    let grouped = legend_items
        .iter()
        .find(|item| item.label == "Grouped")
        .expect("group legend item should exist");
    assert!(matches!(grouped.item_type, LegendItemType::Line { .. }));
}

#[test]
fn test_group_without_label_is_omitted_from_legend() {
    let x = vec![0.0, 1.0, 2.0];
    let y1 = vec![0.0, 1.0, 2.0];
    let y2 = vec![0.0, 2.0, 4.0];

    let plot = Plot::new().group(|g| g.line(&x, &y1).line(&x, &y2));
    let legend_items = plot.collect_legend_items();
    assert!(legend_items.is_empty());
}

#[test]
fn test_group_without_color_uses_single_palette_color() {
    let x = vec![0.0, 1.0, 2.0];
    let y1 = vec![0.0, 1.0, 2.0];
    let y2 = vec![0.0, 2.0, 4.0];
    let y3 = vec![0.0, 3.0, 6.0];

    let plot = Plot::new()
        .group(|g| g.line(&x, &y1).line(&x, &y2))
        .line(&x, &y3)
        .end_series();
    assert_eq!(plot.series_mgr.series.len(), 3);
    assert_eq!(
        plot.series_mgr.series[0].color,
        plot.series_mgr.series[1].color
    );
    assert_ne!(
        plot.series_mgr.series[0].color,
        plot.series_mgr.series[2].color
    );
}

#[test]
fn test_group_mixed_series_uses_first_member_legend_glyph() {
    let x = vec![0.0, 1.0, 2.0];
    let y1 = vec![0.0, 1.0, 2.0];
    let y2 = vec![0.0, 2.0, 4.0];

    let plot = Plot::new().group(|g| {
        g.group_label("Mixed")
            .scatter(&x, &y1)
            .line(&x, &y2)
            .line_style(LineStyle::Dashed)
    });

    let legend_items = plot.collect_legend_items();
    assert_eq!(legend_items.len(), 1);
    assert_eq!(legend_items[0].label, "Mixed");
    assert!(matches!(
        legend_items[0].item_type,
        LegendItemType::Scatter { .. }
    ));
}

#[test]
fn test_render_to_renderer_basic() {
    use crate::render::{SkiaRenderer, Theme};

    let x_data = vec![1.0, 2.0, 3.0];
    let y_data = vec![2.0, 4.0, 3.0];

    let plot = Plot::new()
        .line(&x_data, &y_data)
        .title("Test Plot")
        .xlabel("X")
        .ylabel("Y")
        .end_series();

    let mut renderer = SkiaRenderer::new(400, 300, Theme::default()).unwrap();

    // Should render without error
    let result = plot.render_to_renderer(&mut renderer, 96.0);
    assert!(result.is_ok());
}

#[test]
fn test_render_to_renderer_empty_series() {
    use crate::render::{SkiaRenderer, Theme};

    let plot = Plot::new().title("Empty Plot");
    let mut renderer = SkiaRenderer::new(400, 300, Theme::default()).unwrap();

    // Should fail with no data series
    let result = plot.render_to_renderer(&mut renderer, 96.0);
    assert!(result.is_err());
}

#[test]
fn test_render_to_renderer_multiple_series() {
    use crate::render::{SkiaRenderer, Theme};

    let x1 = vec![1.0, 2.0, 3.0];
    let y1 = vec![2.0, 4.0, 3.0];
    let x2 = vec![1.5, 2.5, 3.5];
    let y2 = vec![1.0, 3.0, 2.0];

    let plot = Plot::new()
        .line(&x1, &y1)
        .label("Series 1")
        .line(&x2, &y2)
        .label("Series 2")
        .title("Multi-series Plot")
        .end_series();

    let mut renderer = SkiaRenderer::new(400, 300, Theme::default()).unwrap();

    // Should render multiple series without error
    let result = plot.render_to_renderer(&mut renderer, 96.0);
    assert!(result.is_ok());
}

#[test]
fn test_render_to_renderer_dpi_scaling() {
    use crate::render::{SkiaRenderer, Theme};

    let x_data = vec![1.0, 2.0, 3.0];
    let y_data = vec![2.0, 4.0, 3.0];

    let plot = Plot::new()
        .line(&x_data, &y_data)
        .title("DPI Test")
        .end_series();

    let mut renderer = SkiaRenderer::new(400, 300, Theme::default()).unwrap();

    // Test different DPI values
    let result_96 = plot.clone().render_to_renderer(&mut renderer, 96.0);
    assert!(result_96.is_ok());

    let result_144 = plot.clone().render_to_renderer(&mut renderer, 144.0);
    assert!(result_144.is_ok());

    let result_300 = plot.render_to_renderer(&mut renderer, 300.0);
    assert!(result_300.is_ok());
}

#[test]
fn test_render_honors_top_and_right_tick_sides() {
    let base_plot = Plot::new()
        .size_px(400, 300)
        .line(&[0.0, 10.0, 20.0], &[0.0, 50.0, 100.0])
        .end_series();
    let all_sides = base_plot.clone().ticks_all_sides();
    let bottom_left = base_plot.ticks_bottom_left();

    let (top_probe, right_probe) = compute_render_tick_probe_points(&all_sides);
    let image_all_sides = all_sides.render().expect("all-sides render should succeed");
    let image_bottom_left = bottom_left
        .render()
        .expect("bottom-left render should succeed");

    assert!(image_pixel_is_dark(
        &image_all_sides,
        top_probe.0,
        top_probe.1
    ));
    assert!(!image_pixel_is_dark(
        &image_bottom_left,
        top_probe.0,
        top_probe.1
    ));
    assert!(image_pixel_is_dark(
        &image_all_sides,
        right_probe.0,
        right_probe.1
    ));
    assert!(!image_pixel_is_dark(
        &image_bottom_left,
        right_probe.0,
        right_probe.1
    ));
}

fn compute_categorical_render_top_tick_probe(plot: &Plot) -> (u32, u32) {
    let (x_min, x_max, y_min, y_max) = plot
        .calculate_data_bounds()
        .expect("data bounds should be available");
    let content = plot.create_plot_content(y_min, y_max);
    let layout_config = match &plot.display.config.margins {
        MarginConfig::ContentDriven {
            edge_buffer,
            center_plot,
        } => LayoutConfig {
            edge_buffer_pt: *edge_buffer,
            center_plot: *center_plot,
            ..Default::default()
        },
        _ => LayoutConfig::default(),
    };
    let calculator = LayoutCalculator::new(layout_config);
    let mut measurement_renderer = crate::render::SkiaRenderer::new(
        plot.display.dimensions.0,
        plot.display.dimensions.1,
        plot.display.theme.clone(),
    )
    .expect("measurement renderer");
    measurement_renderer.set_render_scale(plot.render_scale());
    measurement_renderer.set_text_engine_mode(plot.display.text_engine);
    let measured_dimensions = plot
        .measure_layout_text(
            &measurement_renderer,
            &content,
            plot.display.config.figure.dpi,
        )
        .expect("layout text measurements");
    let layout = calculator.compute(
        plot.display.dimensions,
        &content,
        &plot.display.config.typography,
        &plot.display.config.spacing,
        plot.display.config.figure.dpi,
        measured_dimensions.as_ref(),
    );
    let plot_area = tiny_skia::Rect::from_ltrb(
        layout.plot_area.left,
        layout.plot_area.top,
        layout.plot_area.right,
        layout.plot_area.bottom,
    )
    .expect("valid plot area");
    let categories = plot
        .series_mgr
        .series
        .iter()
        .find_map(|series| {
            if let SeriesType::Bar { categories, .. } = &series.series_type {
                Some(categories.len())
            } else {
                None
            }
        })
        .expect("categorical plot should contain bar categories");
    let x_tick_pixels =
        Plot::categorical_x_tick_pixels(plot_area, x_min, x_max, Some(categories), &[])
            .expect("categorical ticks should be available");
    let x_probe = x_tick_pixels[0].round() as u32;
    (x_probe, (plot_area.top() + 3.0).round() as u32)
}

#[test]
fn test_render_honors_top_ticks_for_categorical_bar() {
    let categories = ["A", "B", "C"];
    let values = [2.0, 4.0, 3.0];
    let base_plot = Plot::new()
        .size_px(400, 300)
        .bar(&categories, &values)
        .end_series();
    let all_sides = base_plot.clone().ticks_all_sides();
    let bottom_left = base_plot.ticks_bottom_left();

    let top_probe = compute_categorical_render_top_tick_probe(&all_sides);
    let image_all_sides = all_sides.render().expect("all-sides render should succeed");
    let image_bottom_left = bottom_left
        .render()
        .expect("bottom-left render should succeed");

    assert!(image_has_dark_pixel_near(
        &image_all_sides,
        top_probe.0,
        top_probe.1,
        1
    ));
    assert!(!image_has_dark_pixel_near(
        &image_bottom_left,
        top_probe.0,
        top_probe.1,
        1
    ));
}

#[test]
fn test_render_to_renderer_honors_top_ticks_for_categorical_bar() {
    let categories = ["A", "B", "C"];
    let values = [2.0, 4.0, 3.0];
    let base_plot = Plot::new()
        .size_px(400, 300)
        .bar(&categories, &values)
        .end_series();
    let all_sides = base_plot.clone().ticks_all_sides();
    let bottom_left = base_plot.ticks_bottom_left();
    let top_probe = compute_categorical_render_top_tick_probe(&all_sides);

    let mut renderer_all =
        crate::render::SkiaRenderer::new(400, 300, all_sides.display.theme.clone())
            .expect("renderer");
    all_sides
        .render_to_renderer(&mut renderer_all, 100.0)
        .expect("all-sides render_to_renderer should succeed");
    let image_all_sides = renderer_all.into_image();

    let mut renderer_bottom_left =
        crate::render::SkiaRenderer::new(400, 300, bottom_left.display.theme.clone())
            .expect("renderer");
    bottom_left
        .render_to_renderer(&mut renderer_bottom_left, 100.0)
        .expect("bottom-left render_to_renderer should succeed");
    let image_bottom_left = renderer_bottom_left.into_image();

    assert!(image_has_dark_pixel_near(
        &image_all_sides,
        top_probe.0,
        top_probe.1,
        1
    ));
    assert!(!image_has_dark_pixel_near(
        &image_bottom_left,
        top_probe.0,
        top_probe.1,
        1
    ));
}

#[test]
fn test_render_to_svg_uses_layout_positions_for_title_and_labels() {
    use crate::render::{FontConfig, FontFamily, TextRenderer};

    let x_data = vec![0.0, 1.0, 2.0, 3.0];
    let y_data = vec![1.0, 3.0, 2.0, 4.0];
    let plot = Plot::new()
        .line(&x_data, &y_data)
        .title("SVG_LAYOUT_TITLE")
        .xlabel("SVG_LAYOUT_X")
        .ylabel("SVG_LAYOUT_Y")
        .end_series();

    let svg = plot.render_to_svg().expect("SVG render should succeed");

    let (_x_min, _x_max, y_min, y_max) = plot
        .calculate_data_bounds()
        .expect("data bounds should be available");
    let content = plot.create_plot_content(y_min, y_max);
    let layout_config = match &plot.display.config.margins {
        MarginConfig::ContentDriven {
            edge_buffer,
            center_plot,
        } => LayoutConfig {
            edge_buffer_pt: *edge_buffer,
            center_plot: *center_plot,
            ..Default::default()
        },
        _ => LayoutConfig::default(),
    };
    let calculator = LayoutCalculator::new(layout_config);
    let mut measurement_renderer = crate::render::SkiaRenderer::new(
        plot.display.dimensions.0,
        plot.display.dimensions.1,
        plot.display.theme.clone(),
    )
    .expect("measurement renderer");
    let render_scale = plot.render_scale();
    measurement_renderer.set_render_scale(render_scale);
    measurement_renderer.set_text_engine_mode(plot.display.text_engine);
    let measured_dimensions = plot
        .measure_layout_text(
            &measurement_renderer,
            &content,
            plot.display.config.figure.dpi,
        )
        .expect("layout text measurements");
    let layout = calculator.compute(
        plot.display.dimensions,
        &content,
        &plot.display.config.typography,
        &plot.display.config.spacing,
        plot.display.config.figure.dpi,
        measured_dimensions.as_ref(),
    );

    let title_pos = layout.title_pos.expect("title position");
    let xlabel_pos = layout.xlabel_pos.expect("xlabel position");
    let ylabel_pos = layout.ylabel_pos.expect("ylabel position");
    let text_renderer = TextRenderer::new();
    let title_metrics = text_renderer
        .measure_text_placement(
            "SVG_LAYOUT_TITLE",
            &FontConfig::new(FontFamily::SansSerif, title_pos.size),
        )
        .expect("title metrics");
    let xlabel_metrics = text_renderer
        .measure_text_placement(
            "SVG_LAYOUT_X",
            &FontConfig::new(FontFamily::SansSerif, xlabel_pos.size),
        )
        .expect("xlabel metrics");

    let (title_x, title_y) = extract_svg_text_xy(&svg, "SVG_LAYOUT_TITLE");
    let (xlabel_x, xlabel_y) = extract_svg_text_xy(&svg, "SVG_LAYOUT_X");
    let (ylabel_x, ylabel_y) = extract_svg_group_translate_xy(&svg, "SVG_LAYOUT_Y");

    assert!(
        (title_x - title_pos.x).abs() <= 0.6
            && (title_y - (title_pos.y + title_metrics.baseline_from_top)).abs() <= 0.6,
        "title should follow layout position: svg=({}, {}), layout=({}, {})",
        title_x,
        title_y,
        title_pos.x,
        title_pos.y + title_metrics.baseline_from_top
    );
    assert!(
        (xlabel_x - xlabel_pos.x).abs() <= 0.6
            && (xlabel_y - (xlabel_pos.y + xlabel_metrics.baseline_from_top)).abs() <= 0.6,
        "xlabel should follow layout position: svg=({}, {}), layout=({}, {})",
        xlabel_x,
        xlabel_y,
        xlabel_pos.x,
        xlabel_pos.y + xlabel_metrics.baseline_from_top
    );
    assert!(
        (ylabel_x - ylabel_pos.x).abs() <= 0.6 && (ylabel_y - ylabel_pos.y).abs() <= 0.6,
        "ylabel should follow layout position: svg=({}, {}), layout=({}, {})",
        ylabel_x,
        ylabel_y,
        ylabel_pos.x,
        ylabel_pos.y
    );
}

#[test]
fn test_render_to_svg_preserves_line_marker_shape() {
    let marker_color = Color::new(17, 119, 51);
    let plot = Plot::new()
        .line(&[0.0, 1.0], &[0.0, 1.0])
        .color(marker_color)
        .marker(MarkerStyle::Square)
        .marker_size(10.0)
        .ticks(false)
        .grid(false)
        .end_series();

    let svg = plot.render_to_svg().expect("SVG render should succeed");
    let marker_fill = r#"fill="rgb(17,119,51)""#;

    assert!(
        svg.lines()
            .any(|line| line.contains("<rect") && line.contains(marker_fill)),
        "square line markers should render as filled rects in SVG"
    );
    assert!(
        !svg.lines()
            .any(|line| line.contains("<circle") && line.contains(marker_fill)),
        "square line markers should not fall back to circles in SVG"
    );
}

#[test]
fn test_render_to_svg_ticks_false_omits_tick_artifacts_but_keeps_axis_labels() {
    let x_data = vec![0.0, 1.0, 2.0, 3.0];
    let y_data = vec![1.0, 3.0, 2.0, 4.0];

    let svg_with_ticks = Plot::new()
        .line(&x_data, &y_data)
        .title("NO_TICK_TITLE")
        .xlabel("NO_TICK_X")
        .ylabel("NO_TICK_Y")
        .render_to_svg()
        .expect("SVG render should succeed");

    let svg_without_ticks = Plot::new()
        .line(&x_data, &y_data)
        .ticks(false)
        .title("NO_TICK_TITLE")
        .xlabel("NO_TICK_X")
        .ylabel("NO_TICK_Y")
        .render_to_svg()
        .expect("SVG render should succeed");

    assert!(
        svg_without_ticks.matches("<line ").count() < svg_with_ticks.matches("<line ").count(),
        "ticks(false) should reduce axis/tick line segments in SVG output"
    );
    assert_eq!(
        svg_without_ticks.matches("</text>").count(),
        3,
        "ticks(false) should keep only title/xlabel/ylabel text nodes"
    );
    assert!(svg_without_ticks.contains(">NO_TICK_TITLE</text>"));
    assert!(svg_without_ticks.contains(">NO_TICK_X</text>"));
    assert!(svg_without_ticks.contains(">NO_TICK_Y</text>"));
}

#[test]
fn test_render_to_svg_ticks_false_keeps_dpi_scaled_frame_stroke() {
    let svg = Plot::new()
        .dpi(200)
        .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0])
        .ticks(false)
        .render_to_svg()
        .expect("SVG render should succeed");

    assert!(
        svg.contains(r#"stroke-width="3.00""#),
        "ticks(false) should keep the DPI-scaled 1.5 logical px frame width"
    );
}

#[cfg(feature = "typst-math")]
#[test]
fn test_render_to_svg_typst_uses_layout_anchor_contract() {
    let x_data = vec![0.0, 1.0, 2.0, 3.0];
    let y_data = vec![1.0, 3.0, 2.0, 4.0];
    let plot = Plot::new()
        .line(&x_data, &y_data)
        .title("SVG_LAYOUT_TITLE")
        .xlabel("SVG_LAYOUT_X")
        .ylabel("SVG_LAYOUT_Y")
        .typst(true)
        .end_series();

    let svg = plot.render_to_svg().expect("SVG render should succeed");

    let (_x_min, _x_max, y_min, y_max) = plot
        .calculate_data_bounds()
        .expect("data bounds should be available");
    let content = plot.create_plot_content(y_min, y_max);
    let layout_config = match &plot.display.config.margins {
        MarginConfig::ContentDriven {
            edge_buffer,
            center_plot,
        } => LayoutConfig {
            edge_buffer_pt: *edge_buffer,
            center_plot: *center_plot,
            ..Default::default()
        },
        _ => LayoutConfig::default(),
    };
    let calculator = LayoutCalculator::new(layout_config);
    let mut measurement_renderer = crate::render::SkiaRenderer::new(
        plot.display.dimensions.0,
        plot.display.dimensions.1,
        plot.display.theme.clone(),
    )
    .expect("measurement renderer");
    let render_scale = plot.render_scale();
    measurement_renderer.set_render_scale(render_scale);
    measurement_renderer.set_text_engine_mode(plot.display.text_engine);
    let measured_dimensions = plot
        .measure_layout_text(
            &measurement_renderer,
            &content,
            plot.display.config.figure.dpi,
        )
        .expect("layout text measurements");
    let layout = calculator.compute(
        plot.display.dimensions,
        &content,
        &plot.display.config.typography,
        &plot.display.config.spacing,
        plot.display.config.figure.dpi,
        measured_dimensions.as_ref(),
    );

    let title_pos = layout.title_pos.expect("title position");
    let xlabel_pos = layout.xlabel_pos.expect("xlabel position");
    let ylabel_pos = layout.ylabel_pos.expect("ylabel position");

    let typst_groups = extract_typst_group_boxes(&svg);
    assert!(
        typst_groups.len() >= 3,
        "expected at least three typst text groups, found {}",
        typst_groups.len()
    );
    let n = typst_groups.len();
    let title = typst_groups[n - 3];
    let xlabel = typst_groups[n - 2];
    let ylabel = typst_groups[n - 1];

    let title_center_x = title.0 + title.2 / 2.0;
    let xlabel_center_x = xlabel.0 + xlabel.2 / 2.0;
    let ylabel_center_x = ylabel.0 + ylabel.2 / 2.0;
    let ylabel_center_y = ylabel.1 + ylabel.3 / 2.0;

    assert!(
        (title_center_x - title_pos.x).abs() <= 0.8 && (title.1 - title_pos.y).abs() <= 0.8,
        "typst title should follow top-center anchor: group=({}, {}, {}x{}), layout=({}, {})",
        title.0,
        title.1,
        title.2,
        title.3,
        title_pos.x,
        title_pos.y
    );
    assert!(
        (xlabel_center_x - xlabel_pos.x).abs() <= 0.8 && (xlabel.1 - xlabel_pos.y).abs() <= 0.8,
        "typst xlabel should follow top-center anchor: group=({}, {}, {}x{}), layout=({}, {})",
        xlabel.0,
        xlabel.1,
        xlabel.2,
        xlabel.3,
        xlabel_pos.x,
        xlabel_pos.y
    );
    assert!(
        (ylabel_center_x - ylabel_pos.x).abs() <= 0.8
            && (ylabel_center_y - ylabel_pos.y).abs() <= 0.8,
        "typst ylabel should follow center anchor: group=({}, {}, {}x{}), layout=({}, {})",
        ylabel.0,
        ylabel.1,
        ylabel.2,
        ylabel.3,
        ylabel_pos.x,
        ylabel_pos.y
    );
}

#[test]
fn test_render_to_svg_preserves_line_width_ratio_across_dpi() {
    let x_data = vec![0.0, 1.0, 2.0, 3.0];
    let y_data = vec![1.0, 3.0, 2.0, 4.0];

    let plot_100 = Plot::new()
        .size(6.4, 4.8)
        .dpi(100)
        .line(&x_data, &y_data)
        .line_width(2.0)
        .end_series();
    let plot_200 = Plot::new()
        .size(6.4, 4.8)
        .dpi(200)
        .line(&x_data, &y_data)
        .line_width(2.0)
        .end_series();

    let svg_100 = plot_100.render_to_svg().expect("100 DPI SVG render");
    let svg_200 = plot_200.render_to_svg().expect("200 DPI SVG render");

    let width_100 = extract_svg_root_attr(&svg_100, "width");
    let width_200 = extract_svg_root_attr(&svg_200, "width");
    let stroke_100 = extract_first_svg_polyline_stroke_width(&svg_100);
    let stroke_200 = extract_first_svg_polyline_stroke_width(&svg_200);

    let ratio_100 = stroke_100 / width_100;
    let ratio_200 = stroke_200 / width_200;

    assert!(
        (ratio_100 - ratio_200).abs() < 0.0005,
        "stroke-to-canvas ratio should remain stable across DPI: {} vs {}",
        ratio_100,
        ratio_200
    );
}

// ========== GPU Integration Tests ==========

#[test]
#[cfg(feature = "gpu")]
fn test_gpu_method_sets_backend() {
    let plot = Plot::new().gpu(true);
    assert_eq!(plot.get_backend_name(), "gpu");
    assert!(plot.render.enable_gpu);
}

#[test]
#[cfg(feature = "gpu")]
fn test_gpu_method_disabled() {
    let plot = Plot::new().gpu(false);
    // When disabled, backend should not be set to GPU
    assert!(!plot.render.enable_gpu);
}

#[test]
#[cfg(feature = "gpu")]
fn test_gpu_threshold_constants() {
    // Verify threshold constants are reasonable
    const DATASHADER_THRESHOLD: usize = 100_000;
    const GPU_THRESHOLD: usize = 5_000;

    let datashader_threshold = std::hint::black_box(DATASHADER_THRESHOLD);
    let gpu_threshold = std::hint::black_box(GPU_THRESHOLD);

    assert!(gpu_threshold < datashader_threshold);
    assert!(gpu_threshold > 0);
}

#[test]
#[cfg(feature = "gpu")]
fn test_gpu_with_small_dataset() {
    // Small datasets should not trigger GPU even with gpu(true)
    let x_data: Vec<f64> = (0..100).map(|i| i as f64).collect();
    let y_data: Vec<f64> = x_data.iter().map(|x| x * x).collect();

    let plot = Plot::new()
        .gpu(true)
        .line(&x_data, &y_data)
        .title("Small Dataset GPU Test")
        .end_series();

    // Should succeed (will use CPU path due to small dataset)
    let result = plot.render();
    assert!(result.is_ok());
}

#[test]
#[cfg(feature = "gpu")]
fn test_gpu_with_medium_dataset() {
    // Medium datasets (>5K) should trigger GPU path
    let x_data: Vec<f64> = (0..6000).map(|i| i as f64 * 0.01).collect();
    let y_data: Vec<f64> = x_data.iter().map(|x| x.sin()).collect();

    let plot = Plot::new()
        .gpu(true)
        .line(&x_data, &y_data)
        .title("Medium Dataset GPU Test")
        .end_series();

    // Should succeed (GPU path if available, otherwise fallback to CPU)
    let result = plot.render();
    assert!(result.is_ok());
}

#[test]
#[cfg(feature = "gpu")]
fn test_gpu_scatter_plot() {
    // Test GPU with scatter plot
    let x_data: Vec<f64> = (0..5500).map(|i| i as f64 * 0.01).collect();
    let y_data: Vec<f64> = x_data.iter().map(|x| x.cos()).collect();

    let plot = Plot::new()
        .gpu(true)
        .scatter(&x_data, &y_data)
        .title("Scatter GPU Test")
        .end_series();

    let result = plot.render();
    assert!(result.is_ok());
}

#[test]
#[cfg(feature = "gpu")]
fn test_gpu_fallback_on_unsupported_series() {
    // Bar charts should fall back to normal rendering even with GPU enabled
    let categories = vec!["A", "B", "C", "D"];
    let values = vec![10.0, 20.0, 15.0, 25.0];

    let plot = Plot::new()
        .gpu(true)
        .bar(&categories, &values)
        .title("Bar Chart GPU Fallback")
        .end_series();

    let result = plot.render();
    assert!(result.is_ok());
}

#[test]
#[cfg(feature = "gpu")]
fn test_plot_series_builder_gpu_method() {
    let x_data: Vec<f64> = (0..100).map(|i| i as f64).collect();
    let y_data: Vec<f64> = x_data.iter().map(|x| x * 2.0).collect();

    // Test that gpu() works on PlotSeriesBuilder
    let plot = Plot::new().line(&x_data, &y_data).gpu(true);

    assert_eq!(plot.get_backend_name(), "gpu");
}

#[test]
fn test_backend_selection_without_gpu_feature() {
    // Test that backend selection works when GPU feature is not enabled
    let plot = Plot::new().backend(BackendType::Parallel);
    assert_eq!(plot.get_backend_name(), "parallel");

    let plot2 = Plot::new().backend(BackendType::DataShader);
    assert_eq!(plot2.get_backend_name(), "datashader");
}

#[test]
fn test_auto_backend_selection() {
    // Test auto-optimization selects appropriate backend
    let x_small: Vec<f64> = (0..100).map(|i| i as f64).collect();
    let y_small: Vec<f64> = x_small.iter().map(|x| x * x).collect();

    let plot = Plot::new().line(&x_small, &y_small).end_series();

    // auto_optimize consumes self and returns Self
    let plot = plot.auto_optimize();

    // Small dataset should use Skia
    let backend_name = plot.get_backend_name();
    assert_eq!(backend_name, "skia");
}

// ========================================================================
// Streaming Data Tests
// ========================================================================

#[test]
fn test_line_streaming_basic() {
    use crate::data::StreamingXY;

    let stream = StreamingXY::new(100);
    stream.push_many(vec![(0.0, 0.0), (1.0, 1.0), (2.0, 4.0), (3.0, 9.0)]);

    let plot = Plot::new()
        .line_streaming(&stream)
        .title("Streaming Line Plot")
        .end_series();

    assert_eq!(plot.series_mgr.series.len(), 1);

    // Verify data was captured
    if let SeriesType::Line { x_data, y_data } = &plot.series_mgr.series[0].series_type {
        let x_resolved = x_data.resolve(0.0);
        let y_resolved = y_data.resolve(0.0);
        assert_eq!(x_resolved.len(), 4);
        assert_eq!(y_resolved.len(), 4);
        assert_eq!(x_resolved[0], 0.0);
        assert_eq!(y_resolved[3], 9.0);
    } else {
        panic!("Expected Line series type");
    }
}

#[test]
fn test_scatter_streaming_basic() {
    use crate::data::StreamingXY;

    let stream = StreamingXY::new(100);
    stream.push_many(vec![(1.0, 10.0), (2.0, 20.0), (3.0, 30.0)]);

    let plot = Plot::new()
        .scatter_streaming(&stream)
        .title("Streaming Scatter")
        .end_series();

    assert_eq!(plot.series_mgr.series.len(), 1);

    if let SeriesType::Scatter { x_data, y_data } = &plot.series_mgr.series[0].series_type {
        assert_eq!(x_data.len(), 3);
        assert_eq!(y_data.len(), 3);
    } else {
        panic!("Expected Scatter series type");
    }
}

#[test]
fn test_streaming_marks_rendered() {
    use crate::data::StreamingXY;

    let stream = StreamingXY::new(100);
    stream.push_many(vec![(0.0, 0.0), (1.0, 1.0)]);

    assert_eq!(stream.appended_count(), 2);

    let plot = Plot::new().line_streaming(&stream).end_series();

    // Construction keeps the stream live. Rendering advances the append mark.
    assert_eq!(stream.appended_count(), 2);
    plot.render().expect("streaming plot should render");
    assert_eq!(stream.appended_count(), 0);
}

#[test]
fn test_line_streaming_reads_updates_after_build() {
    use crate::data::StreamingXY;

    let stream = StreamingXY::new(100);
    stream.push_many(vec![(0.0, 0.0), (1.0, 1.0)]);

    let plot = Plot::new().line_streaming(&stream).end_series();
    stream.push(2.0, 4.0);

    if let SeriesType::Line { x_data, y_data } = &plot.series_mgr.series[0].series_type {
        assert_eq!(x_data.resolve(0.0), vec![0.0, 1.0, 2.0]);
        assert_eq!(y_data.resolve(0.0), vec![0.0, 1.0, 4.0]);
    } else {
        panic!("Expected Line series type");
    }
}

#[test]
fn test_streaming_render_output() {
    use crate::data::StreamingXY;

    let stream = StreamingXY::new(100);
    stream.push_many(vec![(0.0, 0.0), (1.0, 1.0), (2.0, 4.0)]);

    let plot = Plot::new()
        .line_streaming(&stream)
        .title("Streaming Test")
        .end_series();

    let result = plot.render();
    assert!(result.is_ok());
}

#[test]
fn test_streaming_with_ring_buffer_wrap() {
    use crate::data::StreamingXY;

    // Small buffer that wraps
    let stream = StreamingXY::new(3);
    stream.push_many(vec![
        (0.0, 0.0),
        (1.0, 1.0),
        (2.0, 2.0),
        (3.0, 3.0),
        (4.0, 4.0),
    ]);

    // Buffer should only contain last 3 points
    assert_eq!(stream.len(), 3);

    let plot = Plot::new().line_streaming(&stream).end_series();

    if let SeriesType::Line { x_data, y_data: _ } = &plot.series_mgr.series[0].series_type {
        let x_resolved = x_data.resolve(0.0);
        assert_eq!(x_resolved.len(), 3);
        // Should be the last 3 values
        assert_eq!(x_resolved[0], 2.0);
        assert_eq!(x_resolved[1], 3.0);
        assert_eq!(x_resolved[2], 4.0);
    } else {
        panic!("Expected Line series type");
    }
}

#[test]
fn test_streaming_empty_buffer() {
    use crate::data::StreamingXY;

    // Empty stream should still create a valid plot structure
    let stream = StreamingXY::new(100);

    let plot = Plot::new()
        .line_streaming(&stream)
        .title("Empty Stream")
        .end_series();

    assert_eq!(plot.series_mgr.series.len(), 1);

    if let SeriesType::Line { x_data, y_data } = &plot.series_mgr.series[0].series_type {
        assert!(x_data.is_empty());
        assert!(y_data.is_empty());
    }

    // Note: Empty data may fail to render (no bounds can be computed)
    // This is expected behavior - we test that the plot structure is correct
    // A real application would check for empty data before rendering
}

#[test]
fn test_streaming_multiple_series() {
    use crate::data::StreamingXY;

    let stream1 = StreamingXY::new(100);
    let stream2 = StreamingXY::new(100);

    stream1.push_many(vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]);
    stream2.push_many(vec![(0.0, 0.0), (1.0, 2.0), (2.0, 4.0)]);

    let plot = Plot::new()
        .line_streaming(&stream1)
        .label("Linear")
        .line_streaming(&stream2)
        .label("Quadratic")
        .title("Multiple Streaming Series")
        .end_series();

    assert_eq!(plot.series_mgr.series.len(), 2);

    // First series
    if let SeriesType::Line { x_data, .. } = &plot.series_mgr.series[0].series_type {
        assert_eq!(x_data.len(), 3);
    }

    // Second series
    if let SeriesType::Line { x_data, .. } = &plot.series_mgr.series[1].series_type {
        assert_eq!(x_data.len(), 3);
    }

    let result = plot.render();
    assert!(result.is_ok());
}

#[test]
fn test_streaming_mixed_with_static() {
    use crate::data::StreamingXY;

    let stream = StreamingXY::new(100);
    stream.push_many(vec![(0.0, 0.0), (1.0, 1.0), (2.0, 4.0)]);

    // Mix streaming and static data
    let static_x = vec![0.0, 1.0, 2.0];
    let static_y = vec![0.0, 2.0, 4.0];

    let plot = Plot::new()
        .line_streaming(&stream)
        .label("Streaming")
        .line(&static_x, &static_y)
        .label("Static")
        .title("Mixed Data Sources")
        .end_series();

    assert_eq!(plot.series_mgr.series.len(), 2);

    let result = plot.render();
    assert!(result.is_ok());
}

#[test]
fn test_streaming_with_styling() {
    use crate::data::StreamingXY;

    let stream = StreamingXY::new(100);
    stream.push_many(vec![(0.0, 0.0), (1.0, 1.0), (2.0, 4.0)]);

    let plot = Plot::new()
        .line_streaming(&stream)
        .color(Color::new(255, 0, 0))
        .width(3.0)
        .label("Styled Streaming")
        .title("Styled Streaming Plot")
        .xlabel("X Axis")
        .ylabel("Y Axis")
        .end_series();

    assert_eq!(plot.series_mgr.series[0].color, Some(Color::new(255, 0, 0)));
    assert_eq!(plot.series_mgr.series[0].line_width, Some(3.0));

    let result = plot.render();
    assert!(result.is_ok());
}

#[test]
fn test_streaming_scatter_with_styling() {
    use crate::data::StreamingXY;

    let stream = StreamingXY::new(100);
    stream.push_many(vec![(0.0, 0.0), (1.0, 1.0), (2.0, 4.0)]);

    let plot = Plot::new()
        .scatter_streaming(&stream)
        .color(Color::new(0, 255, 0))
        .marker_size(10.0)
        .end_series();

    assert_eq!(plot.series_mgr.series[0].color, Some(Color::new(0, 255, 0)));
    assert_eq!(plot.series_mgr.series[0].marker_size, Some(10.0));

    let result = plot.render();
    assert!(result.is_ok());
}

#[test]
fn test_streaming_version_changes_on_data_update() {
    use crate::data::StreamingXY;

    let stream = StreamingXY::new(100);

    let v0 = stream.version();
    stream.push(1.0, 1.0);
    let v1 = stream.version();

    assert!(v1 > v0, "Version should increase after push");

    // Create plot while keeping the stream live.
    let _plot = Plot::new().line_streaming(&stream).end_series();

    // Push more data
    stream.push(2.0, 2.0);
    let v2 = stream.version();

    assert!(v2 > v1, "Version should increase after second push");
}

#[test]
fn test_plot_is_reactive_for_reactive_title() {
    use crate::data::Observable;

    let title = Observable::new("Reactive Title".to_string());
    let plot = Plot::new()
        .title(title)
        .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0])
        .end_series();

    assert!(plot.is_reactive());
}

// ========== Error Bar Modifier API Tests ==========

#[test]
fn test_with_yerr_symmetric() {
    let x = vec![1.0, 2.0, 3.0];
    let y = vec![2.0, 4.0, 3.0];
    let yerr = vec![0.3, 0.4, 0.25];

    let plot = Plot::new()
        .line(&x, &y)
        .with_yerr(&yerr)
        .label("Test")
        .end_series();

    assert_eq!(plot.series_mgr.series.len(), 1);
    assert!(plot.series_mgr.series[0].y_errors.is_some());
    assert!(plot.series_mgr.series[0].x_errors.is_none());

    // Verify it's symmetric error values
    if let Some(ErrorValues::Symmetric(errs)) = &plot.series_mgr.series[0].y_errors {
        assert_eq!(errs.len(), 3);
        assert!((errs[0] - 0.3).abs() < 1e-10);
    } else {
        panic!("Expected symmetric error values");
    }
}

#[test]
fn test_with_xerr_symmetric() {
    let x = vec![1.0, 2.0, 3.0];
    let y = vec![2.0, 4.0, 3.0];
    let xerr = vec![0.15, 0.2, 0.1];

    let plot = Plot::new()
        .scatter(&x, &y)
        .with_xerr(&xerr)
        .label("Test")
        .end_series();

    assert_eq!(plot.series_mgr.series.len(), 1);
    assert!(plot.series_mgr.series[0].x_errors.is_some());
    assert!(plot.series_mgr.series[0].y_errors.is_none());

    if let Some(ErrorValues::Symmetric(errs)) = &plot.series_mgr.series[0].x_errors {
        assert_eq!(errs.len(), 3);
        assert!((errs[1] - 0.2).abs() < 1e-10);
    } else {
        panic!("Expected symmetric error values");
    }
}

#[test]
fn test_with_yerr_asymmetric() {
    let x = vec![1.0, 2.0, 3.0];
    let y = vec![2.0, 4.0, 3.0];
    let lower = vec![0.2, 0.3, 0.2];
    let upper = vec![0.5, 0.6, 0.4];

    let plot = Plot::new()
        .line(&x, &y)
        .with_yerr_asymmetric(&lower, &upper)
        .label("Test")
        .end_series();

    assert_eq!(plot.series_mgr.series.len(), 1);
    assert!(plot.series_mgr.series[0].y_errors.is_some());

    if let Some(ErrorValues::Asymmetric(lo, hi)) = &plot.series_mgr.series[0].y_errors {
        assert_eq!(lo.len(), 3);
        assert_eq!(hi.len(), 3);
        assert!((lo[0] - 0.2).abs() < 1e-10);
        assert!((hi[0] - 0.5).abs() < 1e-10);
    } else {
        panic!("Expected asymmetric error values");
    }
}

#[test]
fn test_with_xerr_asymmetric() {
    let x = vec![1.0, 2.0, 3.0];
    let y = vec![2.0, 4.0, 3.0];
    let left = vec![0.1, 0.15, 0.1];
    let right = vec![0.2, 0.25, 0.2];

    let plot = Plot::new()
        .scatter(&x, &y)
        .with_xerr_asymmetric(&left, &right)
        .label("Test")
        .end_series();

    assert_eq!(plot.series_mgr.series.len(), 1);
    assert!(plot.series_mgr.series[0].x_errors.is_some());

    if let Some(ErrorValues::Asymmetric(lo, hi)) = &plot.series_mgr.series[0].x_errors {
        assert_eq!(lo.len(), 3);
        assert_eq!(hi.len(), 3);
        assert!((lo[1] - 0.15).abs() < 1e-10);
        assert!((hi[1] - 0.25).abs() < 1e-10);
    } else {
        panic!("Expected asymmetric error values");
    }
}

#[test]
fn test_error_config() {
    let x = vec![1.0, 2.0, 3.0];
    let y = vec![2.0, 4.0, 3.0];
    let yerr = vec![0.3, 0.4, 0.25];

    let config = ErrorBarConfig::default().cap_size(0.15).line_width(2.0);

    let plot = Plot::new()
        .line(&x, &y)
        .with_yerr(&yerr)
        .error_config(config)
        .label("Test")
        .end_series();

    assert!(plot.series_mgr.series[0].error_config.is_some());
    let cfg = plot.series_mgr.series[0].error_config.as_ref().unwrap();
    assert!((cfg.cap_size - 0.15).abs() < 1e-10);
    assert!((cfg.line_width - 2.0).abs() < 1e-10);
}

#[test]
fn test_combined_xy_errors() {
    let x = vec![1.0, 2.0, 3.0];
    let y = vec![2.0, 4.0, 3.0];
    let xerr = vec![0.15, 0.2, 0.1];
    let yerr = vec![0.3, 0.4, 0.25];

    let plot = Plot::new()
        .scatter(&x, &y)
        .with_yerr(&yerr)
        .with_xerr(&xerr)
        .label("Test")
        .end_series();

    assert_eq!(plot.series_mgr.series.len(), 1);
    assert!(plot.series_mgr.series[0].y_errors.is_some());
    assert!(plot.series_mgr.series[0].x_errors.is_some());
}

#[test]
fn test_error_bars_continuation_method() {
    let x = vec![1.0, 2.0, 3.0];
    let y1 = vec![2.0, 4.0, 3.0];
    let y1_err = vec![0.3, 0.4, 0.25];
    let y2 = vec![1.5, 3.5, 2.5];
    let y2_err = vec![0.2, 0.3, 0.2];

    let plot = Plot::new()
        .error_bars(&x, &y1, &y1_err)
        .label("Series A")
        .error_bars(&x, &y2, &y2_err) // Continuation method
        .label("Series B")
        .end_series();

    assert_eq!(plot.series_mgr.series.len(), 2);
    assert!(matches!(
        &plot.series_mgr.series[0].series_type,
        SeriesType::ErrorBars { .. }
    ));
    assert!(matches!(
        &plot.series_mgr.series[1].series_type,
        SeriesType::ErrorBars { .. }
    ));
}

#[test]
fn test_line_with_error_bars_renders() {
    use crate::render::{SkiaRenderer, Theme};

    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![2.0, 4.0, 3.0, 5.0, 4.5];
    let yerr = vec![0.3, 0.4, 0.25, 0.5, 0.35];

    let plot = Plot::new()
        .line(&x, &y)
        .with_yerr(&yerr)
        .title("Line with Error Bars")
        .end_series();

    let mut renderer = SkiaRenderer::new(400, 300, Theme::default()).unwrap();
    let result = plot.render_to_renderer(&mut renderer, 96.0);
    assert!(result.is_ok());
}

#[test]
fn test_scatter_with_xy_error_bars_renders() {
    use crate::render::{SkiaRenderer, Theme};

    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![2.0, 4.0, 3.0, 5.0, 4.5];
    let xerr = vec![0.15, 0.2, 0.1, 0.15, 0.2];
    let yerr = vec![0.3, 0.4, 0.25, 0.5, 0.35];

    let plot = Plot::new()
        .scatter(&x, &y)
        .with_yerr(&yerr)
        .with_xerr(&xerr)
        .title("Scatter with XY Error Bars")
        .end_series();

    let mut renderer = SkiaRenderer::new(400, 300, Theme::default()).unwrap();
    let result = plot.render_to_renderer(&mut renderer, 96.0);
    assert!(result.is_ok());
}

#[test]
fn test_multiple_series_with_different_errors() {
    use crate::render::{SkiaRenderer, Theme};

    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y1 = vec![2.0, 4.0, 3.0, 5.0, 4.5];
    let y1_err = vec![0.3, 0.4, 0.25, 0.5, 0.35];
    let y2 = vec![1.5, 3.5, 2.5, 4.5, 4.0];
    let y2_err = vec![0.2, 0.3, 0.2, 0.4, 0.3];

    let plot = Plot::new()
        .line(&x, &y1)
        .with_yerr(&y1_err)
        .label("Series A")
        .scatter(&x, &y2)
        .with_yerr(&y2_err)
        .label("Series B")
        .title("Multiple Series with Error Bars")
        .end_series();

    let mut renderer = SkiaRenderer::new(400, 300, Theme::default()).unwrap();
    let result = plot.render_to_renderer(&mut renderer, 96.0);
    assert!(result.is_ok());
}

// ========== max_resolution Tests ==========

#[test]
fn test_max_resolution_height_constrained() {
    // Default 4:3 figure (6.4x4.8) fitting into 1920x1080 (16:9)
    // Should be height-constrained → 1440x1080
    let plot = Plot::new().max_resolution(1920, 1080);

    assert_eq!(plot.display.dimensions, (1440, 1080));
    // DPI should be 1080 / 4.8 = 225
    assert!((plot.display.config.figure.dpi - 225.0).abs() < 1.0);
}

#[test]
fn test_max_resolution_width_constrained() {
    // 4:3 figure fitting into 800x800 (square)
    // Should be width-constrained → 800x600
    let plot = Plot::new().max_resolution(800, 800);

    assert_eq!(plot.display.dimensions, (800, 600));
    // DPI should be 800 / 6.4 = 125
    assert!((plot.display.config.figure.dpi - 125.0).abs() < 1.0);
}

#[test]
fn test_max_resolution_exact_fit() {
    // 4:3 figure fitting into 1920x1440 (exactly 4:3)
    // Should fit exactly → 1920x1440
    let plot = Plot::new().max_resolution(1920, 1440);

    assert_eq!(plot.display.dimensions, (1920, 1440));
    // DPI should be 1920 / 6.4 = 300
    assert!((plot.display.config.figure.dpi - 300.0).abs() < 1.0);
}

#[test]
fn test_max_resolution_custom_figure() {
    // 16:9 figure fitting into 1920x1080
    let plot = Plot::new().size(16.0, 9.0).max_resolution(1920, 1080);

    // Should fit exactly for 16:9
    assert_eq!(plot.display.dimensions, (1920, 1080));
    // DPI should be 1920 / 16.0 = 120
    assert!((plot.display.config.figure.dpi - 120.0).abs() < 1.0);
}

#[test]
fn test_max_resolution_equivalent_to_dpi() {
    // max_resolution(1920, 1440) should produce same result as dpi(300)
    let plot_max_res = Plot::new().max_resolution(1920, 1440);
    let plot_dpi = Plot::new().dpi(300);

    assert_eq!(plot_max_res.display.dimensions, plot_dpi.display.dimensions);
    assert!(
        (plot_max_res.display.config.figure.dpi - plot_dpi.display.config.figure.dpi).abs() < 1.0
    );
}

#[test]
fn test_set_output_pixels_uses_actual_dpi_for_geometry() {
    let plot = Plot::with_config(PlotConfig {
        figure: FigureConfig::new(6.4, 4.8, 0.5),
        ..PlotConfig::default()
    })
    .set_output_pixels(800, 600);

    assert!((plot.display.config.figure.width - 1600.0).abs() < f32::EPSILON);
    assert!((plot.display.config.figure.height - 1200.0).abs() < f32::EPSILON);
    assert_eq!(plot.display.dimensions, (800, 600));
}

#[test]
fn test_set_output_pixels_with_zero_dpi_keeps_direct_dpi_error() {
    let err = Plot::with_config(PlotConfig {
        figure: FigureConfig::new(6.4, 4.8, 0.0),
        ..PlotConfig::default()
    })
    .set_output_pixels(800, 600)
    .line(&[0.0, 1.0], &[1.0, 2.0])
    .render()
    .expect_err("zero DPI should still fail with the direct DPI validation error");

    assert!(matches!(
        err,
        PlottingError::InvalidInput(message)
            if message.contains("Figure DPI must be positive") && message.contains("0")
    ));
}

#[test]
fn test_render_rejects_non_positive_figure_width_before_sanitizing() {
    let mut plot = Plot::new().line(&[0.0, 1.0], &[1.0, 2.0]).end_series();
    plot.display.config.figure = FigureConfig::new(0.0, 4.8, 100.0);

    let err = plot
        .render()
        .expect_err("non-positive figure width should fail validation");

    assert!(matches!(
        err,
        PlottingError::InvalidDimensions {
            width: 0,
            height: 480
        }
    ));
}

#[test]
fn test_plot_builder_can_chain_histogram_without_end_series() {
    let plot: Plot = Plot::new()
        .line(&[0.0, 10.0], &[0.0, 1.0])
        .histogram(&[1.0, 2.0, 3.0, 4.0], None)
        .into();

    assert_eq!(plot.series_mgr.series.len(), 2);
    assert!(matches!(
        plot.series_mgr.series[0].series_type,
        SeriesType::Line { .. }
    ));
    assert!(matches!(
        plot.series_mgr.series[1].series_type,
        SeriesType::Histogram { .. }
    ));
}

#[test]
fn test_static_histogram_prepares_histogram_data() {
    let plot: Plot = Plot::new().histogram(&[1.0, 2.0, 3.0, 4.0], None).into();

    match &plot.series_mgr.series[0].series_type {
        SeriesType::Histogram { prepared, .. } => {
            let prepared = prepared.as_ref().expect("expected prepared histogram data");
            assert!(!prepared.counts.is_empty());
            assert_eq!(prepared.bin_edges.len(), prepared.counts.len() + 1);
        }
        other => panic!("expected histogram series, got {other:?}"),
    }
}

#[test]
fn test_histogram_source_keeps_prepared_histogram_lazy() {
    let plot: Plot = Plot::new()
        .histogram_source(vec![1.0, 2.0, 3.0, 4.0], None)
        .into();

    match &plot.series_mgr.series[0].series_type {
        SeriesType::Histogram { prepared, .. } => assert!(prepared.is_none()),
        other => panic!("expected histogram series, got {other:?}"),
    }
}

#[test]
fn test_plot_builder_can_add_styled_vline_without_end_series() {
    let plot: Plot = Plot::new()
        .line(&[0.0, 10.0], &[0.0, 1.0])
        .vline_styled(5.0, Color::RED, 2.0, LineStyle::Dashed)
        .into();

    assert_eq!(plot.series_mgr.series.len(), 1);
    assert_eq!(plot.annotations.len(), 1);
    assert!(matches!(
        plot.annotations[0],
        Annotation::VLine { x, .. } if (x - 5.0).abs() < f64::EPSILON
    ));
}

#[test]
fn test_plot_series_builder_can_chain_boxplot_without_end_series() {
    let plot: Plot = Plot::new()
        .histogram(&[1.0, 2.0, 3.0, 4.0], None)
        .boxplot(&[2.0, 3.0, 5.0, 8.0], None)
        .into();

    assert_eq!(plot.series_mgr.series.len(), 2);
    assert!(matches!(
        plot.series_mgr.series[0].series_type,
        SeriesType::Histogram { .. }
    ));
    assert!(matches!(
        plot.series_mgr.series[1].series_type,
        SeriesType::BoxPlot { .. }
    ));
}

#[test]
fn test_mixed_coordinate_plots_keep_cartesian_axes() {
    let theta = vec![0.0, std::f64::consts::PI * 0.5, std::f64::consts::PI];
    let r = vec![1.0, 2.0, 1.5];
    let plot: Plot = Plot::new()
        .line(&[0.0, 10.0], &[0.0, 1.0])
        .polar_line(&r, &theta)
        .into();

    assert!(plot.needs_cartesian_axes());
    assert!(plot.series_mgr.series[1].inset_layout.is_some());
}

#[test]
fn test_non_cartesian_builder_inset_layout_is_stored() {
    let theta = vec![0.0, std::f64::consts::PI * 0.5, std::f64::consts::PI];
    let r = vec![1.0, 2.0, 1.5];
    let plot: Plot = Plot::new()
        .line(&[0.0, 10.0], &[0.0, 1.0])
        .polar_line(&r, &theta)
        .inset_anchor(InsetAnchor::BottomLeft)
        .inset_size_frac(0.4, 0.25)
        .inset_margin_pt(18.0)
        .into();

    let layout = plot.series_mgr.series[1]
        .inset_layout
        .expect("polar series should store inset metadata");
    assert_eq!(layout.anchor, InsetAnchor::BottomLeft);
    assert!((layout.width_frac - 0.4).abs() < f32::EPSILON);
    assert!((layout.height_frac - 0.25).abs() < f32::EPSILON);
    assert!((layout.margin_pt - 18.0).abs() < f32::EPSILON);
}

#[test]
fn test_mixed_cartesian_polar_raster_render_succeeds() {
    let theta = vec![0.0, std::f64::consts::PI * 0.5, std::f64::consts::PI];
    let r = vec![1.0, 2.0, 1.5];

    let image = Plot::new()
        .line(&[0.0, 10.0], &[0.0, 1.0])
        .polar_line(&r, &theta)
        .render()
        .expect("mixed polar raster render should succeed");

    assert!(!image.pixels.is_empty());
}

#[test]
fn test_mixed_cartesian_polar_renders_svg_with_inset_geometry() {
    let theta = vec![0.0, std::f64::consts::PI * 0.5, std::f64::consts::PI];
    let r = vec![1.0, 2.0, 1.5];

    let svg = Plot::new()
        .line(&[0.0, 10.0], &[0.0, 1.0])
        .polar_line(&r, &theta)
        .render_to_svg()
        .expect("mixed polar SVG render should succeed");

    assert!(
        svg.matches("<polyline").count() >= 2,
        "expected both Cartesian and polar polylines in SVG: {svg}"
    );
    assert!(
        svg.contains("0°"),
        "expected polar theta labels in SVG: {svg}"
    );
}

#[test]
fn test_polar_svg_scales_line_width_and_markers_with_dpi() {
    let theta = vec![0.0, std::f64::consts::PI * 0.5, std::f64::consts::PI];
    let r = vec![1.0, 2.0, 1.5];

    let plot: Plot = Plot::new()
        .dpi(200)
        .polar_line(&r, &theta)
        .marker_size(13.0)
        .into();
    let (expected_stroke_width, expected_marker_radius) =
        match &plot.series_mgr.series[0].series_type {
            SeriesType::Polar { data } => (
                plot.render_scale().points_to_pixels(data.config.line_width),
                plot.render_scale()
                    .points_to_pixels(data.config.marker_size)
                    / 2.0,
            ),
            other => panic!("expected polar series, got {other:?}"),
        };
    let svg = plot
        .render_to_svg()
        .expect("polar SVG render should succeed");

    assert!(
        svg.contains(&format!(r#"stroke-width="{expected_stroke_width:.2}""#)),
        "expected polar line width to scale with DPI: {svg}"
    );
    assert!(
        svg.contains(&format!(r#"r="{expected_marker_radius:.2}" fill=""#)),
        "expected polar marker radius to scale with DPI: {svg}"
    );
}

#[test]
fn test_pie_svg_scales_edge_width_with_dpi() {
    let mut plot_100: Plot = Plot::new().dpi(100).pie(&[2.0, 3.0, 4.0]).into();
    let mut plot_200: Plot = Plot::new().dpi(200).pie(&[2.0, 3.0, 4.0]).into();

    for plot in [&mut plot_100, &mut plot_200] {
        let SeriesType::Pie { data } = &mut plot.series_mgr.series[0].series_type else {
            panic!("expected pie series");
        };
        data.config.edge_color = Some(Color::BLACK);
        data.config.edge_width = 2.5;
    }

    let svg_100 = plot_100.render_to_svg().expect("100 DPI pie SVG render");
    let svg_200 = plot_200.render_to_svg().expect("200 DPI pie SVG render");

    let width_100 = extract_svg_root_attr(&svg_100, "width");
    let width_200 = extract_svg_root_attr(&svg_200, "width");
    let stroke_100 = extract_first_stroked_svg_polygon_stroke_width(&svg_100);
    let stroke_200 = extract_first_stroked_svg_polygon_stroke_width(&svg_200);

    let ratio_100 = stroke_100 / width_100;
    let ratio_200 = stroke_200 / width_200;

    assert!(
        (ratio_100 - ratio_200).abs() < 0.0005,
        "pie edge stroke-to-canvas ratio should remain stable across DPI: {} vs {}",
        ratio_100,
        ratio_200
    );
}

#[test]
fn test_auto_placed_insets_preserve_gap_with_mixed_sizes() {
    let theta = vec![0.0, std::f64::consts::PI * 0.5, std::f64::consts::PI];
    let r = vec![1.0, 2.0, 1.5];
    let plot: Plot = Plot::new()
        .line(&[0.0, 10.0], &[0.0, 1.0])
        .pie(&[2.0, 3.0, 4.0])
        .inset_size_frac(0.18, 0.18)
        .polar_line(&r, &theta)
        .inset_size_frac(0.35, 0.35)
        .into();

    let plot_area =
        tiny_skia::Rect::from_ltrb(0.0, 0.0, 1000.0, 800.0).expect("valid test plot area");
    let rects = plot
        .inset_rects_for_series(&plot.series_mgr.series, plot_area, plot.render_scale())
        .expect("auto inset rects should be computed");

    let right_inset = rects[1].expect("pie inset rect");
    let left_inset = rects[2].expect("polar inset rect");
    let actual_gap = right_inset.x() - (left_inset.x() + left_inset.width());
    let expected_gap = plot
        .render_scale()
        .points_to_pixels(InsetLayout::DEFAULT_MARGIN_PT)
        .max(4.0);

    assert!(
        (actual_gap - expected_gap).abs() < 0.01,
        "auto insets should keep a constant inter-column gap: {} vs {}",
        actual_gap,
        expected_gap
    );
}

#[test]
fn test_radar_plot_area_centers_portrait_insets_before_title_clearance() {
    let plot_area =
        tiny_skia::Rect::from_ltrb(100.0, 200.0, 300.0, 600.0).expect("valid test rect");
    let area = Plot::radar_plot_area(plot_area, -1.25, 1.25, -1.25, 1.25);

    assert!(
        (area.x - 100.0).abs() < 0.01,
        "unexpected radar inset x: {}",
        area.x
    );
    assert!(
        (area.y - 340.0).abs() < 0.01,
        "unexpected radar inset y: {}",
        area.y
    );
    assert!(
        (area.width - 160.0).abs() < 0.01,
        "unexpected radar inset width: {}",
        area.width
    );
    assert!(
        (area.height - 160.0).abs() < 0.01,
        "unexpected radar inset height: {}",
        area.height
    );
}

#[test]
fn test_mixed_cartesian_pie_renders_svg_with_inset_polygons() {
    let svg = Plot::new()
        .line(&[0.0, 1.0, 2.0], &[1.0, 3.0, 2.0])
        .pie(&[2.0, 3.0, 4.0])
        .labels(&["A", "B", "C"])
        .render_to_svg()
        .expect("mixed pie SVG render should succeed");

    assert!(
        svg.matches("<polygon").count() >= 3,
        "expected pie wedge polygons in SVG: {svg}"
    );
    assert!(
        svg.contains("22.2%"),
        "expected pie percentage labels in SVG: {svg}"
    );
    assert!(
        svg.matches("<clipPath").count() >= 2,
        "expected a nested inset clip path in addition to the main plot clip: {svg}"
    );
}

#[test]
fn test_mixed_cartesian_radar_renders_svg_with_inset_geometry() {
    let svg = Plot::new()
        .line(&[0.0, 1.0, 2.0], &[1.0, 3.0, 2.0])
        .radar(&["Speed", "Power", "Skill"])
        .add_series("Alpha", &[1.0, 2.0, 3.0])
        .render_to_svg()
        .expect("mixed radar SVG render should succeed");

    assert!(
        svg.matches("<polygon").count() >= 1,
        "expected radar polygon geometry in SVG: {svg}"
    );
    assert!(
        svg.contains(">Speed<"),
        "expected radar axis labels in SVG: {svg}"
    );
}

// ========== IntoPlot Trait Tests ==========

#[test]
fn test_from_plot_series_builder_for_plot() {
    // Test From<PlotSeriesBuilder> for Plot
    let x_data = vec![1.0, 2.0, 3.0];
    let y_data = vec![2.0, 4.0, 3.0];

    let builder = Plot::new()
        .line(&x_data, &y_data)
        .color(crate::render::Color::RED)
        .label("Test Series");

    // Convert via .into()
    let plot: Plot = builder.into();

    // Verify the series was added
    assert_eq!(plot.series_mgr.series.len(), 1);
    assert_eq!(
        plot.series_mgr.series[0].label,
        Some("Test Series".to_string())
    );
}

#[test]
fn test_into_plot_trait_for_plot() {
    use builder::IntoPlot;

    let plot = Plot::new().title("Test");
    let converted = plot.into_plot();

    // Verify title is set (PlotText doesn't impl PartialEq, so check via pattern match)
    match &converted.display.title {
        Some(data::PlotText::Static(s)) => assert_eq!(s, "Test"),
        _ => panic!("Expected Static PlotText with 'Test'"),
    }
}

#[test]
fn test_into_plot_trait_for_plot_series_builder() {
    use builder::IntoPlot;

    let x_data = vec![1.0, 2.0, 3.0];
    let y_data = vec![2.0, 4.0, 3.0];

    let builder = Plot::new().line(&x_data, &y_data).label("Via IntoPlot");

    let plot = builder.into_plot();

    assert_eq!(plot.series_mgr.series.len(), 1);
    assert_eq!(
        plot.series_mgr.series[0].label,
        Some("Via IntoPlot".to_string())
    );
}

#[test]
fn test_as_plot_for_plot_series_builder() {
    use builder::IntoPlot;

    let x_data = vec![1.0, 2.0, 3.0];
    let y_data = vec![2.0, 4.0, 3.0];

    let builder = Plot::new().title("Inspectable").line(&x_data, &y_data);

    // as_plot() allows inspection without consuming
    let plot_ref = builder.as_plot();
    match &plot_ref.display.title {
        Some(data::PlotText::Static(s)) => assert_eq!(s, "Inspectable"),
        _ => panic!("Expected Static PlotText with 'Inspectable'"),
    }

    // Builder is still usable after as_plot()
    let plot = builder.into_plot();
    assert_eq!(plot.series_mgr.series.len(), 1);
}

#[test]
fn test_order_independent_method_chaining() {
    // Test that plot-level methods can be called in any order
    let x_data = vec![1.0, 2.0, 3.0];
    let y_data = vec![2.0, 4.0, 3.0];

    // Version 1: Plot config before series
    let plot1: Plot = Plot::new()
        .title("My Plot")
        .xlabel("X")
        .line(&x_data, &y_data)
        .into();

    // Version 2: Plot config after series
    let plot2: Plot = Plot::new()
        .line(&x_data, &y_data)
        .title("My Plot")
        .xlabel("X")
        .into();

    // Both should have the same configuration
    // Verify titles match (PlotText doesn't impl PartialEq)
    match (&plot1.display.title, &plot2.display.title) {
        (Some(data::PlotText::Static(s1)), Some(data::PlotText::Static(s2))) => {
            assert_eq!(s1, s2);
        }
        _ => panic!("Expected matching Static PlotText titles"),
    }
    match (&plot1.display.xlabel, &plot2.display.xlabel) {
        (Some(data::PlotText::Static(s1)), Some(data::PlotText::Static(s2))) => {
            assert_eq!(s1, s2);
        }
        _ => panic!("Expected matching Static PlotText xlabels"),
    }
    assert_eq!(plot1.series_mgr.series.len(), plot2.series_mgr.series.len());
}

#[test]
fn test_generic_function_with_into_plot() {
    use builder::IntoPlot;

    // Function that accepts any IntoPlot type
    fn count_series(p: impl IntoPlot) -> usize {
        p.into_plot().series_mgr.series.len()
    }

    let x_data = vec![1.0, 2.0, 3.0];
    let y_data = vec![2.0, 4.0, 3.0];

    // Works with Plot
    assert_eq!(count_series(Plot::new()), 0);

    // Works with PlotSeriesBuilder
    let builder = Plot::new().line(&x_data, &y_data);
    assert_eq!(count_series(builder), 1);
}

#[test]
fn test_implicit_conversion_in_function_param() {
    // Test that Into<Plot> works for function parameters
    fn accepts_into_plot(p: impl Into<Plot>) -> Plot {
        p.into()
    }

    let x_data = vec![1.0, 2.0, 3.0];
    let y_data = vec![2.0, 4.0, 3.0];

    let builder = Plot::new().line(&x_data, &y_data).label("Implicit");
    let plot = accepts_into_plot(builder);

    assert_eq!(plot.series_mgr.series.len(), 1);
    assert_eq!(
        plot.series_mgr.series[0].label,
        Some("Implicit".to_string())
    );
}

#[cfg(feature = "typst-math")]
mod typst;
