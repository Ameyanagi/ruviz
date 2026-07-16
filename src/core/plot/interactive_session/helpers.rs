use super::*;

pub(super) fn compose_images(base: &Image, overlay: &Image) -> Image {
    let mut pixels = base.pixels.clone();
    for (dst, src) in pixels
        .chunks_exact_mut(4)
        .zip(overlay.pixels.chunks_exact(4))
    {
        let alpha = src[3] as f32 / 255.0;
        if alpha <= 0.0 {
            continue;
        }
        dst[0] = blend_channel(dst[0], src[0], alpha);
        dst[1] = blend_channel(dst[1], src[1], alpha);
        dst[2] = blend_channel(dst[2], src[2], alpha);
        dst[3] = 255;
    }
    Image::new(base.width, base.height, pixels)
}

pub(super) fn collect_streaming_draw_ops(
    plot: &Plot,
    frame: &ResolvedFrame<'_>,
    size_px: (u32, u32),
    scale_factor: f32,
    _time_seconds: f64,
) -> Result<Option<Vec<StreamingDrawOp>>> {
    if plot
        .display
        .title
        .as_ref()
        .is_some_and(|title| title.is_reactive())
        || plot
            .display
            .xlabel
            .as_ref()
            .is_some_and(|label| label.is_reactive())
        || plot
            .display
            .ylabel
            .as_ref()
            .is_some_and(|label| label.is_reactive())
        || plot
            .series_mgr
            .series
            .iter()
            .any(PlotSeries::has_reactive_style_sources)
    {
        return Ok(None);
    }

    let prepared_plot = plot.prepared_frame_shell_with_style(size_px, scale_factor, &frame.style);
    let mut draw_ops = Vec::new();
    let mut saw_streaming_update = false;

    for (series_index, series) in plot.series_mgr.series.iter().enumerate() {
        let style = frame.style.series.get(series_index).ok_or_else(|| {
            PlottingError::RenderError("resolved series style count mismatch".to_string())
        })?;
        let color = style
            .color
            .with_alpha((f32::from(style.color.a) / 255.0) * style.alpha);
        let line_width_px = prepared_plot.line_width_px(style.line_width.unwrap_or(2.0));
        let dash_pattern = style.line_style.clone();
        let marker_style = style.marker_style.unwrap_or(MarkerStyle::Circle);
        let marker_size = match series.series_type {
            SeriesType::Scatter { .. } => style.marker_size.unwrap_or(10.0),
            _ => style.marker_size.unwrap_or(8.0),
        };
        let marker_size_px = prepared_plot.line_width_px(marker_size);

        match &series.series_type {
            SeriesType::Line { x_data, y_data } => {
                let Some(op) = streaming_draw_op(
                    series,
                    x_data,
                    y_data,
                    frame,
                    StreamingDrawKind::Line,
                    color,
                    line_width_px,
                    dash_pattern,
                    marker_style,
                    marker_size_px,
                )?
                else {
                    if series.series_type.is_reactive() {
                        return Ok(None);
                    }
                    continue;
                };
                saw_streaming_update = true;
                draw_ops.push(op);
            }
            SeriesType::Scatter { x_data, y_data } => {
                let Some(op) = streaming_draw_op(
                    series,
                    x_data,
                    y_data,
                    frame,
                    StreamingDrawKind::Scatter,
                    color,
                    line_width_px,
                    dash_pattern,
                    marker_style,
                    marker_size_px,
                )?
                else {
                    if series.series_type.is_reactive() {
                        return Ok(None);
                    }
                    continue;
                };
                saw_streaming_update = true;
                draw_ops.push(op);
            }
            _ if series.series_type.is_reactive() => return Ok(None),
            _ => {}
        }
    }

    Ok(saw_streaming_update.then_some(draw_ops))
}

pub(super) fn streaming_draw_op(
    series: &PlotSeries,
    x_data: &PlotData,
    y_data: &PlotData,
    frame: &ResolvedFrame<'_>,
    kind: StreamingDrawKind,
    color: Color,
    line_width_px: f32,
    line_style: LineStyle,
    marker_style: MarkerStyle,
    marker_size_px: f32,
) -> Result<Option<StreamingDrawOp>> {
    if !matches!(x_data, PlotData::Streaming(_)) || !matches!(y_data, PlotData::Streaming(_)) {
        return Ok(None);
    }
    let Some(source) = series.streaming_source.as_ref() else {
        return Ok(None);
    };
    let Some(snapshot) = frame
        .paired_acknowledgements
        .iter()
        .find(|snapshot| snapshot.source.shares_source(source))
    else {
        return Ok(None);
    };
    let appended_count = match snapshot.render_state {
        crate::data::StreamingRenderState::AppendOnly { visible_appended } => visible_appended,
        _ => return Ok(None),
    };
    if appended_count == 0 {
        return Ok(None);
    }

    let x_values = snapshot.x.as_ref();
    let y_values = snapshot.y.as_ref();
    let len = x_values.len().min(y_values.len());
    if len == 0 || appended_count > len {
        return Ok(None);
    }

    let split_index = len - appended_count;
    let previous_point = if split_index > 0 {
        Some((x_values[split_index - 1], y_values[split_index - 1]))
    } else {
        None
    };
    let mut points = Vec::with_capacity(appended_count);
    for (&x, &y) in x_values[split_index..len]
        .iter()
        .zip(&y_values[split_index..len])
    {
        if x.is_finite() && y.is_finite() {
            points.push((x, y));
        }
    }

    if points.is_empty() {
        return Ok(None);
    }

    Ok(Some(StreamingDrawOp {
        kind,
        points,
        previous_point,
        color,
        line_width_px,
        line_style,
        marker_style,
        marker_size_px,
        draw_markers: kind == StreamingDrawKind::Scatter || series.marker_style.is_some(),
    }))
}

pub(super) fn apply_streaming_draw_ops(
    base: &Image,
    geometry: &GeometrySnapshot,
    draw_ops: &[StreamingDrawOp],
) -> Result<Image> {
    let size =
        tiny_skia::IntSize::from_wh(base.width, base.height).ok_or(PlottingError::InvalidData {
            message: "Invalid frame size for incremental streaming render".to_string(),
            position: None,
        })?;
    let mut pixmap = tiny_skia::Pixmap::from_vec(base.pixels.clone(), size).ok_or(
        PlottingError::RenderError("Failed to create incremental streaming pixmap".to_string()),
    )?;
    let clip_mask = create_geometry_clip_mask(base.width, base.height, geometry.plot_area)?;

    for op in draw_ops {
        let mut mapped_points: Vec<(f32, f32)> = Vec::with_capacity(op.points.len());
        for &(x, y) in &op.points {
            let point = geometry.data_to_screen(ViewportPoint::new(x, y));
            mapped_points.push((point.x as f32, point.y as f32));
        }

        if op.kind == StreamingDrawKind::Line {
            draw_incremental_polyline(
                &mut pixmap,
                geometry,
                op.previous_point,
                &mapped_points,
                op.color,
                op.line_width_px,
                &op.line_style,
                Some(&clip_mask),
            )?;
        }

        if op.draw_markers {
            for &(px, py) in &mapped_points {
                draw_incremental_marker(
                    &mut pixmap,
                    px,
                    py,
                    op.marker_size_px,
                    op.marker_style,
                    op.color,
                    Some(&clip_mask),
                )?;
            }
        }
    }

    Ok(Image::new(base.width, base.height, pixmap.take()))
}

pub(super) fn draw_incremental_polyline(
    pixmap: &mut tiny_skia::Pixmap,
    geometry: &GeometrySnapshot,
    previous_point: Option<(f64, f64)>,
    points: &[(f32, f32)],
    color: Color,
    line_width_px: f32,
    line_style: &LineStyle,
    mask: Option<&tiny_skia::Mask>,
) -> Result<()> {
    let mut path = tiny_skia::PathBuilder::new();

    if let Some((x, y)) = previous_point {
        let point = geometry.data_to_screen(ViewportPoint::new(x, y));
        path.move_to(point.x as f32, point.y as f32);
    } else if let Some(&(px, py)) = points.first() {
        path.move_to(px, py);
    } else {
        return Ok(());
    }

    for &(px, py) in points {
        path.line_to(px, py);
    }

    let Some(path) = path.finish() else {
        return Ok(());
    };

    let mut paint = tiny_skia::Paint::default();
    paint.set_color(color.to_tiny_skia_color());
    paint.anti_alias = true;

    let mut stroke = tiny_skia::Stroke {
        width: line_width_px.max(1.0),
        ..tiny_skia::Stroke::default()
    };
    if let Some(pattern) = line_style.to_dash_array() {
        stroke.dash = tiny_skia::StrokeDash::new(pattern, 0.0);
    }

    pixmap.stroke_path(
        &path,
        &paint,
        &stroke,
        tiny_skia::Transform::identity(),
        mask,
    );

    Ok(())
}

pub(super) fn draw_incremental_marker(
    pixmap: &mut tiny_skia::Pixmap,
    x: f32,
    y: f32,
    size: f32,
    style: MarkerStyle,
    color: Color,
    mask: Option<&tiny_skia::Mask>,
) -> Result<()> {
    let radius = size * 0.5;
    let mut paint = tiny_skia::Paint::default();
    paint.set_color(color.to_tiny_skia_color());
    paint.anti_alias = true;

    match style {
        MarkerStyle::Circle | MarkerStyle::CircleOpen => {
            let circle = tiny_skia::PathBuilder::from_circle(x, y, radius).ok_or(
                PlottingError::RenderError("Failed to create circle marker path".to_string()),
            )?;
            if style.is_filled() {
                pixmap.fill_path(
                    &circle,
                    &paint,
                    tiny_skia::FillRule::Winding,
                    tiny_skia::Transform::identity(),
                    mask,
                );
            } else {
                let stroke = tiny_skia::Stroke {
                    width: (size * 0.15).max(1.0),
                    ..tiny_skia::Stroke::default()
                };
                pixmap.stroke_path(
                    &circle,
                    &paint,
                    &stroke,
                    tiny_skia::Transform::identity(),
                    mask,
                );
            }
        }
        MarkerStyle::Square | MarkerStyle::SquareOpen => {
            let rect = tiny_skia::Rect::from_ltrb(x - radius, y - radius, x + radius, y + radius)
                .ok_or(PlottingError::RenderError(
                "Failed to create square marker path".to_string(),
            ))?;
            let path = tiny_skia::PathBuilder::from_rect(rect);
            if style.is_filled() {
                pixmap.fill_path(
                    &path,
                    &paint,
                    tiny_skia::FillRule::Winding,
                    tiny_skia::Transform::identity(),
                    mask,
                );
            } else {
                let stroke = tiny_skia::Stroke {
                    width: (size * 0.15).max(1.0),
                    ..tiny_skia::Stroke::default()
                };
                pixmap.stroke_path(
                    &path,
                    &paint,
                    &stroke,
                    tiny_skia::Transform::identity(),
                    mask,
                );
            }
        }
        MarkerStyle::Triangle | MarkerStyle::TriangleOpen | MarkerStyle::TriangleDown => {
            let mut path = tiny_skia::PathBuilder::new();
            if style == MarkerStyle::TriangleDown {
                path.move_to(x, y + radius);
                path.line_to(x - radius * 0.866, y - radius * 0.5);
                path.line_to(x + radius * 0.866, y - radius * 0.5);
            } else {
                path.move_to(x, y - radius);
                path.line_to(x - radius * 0.866, y + radius * 0.5);
                path.line_to(x + radius * 0.866, y + radius * 0.5);
            }
            path.close();
            let path = path.finish().ok_or(PlottingError::RenderError(
                "Failed to create triangle marker path".to_string(),
            ))?;
            if style.is_filled() {
                pixmap.fill_path(
                    &path,
                    &paint,
                    tiny_skia::FillRule::Winding,
                    tiny_skia::Transform::identity(),
                    mask,
                );
            } else {
                let stroke = tiny_skia::Stroke {
                    width: (size * 0.15).max(1.0),
                    ..tiny_skia::Stroke::default()
                };
                pixmap.stroke_path(
                    &path,
                    &paint,
                    &stroke,
                    tiny_skia::Transform::identity(),
                    mask,
                );
            }
        }
        MarkerStyle::Diamond | MarkerStyle::DiamondOpen => {
            let mut path = tiny_skia::PathBuilder::new();
            path.move_to(x, y - radius);
            path.line_to(x + radius, y);
            path.line_to(x, y + radius);
            path.line_to(x - radius, y);
            path.close();
            let path = path.finish().ok_or(PlottingError::RenderError(
                "Failed to create diamond marker path".to_string(),
            ))?;
            if style.is_filled() {
                pixmap.fill_path(
                    &path,
                    &paint,
                    tiny_skia::FillRule::Winding,
                    tiny_skia::Transform::identity(),
                    mask,
                );
            } else {
                let stroke = tiny_skia::Stroke {
                    width: (size * 0.15).max(1.0),
                    ..tiny_skia::Stroke::default()
                };
                pixmap.stroke_path(
                    &path,
                    &paint,
                    &stroke,
                    tiny_skia::Transform::identity(),
                    mask,
                );
            }
        }
        MarkerStyle::Plus | MarkerStyle::Cross | MarkerStyle::Star => {
            let stroke = tiny_skia::Stroke {
                width: (size * 0.25).max(1.0),
                ..tiny_skia::Stroke::default()
            };
            let mut path = tiny_skia::PathBuilder::new();
            if matches!(style, MarkerStyle::Plus | MarkerStyle::Star) {
                path.move_to(x - radius, y);
                path.line_to(x + radius, y);
                path.move_to(x, y - radius);
                path.line_to(x, y + radius);
            }
            if matches!(style, MarkerStyle::Cross | MarkerStyle::Star) {
                let offset = radius * 0.707;
                path.move_to(x - offset, y - offset);
                path.line_to(x + offset, y + offset);
                path.move_to(x - offset, y + offset);
                path.line_to(x + offset, y - offset);
            }
            let path = path.finish().ok_or(PlottingError::RenderError(
                "Failed to create cross marker path".to_string(),
            ))?;
            pixmap.stroke_path(
                &path,
                &paint,
                &stroke,
                tiny_skia::Transform::identity(),
                mask,
            );
        }
    }

    Ok(())
}

pub(super) fn create_geometry_clip_mask(
    width: u32,
    height: u32,
    plot_area: tiny_skia::Rect,
) -> Result<tiny_skia::Mask> {
    let mut mask = tiny_skia::Mask::new(width, height).ok_or(PlottingError::RenderError(
        "Failed to create incremental clip mask".to_string(),
    ))?;
    let clip_path = tiny_skia::PathBuilder::from_rect(plot_area);
    mask.fill_path(
        &clip_path,
        tiny_skia::FillRule::Winding,
        false,
        tiny_skia::Transform::identity(),
    );
    Ok(mask)
}

pub(super) fn blend_channel(background: u8, foreground: u8, alpha: f32) -> u8 {
    let bg = background as f32 / 255.0;
    let fg = foreground as f32 / 255.0;
    ((bg * (1.0 - alpha) + fg * alpha) * 255.0) as u8
}

/// Straight-alpha source-over composition of `color` onto one RGBA pixel.
///
/// Unlike a plain channel blend that overwrites destination alpha, this
/// preserves existing coverage (e.g. dynamic annotations underneath a brush
/// or hover marker) so the base layer does not bleed through.
fn blend_pixel_over(dst: &mut [u8], color: Color) {
    let src_a = color.a as f32 / 255.0;
    if src_a <= 0.0 {
        return;
    }
    let dst_a = dst[3] as f32 / 255.0;
    let out_a = src_a + dst_a * (1.0 - src_a);
    if out_a <= 0.0 {
        return;
    }
    let blend = |dst_c: u8, src_c: u8| -> u8 {
        let d = dst_c as f32 / 255.0;
        let s = src_c as f32 / 255.0;
        (((s * src_a + d * dst_a * (1.0 - src_a)) / out_a) * 255.0).round() as u8
    };
    dst[0] = blend(dst[0], color.r);
    dst[1] = blend(dst[1], color.g);
    dst[2] = blend(dst[2], color.b);
    dst[3] = (out_a * 255.0).round() as u8;
}

pub(super) fn draw_hit(
    pixels: &mut [u8],
    size_px: (u32, u32),
    hit: &HitResult,
    color: Color,
    clip: Option<ViewportRect>,
) {
    match hit {
        HitResult::SeriesPoint {
            screen_position, ..
        } => {
            draw_circle_clipped(pixels, size_px, *screen_position, 6.0, color, clip);
        }
        HitResult::HeatmapCell { screen_rect, .. } => {
            draw_rect_clipped(pixels, size_px, *screen_rect, color, clip)
        }
        HitResult::None => {}
    }
}

pub(super) fn draw_circle(
    pixels: &mut [u8],
    size_px: (u32, u32),
    center: ViewportPoint,
    radius: f64,
    color: Color,
) {
    draw_circle_clipped(pixels, size_px, center, radius, color, None);
}

fn draw_circle_clipped(
    pixels: &mut [u8],
    size_px: (u32, u32),
    center: ViewportPoint,
    radius: f64,
    color: Color,
    clip: Option<ViewportRect>,
) {
    let width = size_px.0 as i32;
    let height = size_px.1 as i32;
    let radius_sq = (radius * radius) as i32;
    let cx = center.x as i32;
    let cy = center.y as i32;

    for dy in -(radius as i32)..=(radius as i32) {
        for dx in -(radius as i32)..=(radius as i32) {
            if dx * dx + dy * dy > radius_sq {
                continue;
            }
            let x = cx + dx;
            let y = cy + dy;
            if x < 0 || y < 0 || x >= width || y >= height {
                continue;
            }
            if clip.is_some_and(|clip| {
                f64::from(x) < clip.min.x
                    || f64::from(x) > clip.max.x
                    || f64::from(y) < clip.min.y
                    || f64::from(y) > clip.max.y
            }) {
                continue;
            }
            let index = ((y * width + x) * 4) as usize;
            blend_pixel_over(&mut pixels[index..index + 4], color);
        }
    }
}

pub(super) fn draw_rect(pixels: &mut [u8], size_px: (u32, u32), rect: ViewportRect, color: Color) {
    draw_rect_clipped(pixels, size_px, rect, color, None);
}

fn draw_rect_clipped(
    pixels: &mut [u8],
    size_px: (u32, u32),
    rect: ViewportRect,
    color: Color,
    clip: Option<ViewportRect>,
) {
    let width = size_px.0 as i32;
    let height = size_px.1 as i32;
    let x1 = rect.min.x.round() as i32;
    let y1 = rect.min.y.round() as i32;
    let x2 = rect.max.x.round() as i32;
    let y2 = rect.max.y.round() as i32;

    let clip_x1 = clip.map_or(0, |clip| clip.min.x.ceil() as i32);
    let clip_y1 = clip.map_or(0, |clip| clip.min.y.ceil() as i32);
    let clip_x2 = clip.map_or(width, |clip| clip.max.x.floor() as i32 + 1);
    let clip_y2 = clip.map_or(height, |clip| clip.max.y.floor() as i32 + 1);

    for y in y1.max(0).max(clip_y1)..y2.min(height).min(clip_y2) {
        for x in x1.max(0).max(clip_x1)..x2.min(width).min(clip_x2) {
            let index = ((y * width + x) * 4) as usize;
            blend_pixel_over(&mut pixels[index..index + 4], color);
        }
    }
}

pub(super) fn draw_rect_outline(
    pixels: &mut [u8],
    size_px: (u32, u32),
    rect: ViewportRect,
    color: Color,
    thickness: i32,
) {
    let width = size_px.0 as i32;
    let height = size_px.1 as i32;
    let x1 = rect.min.x.round() as i32;
    let y1 = rect.min.y.round() as i32;
    let x2 = rect.max.x.round() as i32;
    let y2 = rect.max.y.round() as i32;
    let thickness = thickness.max(1);

    for y in y1.max(0)..=y2.min(height - 1) {
        for x in x1.max(0)..=x2.min(width - 1) {
            let on_border = x - x1 < thickness
                || x2 - x < thickness
                || y - y1 < thickness
                || y2 - y < thickness;
            if !on_border {
                continue;
            }
            let index = ((y * width + x) * 4) as usize;
            if index + 3 < pixels.len() {
                blend_pixel_over(&mut pixels[index..index + 4], color);
            }
        }
    }
}

pub(super) fn draw_brush_rect(
    pixels: &mut [u8],
    size_px: (u32, u32),
    rect: ViewportRect,
    fill_color: Color,
    outline_color: Color,
) {
    draw_rect(pixels, size_px, rect, fill_color);
    draw_rect_outline(pixels, size_px, rect, outline_color, 2);
}

pub(super) fn draw_tooltip_overlay(pixels: &mut [u8], size_px: (u32, u32), tooltip: &TooltipState) {
    const TOOLTIP_FONT_SIZE: f32 = 13.0;
    const TOOLTIP_PADDING_X: f64 = 8.0;
    const TOOLTIP_PADDING_Y: f64 = 6.0;
    const TOOLTIP_CURSOR_GAP: f64 = 12.0;

    let text_renderer = TextRenderer::new();
    let font = FontConfig::new(FontFamily::SansSerif, TOOLTIP_FONT_SIZE);
    let (text_width, text_height) = text_renderer
        .measure_text(&tooltip.content, &font)
        .unwrap_or_else(|_| {
            (
                tooltip.content.chars().count() as f32 * TOOLTIP_FONT_SIZE * 0.6,
                TOOLTIP_FONT_SIZE * 1.2,
            )
        });

    let tooltip_width = f64::from(text_width) + TOOLTIP_PADDING_X * 2.0;
    let tooltip_height = f64::from(text_height) + TOOLTIP_PADDING_Y * 2.0;
    let view_width = size_px.0 as f64;
    let view_height = size_px.1 as f64;
    let max_left = (view_width - tooltip_width).max(0.0);
    let max_top = (view_height - tooltip_height).max(0.0);

    let mut left = tooltip.position_px.x + TOOLTIP_CURSOR_GAP;
    if left + tooltip_width > view_width {
        left = tooltip.position_px.x - tooltip_width - TOOLTIP_CURSOR_GAP;
    }
    let mut top = tooltip.position_px.y - tooltip_height - TOOLTIP_CURSOR_GAP;
    if top < 0.0 {
        top = tooltip.position_px.y + TOOLTIP_CURSOR_GAP;
    }

    left = left.clamp(0.0, max_left);
    top = top.clamp(0.0, max_top);

    let rect = ViewportRect {
        min: ViewportPoint::new(left, top),
        max: ViewportPoint::new(left + tooltip_width, top + tooltip_height),
    };
    draw_rect(pixels, size_px, rect, Color::new_rgba(255, 255, 220, 220));

    let Some(size) = tiny_skia::IntSize::from_wh(size_px.0, size_px.1) else {
        log::debug!("Skipping tooltip text render because overlay size is invalid");
        return;
    };
    let Some(mut pixmap) = tiny_skia::Pixmap::from_vec(pixels.to_vec(), size) else {
        log::debug!("Skipping tooltip text render because tooltip pixmap creation failed");
        return;
    };

    if let Err(err) = text_renderer.render_text(
        &mut pixmap,
        &tooltip.content,
        (left + TOOLTIP_PADDING_X) as f32,
        (top + TOOLTIP_PADDING_Y) as f32,
        &font,
        Color::new_rgba(24, 24, 24, 255),
    ) {
        log::debug!("Skipping tooltip text render after text rasterization failed: {err}");
        return;
    }

    let rendered = pixmap.take();
    pixels.copy_from_slice(&rendered);
}

pub(super) fn tooltip_from_hit(hit: &HitResult) -> TooltipState {
    match hit {
        HitResult::SeriesPoint {
            screen_position,
            data_position,
            ..
        } => TooltipState {
            content: format!("x={:.3}, y={:.3}", data_position.x, data_position.y),
            position_px: *screen_position,
        },
        HitResult::HeatmapCell {
            screen_rect,
            row,
            col,
            value,
            ..
        } => TooltipState {
            content: format!("row={}, col={}, value={:.3}", row, col, value),
            position_px: screen_rect.max,
        },
        HitResult::None => TooltipState {
            content: String::new(),
            position_px: ViewportPoint::default(),
        },
    }
}
