use glam::{Vec2, Vec3};

use crate::axes::{format_tick_labels, generate_ticks};
use crate::core::legend::{
    Legend, LegendItem, LegendPlacement, LegendPosition, LegendStyle, estimated_label_width,
    layout_legend, measure_legend_size,
};
use crate::core::{PlottingError, Result};
use crate::render::{Color, ColorMap, LineStyle, MarkerStyle};

use super::builder::Series3D;
use super::resolve::ResolvedFrame3D;
use super::types::PreparedCamera3D;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Viewport3D {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl Viewport3D {
    pub(crate) fn right(self) -> f32 {
        self.x.saturating_add(self.width) as f32
    }

    pub(crate) fn bottom(self) -> f32 {
        self.y.saturating_add(self.height) as f32
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ScreenPoint3D {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) depth: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OverlayLine3D {
    pub(crate) start: Vec2,
    pub(crate) end: Vec2,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct OverlayText3D {
    pub(crate) text: String,
    pub(crate) position: Vec2,
    pub(crate) centered: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OverlayRect3D {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

impl OverlayRect3D {
    pub(crate) fn right(self) -> f32 {
        self.x + self.width
    }

    pub(crate) fn bottom(self) -> f32 {
        self.y + self.height
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LegendGlyph3D {
    Marker,
    Line,
    Fill,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LegendItem3D {
    pub(crate) glyph: LegendGlyph3D,
    pub(crate) color: Color,
    pub(crate) glyph_rect: OverlayRect3D,
    pub(crate) label: OverlayText3D,
}

/// A laid-out 3D legend: the frame, and everything inside it.
///
/// Every value here comes from the resolved [`Legend`] via `layout_legend`, so
/// the overlay paints what the layout measured. Nothing downstream may go back
/// to the theme for a font size or a colour — that split is exactly how the
/// frame used to be sized for one font size and the label drawn at another.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Legend3D {
    pub(crate) bounds: OverlayRect3D,
    /// Label and title size in device pixels, from [`Legend::font_size`].
    pub(crate) font_size: f32,
    /// Label and title colour, from [`Legend::text_color`].
    pub(crate) text_color: Color,
    /// Frame paint, from [`Legend::style`], already scaled to device pixels.
    pub(crate) style: LegendStyle,
    /// Centred title, from [`Legend::title`].
    pub(crate) title: Option<OverlayText3D>,
    pub(crate) items: Vec<LegendItem3D>,
}

#[derive(Clone, Debug)]
pub(crate) struct Colorbar3D {
    pub(crate) bounds: OverlayRect3D,
    pub(crate) colormap: ColorMap,
    pub(crate) data_range: (f64, f64),
    pub(crate) tick_marks: Vec<OverlayLine3D>,
    pub(crate) tick_labels: Vec<OverlayText3D>,
}

#[derive(Clone, Debug)]
pub(crate) struct Axis3Layout {
    pub(crate) canvas_width: u32,
    pub(crate) canvas_height: u32,
    pub(crate) viewport: Viewport3D,
    pub(crate) camera: PreparedCamera3D,
    pub(crate) panes: Vec<[Vec2; 4]>,
    pub(crate) grid_lines: Vec<OverlayLine3D>,
    pub(crate) box_edges: Vec<OverlayLine3D>,
    pub(crate) tick_marks: Vec<OverlayLine3D>,
    pub(crate) tick_labels: Vec<OverlayText3D>,
    pub(crate) axis_labels: Vec<OverlayText3D>,
    pub(crate) title: Option<OverlayText3D>,
    pub(crate) legend: Option<Legend3D>,
    pub(crate) colorbars: Vec<Colorbar3D>,
}

impl Axis3Layout {
    pub(crate) fn resolve(frame: &ResolvedFrame3D) -> Result<Self> {
        let (canvas_width, canvas_height) = frame.figure.canvas_size();
        if canvas_width == 0 || canvas_height == 0 {
            return Err(PlottingError::InvalidDimensions {
                width: canvas_width,
                height: canvas_height,
            });
        }

        let decorations = decoration_sources(frame);
        let decoration_width = decoration_band_width(frame, &decorations, canvas_width)?;
        let viewport = axis_viewport(frame, canvas_width, canvas_height, decoration_width);
        let camera = frame
            .camera
            .prepare(viewport.width as f32 / viewport.height as f32, frame.bounds)?;
        let corners = projected_box_corners(camera, viewport)?;
        let anchor_index = outer_anchor_corner(&corners);
        let anchor_signs = corner_signs(anchor_index);
        let center = project_local(Vec3::ZERO, camera, viewport)?;
        let line_scale = frame.figure.dpi / 72.0;

        let mut panes = Vec::with_capacity(3);
        for axis in 0..3 {
            let indices = face_corner_indices(anchor_index, axis);
            panes.push(
                [
                    corners[indices[0]],
                    corners[indices[1]],
                    corners[indices[2]],
                    corners[indices[3]],
                ]
                .map(|point| Vec2::new(point.x, point.y)),
            );
        }

        let edge_indices = [
            [0, 1],
            [0, 2],
            [0, 4],
            [1, 3],
            [1, 5],
            [2, 3],
            [2, 6],
            [3, 7],
            [4, 5],
            [4, 6],
            [5, 7],
            [6, 7],
        ];
        let box_edges = edge_indices
            .into_iter()
            .map(|[start, end]| OverlayLine3D {
                start: Vec2::new(corners[start].x, corners[start].y),
                end: Vec2::new(corners[end].x, corners[end].y),
            })
            .collect();

        let ranges = [
            (frame.bounds.min.x, frame.bounds.max.x),
            (frame.bounds.min.y, frame.bounds.max.y),
            (frame.bounds.min.z, frame.bounds.max.z),
        ];
        let labels = [
            frame.xlabel.as_deref(),
            frame.ylabel.as_deref(),
            frame.zlabel.as_deref(),
        ];
        let mut grid_lines = Vec::new();
        let mut tick_marks = Vec::new();
        let mut tick_labels = Vec::new();
        let mut axis_labels = Vec::new();

        for axis in 0..3 {
            let mut tick_values = generate_ticks(ranges[axis].0, ranges[axis].1, 6);
            tick_values.dedup_by(|left, right| left.to_bits() == right.to_bits());
            let formatted = format_tick_labels(&tick_values);
            let axis_start = local_corner(anchor_signs);
            let mut axis_end = axis_start;
            axis_end[axis] = -axis_end[axis];
            let projected_start = project_local(axis_start, camera, viewport)?;
            let projected_end = project_local(axis_end, camera, viewport)?;
            let edge_midpoint = Vec2::new(
                (projected_start.x + projected_end.x) * 0.5,
                (projected_start.y + projected_end.y) * 0.5,
            );
            let edge_outward = outward_direction(
                edge_midpoint,
                Vec2::new(center.x, center.y),
                Vec2::new(
                    projected_end.x - projected_start.x,
                    projected_end.y - projected_start.y,
                ),
            );

            for (&value, text) in tick_values.iter().zip(formatted) {
                let parameter = normalized_tick(value, ranges[axis]);
                let mut local = axis_start;
                local[axis] = parameter;
                let projected = project_local(local, camera, viewport)?;
                let position = Vec2::new(projected.x, projected.y);
                let outward = outward_direction(
                    position,
                    Vec2::new(center.x, center.y),
                    Vec2::new(
                        projected_end.x - projected_start.x,
                        projected_end.y - projected_start.y,
                    ),
                );
                tick_marks.push(OverlayLine3D {
                    start: position,
                    end: position + outward * (4.0 * line_scale),
                });
                let candidate = OverlayText3D {
                    text,
                    position: position + outward * (9.0 * line_scale),
                    centered: true,
                };
                push_text_avoiding_overlap(
                    &mut tick_labels,
                    candidate,
                    outward,
                    frame.theme.tick_label_font_size * line_scale,
                );

                for other_axis in 0..3 {
                    if other_axis == axis {
                        continue;
                    }
                    let mut grid_end = local;
                    grid_end[other_axis] = -grid_end[other_axis];
                    let projected_grid_end = project_local(grid_end, camera, viewport)?;
                    grid_lines.push(OverlayLine3D {
                        start: position,
                        end: Vec2::new(projected_grid_end.x, projected_grid_end.y),
                    });
                }
            }

            if let Some(label) = labels[axis].filter(|label| !label.is_empty()) {
                axis_labels.push(OverlayText3D {
                    text: label.to_string(),
                    position: edge_midpoint + edge_outward * (28.0 * line_scale),
                    centered: true,
                });
            }
        }

        let title = frame
            .title
            .as_ref()
            .filter(|title| !title.is_empty())
            .map(|title| OverlayText3D {
                text: title.clone(),
                position: Vec2::new(canvas_width as f32 * 0.5, 8.0 * line_scale),
                centered: true,
            });
        let (legend, colorbars) = resolve_decorations(frame, viewport, canvas_width, &decorations)?;

        Ok(Self {
            canvas_width,
            canvas_height,
            viewport,
            camera,
            panes,
            grid_lines,
            box_edges,
            tick_marks,
            tick_labels,
            axis_labels,
            title,
            legend,
            colorbars,
        })
    }

    pub(crate) fn project_local(&self, local: Vec3) -> Result<ScreenPoint3D> {
        project_local(local, self.camera, self.viewport)
    }

    pub(crate) fn screen_ray_local(
        &self,
        screen_x: f32,
        screen_y: f32,
    ) -> Result<Option<(Vec3, Vec3)>> {
        if !screen_x.is_finite() || !screen_y.is_finite() {
            return Err(PlottingError::InvalidInput(
                "3D pick coordinates must be finite".to_string(),
            ));
        }
        let viewport = self.viewport;
        if screen_x < viewport.x as f32
            || screen_x >= viewport.right()
            || screen_y < viewport.y as f32
            || screen_y >= viewport.bottom()
        {
            return Ok(None);
        }
        let ndc_x = (screen_x - viewport.x as f32) / viewport.width as f32 * 2.0 - 1.0;
        let ndc_y = 1.0 - (screen_y - viewport.y as f32) / viewport.height as f32 * 2.0;
        let unproject = |depth| -> Result<Vec3> {
            let homogeneous =
                self.camera.inverse_view_projection * Vec3::new(ndc_x, ndc_y, depth).extend(1.0);
            if !homogeneous.is_finite() || homogeneous.w.abs() <= f32::EPSILON {
                return Err(PlottingError::InvalidTopology3D {
                    reason: "3D pick unprojection produced an invalid divisor".to_string(),
                });
            }
            Ok((homogeneous.truncate() / homogeneous.w) / self.camera.axis_aspect)
        };
        let origin = unproject(0.0)?;
        let far = unproject(1.0)?;
        let direction = far - origin;
        if !direction.is_finite() || direction.length_squared() <= f32::EPSILON {
            return Err(PlottingError::InvalidTopology3D {
                reason: "3D pick unprojection produced a degenerate ray".to_string(),
            });
        }
        Ok(Some((origin, direction.normalize())))
    }

    pub(crate) fn unproject_local_at_depth(
        &self,
        screen_x: f32,
        screen_y: f32,
        depth: f32,
    ) -> Result<Vec3> {
        if !screen_x.is_finite()
            || !screen_y.is_finite()
            || !depth.is_finite()
            || !(0.0..=1.0).contains(&depth)
        {
            return Err(PlottingError::InvalidInput(
                "3D unprojection requires finite screen coordinates and depth in 0..=1".to_string(),
            ));
        }
        let ndc_x = (screen_x - self.viewport.x as f32) / self.viewport.width as f32 * 2.0 - 1.0;
        let ndc_y = 1.0 - (screen_y - self.viewport.y as f32) / self.viewport.height as f32 * 2.0;
        let homogeneous =
            self.camera.inverse_view_projection * Vec3::new(ndc_x, ndc_y, depth).extend(1.0);
        if !homogeneous.is_finite() || homogeneous.w.abs() <= f32::EPSILON {
            return Err(PlottingError::InvalidTopology3D {
                reason: "3D unprojection produced an invalid homogeneous divisor".to_string(),
            });
        }
        Ok((homogeneous.truncate() / homogeneous.w) / self.camera.axis_aspect)
    }
}

#[derive(Clone, Debug)]
struct DecorationSources3D {
    legend: Vec<LegendSource3D>,
    colorbars: Vec<ColorbarSource3D>,
}

#[derive(Clone, Debug)]
struct LegendSource3D {
    label: String,
    color: Color,
    glyph: LegendGlyph3D,
}

#[derive(Clone, Debug)]
struct ColorbarSource3D {
    colormap: ColorMap,
    data_range: (f64, f64),
}

fn decoration_sources(frame: &ResolvedFrame3D) -> DecorationSources3D {
    let mut legend = Vec::new();
    let mut colorbars = Vec::new();
    for (series_index, series) in frame.series.iter().enumerate() {
        match series {
            Series3D::Scatter { config, label, .. } => {
                push_legend_source(
                    &mut legend,
                    label,
                    config
                        .color
                        .unwrap_or_else(|| palette_color(frame, series_index)),
                    LegendGlyph3D::Marker,
                );
            }
            Series3D::Line { config, label, .. } => {
                push_legend_source(
                    &mut legend,
                    label,
                    config
                        .color
                        .unwrap_or_else(|| palette_color(frame, series_index)),
                    LegendGlyph3D::Line,
                );
            }
            Series3D::Surface {
                data,
                config,
                label,
            } => {
                let legend_color = config.color.unwrap_or_else(|| config.colormap.sample(0.5));
                push_legend_source(&mut legend, label, legend_color, LegendGlyph3D::Fill);
                if config.colorbar {
                    let colormap = config.color.map_or_else(
                        || config.colormap.clone(),
                        |color| ColorMap::new("solid 3d surface".to_string(), vec![color]),
                    );
                    if let Some(data_range) = finite_range(&data.z) {
                        colorbars.push(ColorbarSource3D {
                            colormap,
                            data_range,
                        });
                    }
                }
            }
            Series3D::Wireframe { config, label, .. } => {
                push_legend_source(
                    &mut legend,
                    label,
                    config.color.unwrap_or(frame.theme.foreground),
                    LegendGlyph3D::Line,
                );
            }
        }
    }
    DecorationSources3D { legend, colorbars }
}

fn push_legend_source(
    output: &mut Vec<LegendSource3D>,
    label: &Option<String>,
    color: Color,
    glyph: LegendGlyph3D,
) {
    if let Some(label) = label.as_ref().filter(|label| !label.is_empty()) {
        output.push(LegendSource3D {
            label: label.clone(),
            color,
            glyph,
        });
    }
}

fn finite_range(values: &[f64]) -> Option<(f64, f64)> {
    let mut minimum = f64::INFINITY;
    let mut maximum = f64::NEG_INFINITY;
    for &value in values {
        if value.is_finite() {
            minimum = minimum.min(value);
            maximum = maximum.max(value);
        }
    }
    (minimum.is_finite() && maximum.is_finite()).then_some((minimum, maximum))
}

fn palette_color(frame: &ResolvedFrame3D, series_index: usize) -> Color {
    if frame.theme.color_palette.is_empty() {
        frame.theme.foreground
    } else {
        frame.theme.color_palette[series_index % frame.theme.color_palette.len()]
    }
}

/// The legend configuration a 3D frame is drawn with, in pixels.
///
/// `Plot3D::legend` is honoured verbatim when the user set one; otherwise the
/// legend is derived from the theme. Either way it is the same [`Legend`] the
/// 2D API takes, with the same
/// [`LegendSpacing`](crate::core::LegendSpacing) and
/// [`LegendStyle`](crate::core::LegendStyle), scaled to device pixels by the
/// same `Legend::scaled_for_render` the 2D backends use, and it goes through
/// the same `layout_legend` — so there is one legend layout in the crate, not a
/// 3D look-alike with its own hardcoded padding.
fn legend_config(frame: &ResolvedFrame3D) -> Legend {
    let scale = frame.figure.render_scale();
    let configured = frame.legend.clone().unwrap_or_else(|| Legend {
        enabled: true,
        // No user legend: the theme's legend lives in the decoration band
        // beside the plotting box, which is what 3D has always drawn.
        position: LegendPosition::OutsideRight,
        font_size: frame.theme.legend_font_size,
        text_color: frame.theme.foreground,
        style: LegendStyle {
            visible: true,
            // Opaque, square, hairline: the themed 3D frame, expressed as a
            // `LegendStyle` so the overlay has one frame painter, not two.
            alpha: 1.0,
            face_color: frame.theme.background,
            edge_color: Some(frame.theme.grid_color),
            border_width: scale.pixels_to_points(1.0),
            fancy_box: false,
            corner_radius: 0.0,
            shadow: false,
            ..LegendStyle::default()
        },
        ..Legend::default()
    });
    configured.scaled_for_render(scale)
}

/// Whether this legend is drawn in the decoration band beside the plotting box.
///
/// The 3D decoration band is on the right, and it is also where colorbars go,
/// so every *outside* position resolves to it; every inside position (and
/// [`LegendPosition::Best`], which resolves to one) is laid out within the
/// plotting viewport exactly as it would be in 2D.
fn legend_uses_decoration_band(position: LegendPosition) -> bool {
    position.is_outside()
}

/// The legend to draw for this frame, or `None` when there is nothing to draw.
///
/// One gate for both the band reservation and the legend box drawn inside it,
/// so the figure cannot reserve room for a legend it then declines to draw.
fn resolved_legend(
    frame: &ResolvedFrame3D,
    sources: &[LegendSource3D],
) -> Option<(Legend, Vec<LegendItem>)> {
    let config = legend_config(frame);
    if sources.is_empty() || !config.enabled {
        return None;
    }
    Some((config, legend_items(sources)))
}

/// The shared legend items behind the 3D glyph list.
///
/// The glyph kind stays on [`LegendSource3D`] because the 3D overlay draws its
/// own keys; these items exist so the *geometry* comes from `layout_legend`.
fn legend_items(sources: &[LegendSource3D]) -> Vec<LegendItem> {
    sources.iter().map(legend_item).collect()
}

fn legend_item(source: &LegendSource3D) -> LegendItem {
    let label = source.label.clone();
    let color = source.color;
    match source.glyph {
        LegendGlyph3D::Line => LegendItem::line(label, color, LineStyle::Solid, 1.5),
        LegendGlyph3D::Marker => LegendItem::scatter(label, color, MarkerStyle::Square, 6.0),
        LegendGlyph3D::Fill => LegendItem::bar(label, color),
    }
}

/// Estimate a label's width for the 3D layout.
///
/// `Axis3Layout::resolve` runs before any renderer exists, so there is no text
/// engine to ask; [`estimated_label_width`] is the shared fallback, and it
/// counts wide glyphs double so a CJK legend is not sized for half its text.
fn estimate_3d_label(text: &str, font_size: f32) -> Result<f32> {
    Ok(estimated_label_width(text, font_size))
}

fn decoration_band_width(
    frame: &ResolvedFrame3D,
    decorations: &DecorationSources3D,
    canvas_width: u32,
) -> Result<f32> {
    let dpi_scale = frame.figure.dpi / 72.0;
    // Reserve exactly what `resolve_decorations` will lay out below — the band
    // and the legend inside it are sized by one call, not two formulas.
    let legend_width = match resolved_legend(frame, &decorations.legend) {
        // A legend placed inside the plotting box needs no band; reserving one
        // for it would shrink the very viewport it is drawn over.
        Some((config, keys)) if legend_uses_decoration_band(config.position) => {
            let (width, _) = measure_legend_size(&keys, &config, |text| {
                estimate_3d_label(text, config.font_size)
            })?;
            width
        }
        _ => 0.0,
    };
    let colorbar_width = if decorations.colorbars.is_empty() {
        0.0
    } else {
        76.0 * dpi_scale
    };
    if legend_width <= 0.0 && colorbar_width <= 0.0 {
        return Ok(0.0);
    }
    let maximum = (canvas_width as f32 * 0.36).max(1.0);
    let minimum = (70.0 * dpi_scale).min(maximum);
    Ok((legend_width.max(colorbar_width) + 14.0 * dpi_scale).clamp(minimum, maximum))
}

/// Run one legend through the shared layout and keep everything it computed.
///
/// The frame, the font size, the colours and every glyph position leave this
/// function together, so the overlay never has to re-derive any of them.
fn build_legend_3d(
    config: &Legend,
    keys: &[LegendItem],
    sources: &[LegendSource3D],
    plot_area: (f32, f32, f32, f32),
    placement: LegendPlacement<'_>,
) -> Result<Legend3D> {
    let layout = layout_legend(keys, config, plot_area, placement, |text| {
        estimate_3d_label(text, config.font_size)
    })?;
    let items = layout
        .entries
        .iter()
        .map(|entry| {
            let source = &sources[entry.item_index];
            LegendItem3D {
                glyph: source.glyph,
                color: source.color,
                glyph_rect: OverlayRect3D {
                    x: entry.handle_x,
                    y: entry.handle_center_y - layout.spacing.handle_height * 0.5,
                    width: layout.spacing.handle_length,
                    height: layout.spacing.handle_height,
                },
                label: OverlayText3D {
                    text: source.label.clone(),
                    position: Vec2::new(entry.label_x, entry.handle_center_y),
                    centered: false,
                },
            }
        })
        .collect();
    let title = layout
        .title
        .zip(config.title.as_deref())
        .map(|(title, text)| OverlayText3D {
            text: text.to_string(),
            // `OverlayText3D` positions text by its vertical centre; the shared
            // layout hands back the top of the box.
            position: Vec2::new(title.center_x, title.top_y + layout.font_size * 0.5),
            centered: true,
        });
    Ok(Legend3D {
        bounds: OverlayRect3D {
            x: layout.x,
            y: layout.y,
            width: layout.width,
            height: layout.height,
        },
        font_size: layout.font_size,
        text_color: config.text_color,
        style: config.style.clone(),
        title,
        items,
    })
}

fn resolve_decorations(
    frame: &ResolvedFrame3D,
    viewport: Viewport3D,
    canvas_width: u32,
    sources: &DecorationSources3D,
) -> Result<(Option<Legend3D>, Vec<Colorbar3D>)> {
    let dpi_scale = frame.figure.dpi / 72.0;
    let band_x = (viewport.right() + 10.0 * dpi_scale).min(canvas_width.saturating_sub(1) as f32);
    let band_right = canvas_width as f32 - 6.0 * dpi_scale;
    let band_width = (band_right - band_x)
        .max(1.0)
        .min(canvas_width as f32 - band_x);
    let mut band_legend_bottom = None;
    let legend = match resolved_legend(frame, &sources.legend) {
        Some((config, keys)) if legend_uses_decoration_band(config.position) => {
            let (natural_width, height) = measure_legend_size(&keys, &config, |text| {
                estimate_3d_label(text, config.font_size)
            })?;
            // Reserve a rectangle anchored to the right of the canvas, then
            // fill it — the same reserve-then-draw path the 2D outside legend
            // takes. It is normally exactly the band; when the legend needs
            // more than `decoration_band_width` was allowed to give it, it
            // grows *leftwards* over the plotting box rather than painting its
            // label off the edge of the canvas. Overlapping the scene is
            // recoverable; clipped text is not.
            let width = natural_width.max(band_width).min(band_right.max(1.0));
            let reserved = (
                (band_right - width).max(0.0),
                viewport.y as f32,
                band_right,
                viewport.y as f32 + height,
            );
            let legend = build_legend_3d(
                &config,
                &keys,
                &sources.legend,
                reserved,
                LegendPlacement {
                    reserved: Some(reserved),
                    occupancy: None,
                },
            )?;
            // Only a banded legend pushes the colorbars down; one drawn inside
            // the plotting box shares no space with them.
            band_legend_bottom = Some(legend.bounds.bottom());
            Some(legend)
        }
        // Every inside position — and `Best`, which resolves to one — is placed
        // within the plotting viewport, exactly as the 2D layout would.
        Some((config, keys)) => Some(build_legend_3d(
            &config,
            &keys,
            &sources.legend,
            (
                viewport.x as f32,
                viewport.y as f32,
                viewport.right(),
                viewport.bottom(),
            ),
            LegendPlacement::default(),
        )?),
        None => None,
    };

    let colorbar_top =
        band_legend_bottom.map_or(viewport.y as f32, |bottom| bottom + 12.0 * dpi_scale);
    let colorbar_bottom = viewport.bottom();
    let colorbar_count = sources.colorbars.len();
    let colorbar_gap = 10.0 * dpi_scale;
    let total_gap = colorbar_gap * colorbar_count.saturating_sub(1) as f32;
    let colorbar_height =
        ((colorbar_bottom - colorbar_top - total_gap) / colorbar_count.max(1) as f32).max(1.0);
    let bar_width = (14.0 * dpi_scale).min((band_width * 0.28).max(1.0));
    let mut colorbars = Vec::with_capacity(colorbar_count);
    for (index, source) in sources.colorbars.iter().enumerate() {
        let bounds = OverlayRect3D {
            x: band_x,
            y: colorbar_top + index as f32 * (colorbar_height + colorbar_gap),
            width: bar_width,
            height: colorbar_height,
        };
        let tick_values = colorbar_tick_values(source.data_range);
        let tick_text = format_tick_labels(&tick_values);
        let mut tick_marks = Vec::with_capacity(tick_values.len());
        let mut tick_labels = Vec::with_capacity(tick_values.len());
        for (&value, text) in tick_values.iter().zip(tick_text) {
            let normalized = normalized_colorbar_value(value, source.data_range);
            let y = bounds.y + bounds.height * (1.0 - normalized);
            tick_marks.push(OverlayLine3D {
                start: Vec2::new(bounds.right(), y),
                end: Vec2::new(bounds.right() + 4.0 * dpi_scale, y),
            });
            tick_labels.push(OverlayText3D {
                text,
                position: Vec2::new(bounds.right() + 7.0 * dpi_scale, y),
                centered: false,
            });
        }
        colorbars.push(Colorbar3D {
            bounds,
            colormap: source.colormap.clone(),
            data_range: source.data_range,
            tick_marks,
            tick_labels,
        });
    }
    Ok((legend, colorbars))
}

/// Target tick count handed to the shared locator.
///
/// `generate_ticks` gives up on nice numbers and rounds the raw endpoints
/// instead whenever fewer than three nice ticks land inside the range. Its step
/// is at most `2.5 * range / (target - 1)`, so a target of 9 keeps at least
/// three ticks in range for every non-degenerate span and never returns more
/// than nine.
const COLORBAR_TICK_TARGET: usize = 9;

/// Nice-number colorbar ticks, sharing the 2D locator so a 3D colorbar reads
/// `0.2 / 0 / -0.2` instead of the raw data endpoints.
fn colorbar_tick_values(range: (f64, f64)) -> Vec<f64> {
    if range.0.to_bits() == range.1.to_bits() {
        return vec![range.0];
    }
    let (min, max) = if range.0 <= range.1 {
        (range.0, range.1)
    } else {
        (range.1, range.0)
    };
    // `generate_ticks` normally drops ticks outside `[min, max]`, but its
    // degenerate-range escape hatches return the raw endpoints, so filter again —
    // a tick beyond the range would be drawn off the end of the bar.
    let ticks: Vec<f64> = generate_ticks(min, max, COLORBAR_TICK_TARGET)
        .into_iter()
        .filter(|value| value.is_finite() && *value >= min && *value <= max)
        .collect();
    if ticks.len() < 2 {
        vec![min, min * 0.5 + max * 0.5, max]
    } else {
        ticks
    }
}

fn normalized_colorbar_value(value: f64, range: (f64, f64)) -> f32 {
    if range.0.to_bits() == range.1.to_bits() {
        0.5
    } else {
        ((value - range.0) / (range.1 - range.0)).clamp(0.0, 1.0) as f32
    }
}

fn axis_viewport(
    frame: &ResolvedFrame3D,
    canvas_width: u32,
    canvas_height: u32,
    decoration_width: f32,
) -> Viewport3D {
    let width = canvas_width as f32;
    let height = canvas_height as f32;
    let dpi_scale = frame.figure.dpi / 72.0;
    let left = (width * 0.14).max(42.0 * dpi_scale);
    let right = (width * 0.10).max(24.0 * dpi_scale).max(decoration_width);
    let top = if frame.title.is_some() {
        (height * 0.14).max(36.0 * dpi_scale)
    } else {
        (height * 0.09).max(18.0 * dpi_scale)
    };
    let bottom = (height * 0.16).max(42.0 * dpi_scale);

    let x = left.floor().clamp(0.0, width - 1.0) as u32;
    let y = top.floor().clamp(0.0, height - 1.0) as u32;
    let viewport_width = (width - left - right).floor().max(1.0) as u32;
    let viewport_height = (height - top - bottom).floor().max(1.0) as u32;
    Viewport3D {
        x,
        y,
        width: viewport_width.min(canvas_width.saturating_sub(x).max(1)),
        height: viewport_height.min(canvas_height.saturating_sub(y).max(1)),
    }
}

fn projected_box_corners(
    camera: PreparedCamera3D,
    viewport: Viewport3D,
) -> Result<[ScreenPoint3D; 8]> {
    let mut corners = [ScreenPoint3D {
        x: 0.0,
        y: 0.0,
        depth: 0.0,
    }; 8];
    for (index, corner) in corners.iter_mut().enumerate() {
        *corner = project_local(local_corner(corner_signs(index)), camera, viewport)?;
    }
    Ok(corners)
}

fn corner_signs(index: usize) -> [f32; 3] {
    [
        if index & 1 == 0 { -1.0 } else { 1.0 },
        if index & 2 == 0 { -1.0 } else { 1.0 },
        if index & 4 == 0 { -1.0 } else { 1.0 },
    ]
}

fn local_corner(signs: [f32; 3]) -> Vec3 {
    Vec3::from_array(signs)
}

fn outer_anchor_corner(corners: &[ScreenPoint3D; 8]) -> usize {
    let mut selected = 0;
    for index in 1..corners.len() {
        let candidate = corners[index];
        let current = corners[selected];
        if candidate.y > current.y
            || (candidate.y.to_bits() == current.y.to_bits() && candidate.x < current.x)
        {
            selected = index;
        }
    }
    selected
}

fn face_corner_indices(anchor: usize, fixed_axis: usize) -> [usize; 4] {
    let first_axis = (fixed_axis + 1) % 3;
    let second_axis = (fixed_axis + 2) % 3;
    let first = anchor ^ (1 << first_axis);
    let second = anchor ^ (1 << second_axis);
    [anchor, first, first ^ (1 << second_axis), second]
}

fn normalized_tick(value: f64, range: (f64, f64)) -> f32 {
    if range.0 == range.1 {
        0.0
    } else {
        let center = range.0 * 0.5 + range.1 * 0.5;
        let half_span = range.1 * 0.5 - range.0 * 0.5;
        ((value - center) / half_span).clamp(-1.0, 1.0) as f32
    }
}

fn outward_direction(position: Vec2, center: Vec2, edge: Vec2) -> Vec2 {
    let radial = position - center;
    let perpendicular = Vec2::new(-edge.y, edge.x);
    if perpendicular.length_squared() > 1.0e-6 {
        let perpendicular = perpendicular.normalize();
        if perpendicular.dot(radial) >= 0.0 {
            perpendicular
        } else {
            -perpendicular
        }
    } else if radial.length_squared() > 1.0e-6 {
        radial.normalize()
    } else {
        Vec2::Y
    }
}

fn push_text_avoiding_overlap(
    labels: &mut Vec<OverlayText3D>,
    mut candidate: OverlayText3D,
    outward: Vec2,
    font_size: f32,
) {
    let step = (font_size * 0.85).max(3.0);
    for _ in 0..6 {
        if labels
            .iter()
            .all(|label| !estimated_text_overlap(label, &candidate, font_size))
        {
            break;
        }
        candidate.position += outward * step;
    }
    labels.push(candidate);
}

fn estimated_text_overlap(left: &OverlayText3D, right: &OverlayText3D, font_size: f32) -> bool {
    // Same estimate the 3D legend is sized with, so a tick label and a legend
    // label of the same text can never be assumed two different widths.
    let left_half_width = estimated_label_width(&left.text, font_size) * 0.5;
    let right_half_width = estimated_label_width(&right.text, font_size) * 0.5;
    let horizontal = (left.position.x - right.position.x).abs()
        < left_half_width + right_half_width + font_size * 0.2;
    let vertical = (left.position.y - right.position.y).abs() < font_size * 0.9;
    horizontal && vertical
}

fn project_local(
    local: Vec3,
    camera: PreparedCamera3D,
    viewport: Viewport3D,
) -> Result<ScreenPoint3D> {
    let clip = camera.view_projection * (local * camera.axis_aspect).extend(1.0);
    if !clip.is_finite() || clip.w <= f32::EPSILON {
        return Err(PlottingError::InvalidTopology3D {
            reason: "Axis3 projection produced a non-finite or non-positive divisor".to_string(),
        });
    }
    let ndc = clip.truncate() / clip.w;
    Ok(ScreenPoint3D {
        x: viewport.x as f32 + (ndc.x * 0.5 + 0.5) * viewport.width as f32,
        y: viewport.y as f32 + (0.5 - ndc.y * 0.5) * viewport.height as f32,
        depth: ndc.z,
    })
}

#[cfg(test)]
mod tests {
    use crate::{scatter3d, surface};

    use super::*;

    #[test]
    fn layout_stays_inside_the_canvas_and_has_a_complete_box() {
        let frame = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
            .title("3d")
            .xlabel("x")
            .ylabel("y")
            .zlabel("z")
            .finalize()
            .resolve()
            .expect("frame");
        let layout = Axis3Layout::resolve(&frame).expect("layout");
        assert_eq!(layout.box_edges.len(), 12);
        assert_eq!(layout.panes.len(), 3);
        assert!(layout.viewport.x < layout.canvas_width);
        assert!(layout.viewport.y < layout.canvas_height);
        assert!(layout.viewport.right() <= layout.canvas_width as f32);
        assert!(layout.viewport.bottom() <= layout.canvas_height as f32);
        assert_eq!(layout.axis_labels.len(), 3);
        assert!(layout.title.is_some());
        for (index, label) in layout.tick_labels.iter().enumerate() {
            assert!(
                layout.tick_labels[index + 1..]
                    .iter()
                    .all(|other| label.position.distance(other.position) > 0.25)
            );
        }
    }

    /// The right-hand band is reserved by the same call that lays the legend
    /// out inside it, so the reservation grows with the *rendered* label, not
    /// with a character count. Six CJK glyphs are about twice as wide as six
    /// Latin ones and must claim a wider band.
    #[test]
    fn wide_script_legend_labels_reserve_a_wider_band() {
        fn viewport_width(label: &str) -> u32 {
            let frame = surface(&[0.0, 1.0], &[0.0, 1.0], &[[0.0, 1.0], [2.0, 3.0]])
                .label(label)
                .finalize()
                .resolve()
                .expect("frame");
            Axis3Layout::resolve(&frame).expect("layout").viewport.width
        }

        assert!(
            viewport_width("日本語ラベル") < viewport_width("abcdef"),
            "a CJK legend label must reserve more band than the same glyph count in Latin"
        );
    }

    /// The legend keys are laid out inside the box the band reserved for them,
    /// which is the whole point of routing 3D through the shared layout.
    #[test]
    fn legend_keys_sit_inside_the_reserved_legend_box() {
        let frame = surface(&[0.0, 1.0], &[0.0, 1.0], &[[0.0, 1.0], [2.0, 3.0]])
            .label("terrain")
            .finalize()
            .resolve()
            .expect("frame");
        let layout = Axis3Layout::resolve(&frame).expect("layout");
        let legend = layout.legend.as_ref().expect("legend");

        assert!(!legend.items.is_empty());
        for item in &legend.items {
            assert!(item.glyph_rect.x >= legend.bounds.x, "{item:?}");
            assert!(item.glyph_rect.right() <= legend.bounds.right(), "{item:?}");
            assert!(item.label.position.x > item.glyph_rect.right(), "{item:?}");
            assert!(item.label.position.y >= legend.bounds.y, "{item:?}");
            assert!(item.label.position.y <= legend.bounds.bottom(), "{item:?}");
        }
    }

    /// `Plot3D::legend` is the user's knob, and it reaches the layout.
    ///
    /// Before this landed the only 3D legend setting was `Theme::legend_font_size`
    /// and the layout hardcoded everything else, so a caller could not turn the
    /// legend off, retitle it or widen it. All three must now change the layout.
    #[test]
    fn user_legend_configuration_reaches_the_3d_layout() {
        fn layout_with(legend: Option<Legend>) -> Axis3Layout {
            let plot =
                surface(&[0.0, 1.0], &[0.0, 1.0], &[[0.0, 1.0], [2.0, 3.0]]).label("terrain");
            let plot = match legend {
                Some(legend) => plot.legend(legend),
                None => plot,
            };
            let frame = plot.finalize().resolve().expect("frame");
            Axis3Layout::resolve(&frame).expect("layout")
        }

        let default = layout_with(None);
        assert!(default.legend.is_some(), "a labelled series gets a legend");

        // `enabled: false` suppresses the legend *and* the band it reserved,
        // so the viewport grows back into the space.
        let disabled = layout_with(Some(Legend::new()));
        assert!(disabled.legend.is_none());
        assert!(disabled.viewport.width > default.viewport.width);

        // A title has to widen the reservation, not overhang the box.
        let titled = layout_with(Some(Legend {
            enabled: true,
            position: LegendPosition::OutsideRight,
            title: Some("a legend title far wider than `terrain`".to_string()),
            ..Legend::default()
        }));
        let titled_legend = titled.legend.as_ref().expect("legend");
        let default_legend = default.legend.as_ref().expect("legend");
        assert!(titled.viewport.width < default.viewport.width);
        assert!(titled_legend.bounds.width > default_legend.bounds.width);

        // A bigger font makes bigger keys.
        let large = layout_with(Some(Legend {
            enabled: true,
            position: LegendPosition::OutsideRight,
            font_size: Legend::default().font_size * 3.0,
            ..Legend::default()
        }));
        let large_legend = large.legend.as_ref().expect("legend");
        assert!(large_legend.bounds.height > default_legend.bounds.height);
    }

    #[test]
    fn degenerate_ranges_produce_finite_tick_geometry() {
        let frame = surface(&[2.0, 2.0], &[3.0, 3.0], &[[4.0, 4.0], [4.0, 4.0]])
            .finalize()
            .resolve()
            .expect("frame");
        let layout = Axis3Layout::resolve(&frame).expect("layout");
        assert!(!layout.tick_marks.is_empty());
        assert!(layout.tick_marks.iter().all(|line| {
            line.start.is_finite()
                && line.end.is_finite()
                && line.start.x >= 0.0
                && line.start.y >= 0.0
        }));
    }

    #[test]
    fn perspective_and_orthographic_layouts_are_distinct() {
        let orthographic = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
            .finalize()
            .resolve()
            .expect("orthographic");
        let perspective = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
            .perspective_deg(45.0)
            .finalize()
            .resolve()
            .expect("perspective");
        let orthographic = Axis3Layout::resolve(&orthographic).expect("layout");
        let perspective = Axis3Layout::resolve(&perspective).expect("layout");
        assert_ne!(orthographic.box_edges, perspective.box_edges);
    }

    #[test]
    fn labels_and_requested_colorbars_resolve_into_a_bounded_right_band() {
        let undecorated = surface(&[0.0, 1.0], &[0.0, 1.0], &[[0.0, 1.0], [2.0, 3.0]])
            .finalize()
            .resolve()
            .expect("undecorated");
        let decorated = surface(&[0.0, 1.0], &[0.0, 1.0], &[[0.0, 1.0], [2.0, 3.0]])
            .label("terrain")
            .colorbar(true)
            .finalize()
            .resolve()
            .expect("decorated");
        let undecorated = Axis3Layout::resolve(&undecorated).expect("undecorated layout");
        let decorated = Axis3Layout::resolve(&decorated).expect("decorated layout");

        assert!(undecorated.legend.is_none());
        assert!(undecorated.colorbars.is_empty());
        assert!(decorated.viewport.width < undecorated.viewport.width);
        let legend = decorated.legend.as_ref().expect("legend");
        assert_eq!(legend.items.len(), 1);
        assert_eq!(legend.items[0].label.text, "terrain");
        assert!(legend.bounds.right() <= decorated.canvas_width as f32);
        let colorbar = decorated.colorbars.first().expect("colorbar");
        assert_eq!(colorbar.data_range, (0.0, 3.0));
        // Nice-number ticks over [0, 3] step by 0.5 instead of printing endpoints.
        assert_eq!(colorbar.tick_labels.len(), colorbar.tick_marks.len());
        let texts: Vec<&str> = colorbar
            .tick_labels
            .iter()
            .map(|label| label.text.as_str())
            .collect();
        assert_eq!(texts, ["0", "0.5", "1", "1.5", "2", "2.5", "3"]);
        assert!(colorbar.bounds.right() <= decorated.canvas_width as f32);
        assert!(colorbar.bounds.bottom() <= decorated.canvas_height as f32);
    }

    #[test]
    fn colorbar_ticks_are_round_numbers_inside_the_data_range() {
        let range = (-0.217_057, 0.982_343);
        let ticks = colorbar_tick_values(range);
        assert!(ticks.len() >= 2, "expected at least two ticks: {ticks:?}");
        for &tick in &ticks {
            assert!(
                tick >= range.0 && tick <= range.1,
                "tick {tick} escaped the data range {range:?}"
            );
            // A "nice" tick is a small multiple of the step, so scaling by the
            // step's magnitude must land on an integer.
            let scaled = tick * 10.0;
            assert!(
                (scaled - scaled.round()).abs() < 1e-9,
                "tick {tick} is not a round number"
            );
        }
        // The raw endpoints must no longer be printed verbatim.
        let labels = format_tick_labels(&ticks);
        assert!(
            labels.iter().all(|label| label.len() <= 4),
            "expected short labels, got {labels:?}"
        );
    }

    #[test]
    fn degenerate_colorbar_range_keeps_a_single_tick() {
        assert_eq!(colorbar_tick_values((2.5, 2.5)), vec![2.5]);
    }

    #[test]
    fn reversed_colorbar_range_still_yields_in_range_ticks() {
        let range = (5.0, -5.0);
        let ticks = colorbar_tick_values(range);
        assert!(ticks.len() >= 2);
        assert!(ticks.iter().all(|&tick| (-5.0..=5.0).contains(&tick)));
    }

    #[test]
    fn local_screen_ray_passes_through_the_projected_center() {
        let frame = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
            .finalize()
            .resolve()
            .expect("frame");
        let layout = Axis3Layout::resolve(&frame).expect("layout");
        let center = layout.project_local(Vec3::ZERO).expect("center");
        let (origin, direction) = layout
            .screen_ray_local(center.x, center.y)
            .expect("ray")
            .expect("inside viewport");
        let parameter = -origin.dot(direction);
        assert!((origin + direction * parameter).length() <= 1.0e-4);
        assert!(
            layout
                .screen_ray_local(0.0, 0.0)
                .expect("outside")
                .is_none()
        );
    }

    /// The label is painted at the size its frame was measured for.
    ///
    /// The frame came from `Legend::font_size` while the label was painted at
    /// `Theme::legend_font_size`, so the two disagreed for every user font
    /// size: a small one produced a label wider than its own box (clipped by
    /// the canvas edge), a large one produced a box full of air. Both sides now
    /// read the single resolved font size, so the ink has to grow with it and
    /// stay inside the frame at every size.
    #[test]
    fn legend_label_ink_scales_with_the_user_font_size_and_stays_inside_its_frame() {
        /// `(frame bounds, ink bounds)` — ink is the magenta label text, a
        /// colour nothing else in the figure paints.
        fn measure(font_size: f32) -> (OverlayRect3D, (u32, u32, u32, u32)) {
            let legend = || Legend {
                enabled: true,
                position: LegendPosition::OutsideRight,
                font_size,
                text_color: Color::from_rgb(255, 0, 255),
                ..Legend::default()
            };
            let plot = || {
                crate::line3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
                    .size_px(700, 500)
                    .legend(legend())
                    .label("a long legend label")
            };
            let frame = plot().finalize().resolve().expect("frame");
            let bounds = Axis3Layout::resolve(&frame)
                .expect("layout")
                .legend
                .expect("legend")
                .bounds;

            let image = plot().render().expect("image");
            let (mut left, mut top, mut right, mut bottom) = (u32::MAX, u32::MAX, 0, 0);
            for y in 0..image.height {
                for x in 0..image.width {
                    let offset = ((y * image.width + x) * 4) as usize;
                    let (r, g, b) = (
                        image.pixels[offset],
                        image.pixels[offset + 1],
                        image.pixels[offset + 2],
                    );
                    if r > 150 && b > 150 && g < 100 {
                        left = left.min(x);
                        top = top.min(y);
                        right = right.max(x);
                        bottom = bottom.max(y);
                    }
                }
            }
            assert!(
                left <= right,
                "no legend label ink at font size {font_size}"
            );
            (bounds, (left, top, right, bottom))
        }

        let (small_frame, small_ink) = measure(8.0);
        let (large_frame, large_ink) = measure(22.0);

        assert!(
            large_ink.2 - large_ink.0 > small_ink.2 - small_ink.0,
            "a larger legend font must paint wider label ink: {small_ink:?} vs {large_ink:?}"
        );
        assert!(
            large_ink.3 - large_ink.1 > small_ink.3 - small_ink.1,
            "a larger legend font must paint taller label ink: {small_ink:?} vs {large_ink:?}"
        );

        for (frame, ink, font_size) in [
            (small_frame, small_ink, 8.0),
            (large_frame, large_ink, 22.0),
        ] {
            assert!(
                ink.0 as f32 >= frame.x && (ink.2 as f32) <= frame.right(),
                "label ink {ink:?} escapes its frame {frame:?} at font size {font_size}"
            );
            assert!(
                ink.1 as f32 >= frame.y && (ink.3 as f32) <= frame.bottom(),
                "label ink {ink:?} escapes its frame {frame:?} at font size {font_size}"
            );
        }
    }

    /// `Legend::position` is consulted: an inside position puts the legend
    /// inside the plotting box instead of the right-hand decoration band, and
    /// stops reserving band width the legend no longer occupies.
    #[test]
    fn inside_legend_positions_move_the_3d_legend_out_of_the_decoration_band() {
        fn layout_at(position: LegendPosition) -> Axis3Layout {
            let frame = crate::line3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
                .size_px(700, 500)
                .legend(Legend {
                    enabled: true,
                    position,
                    ..Legend::default()
                })
                .label("series")
                .finalize()
                .resolve()
                .expect("frame");
            Axis3Layout::resolve(&frame).expect("layout")
        }

        let banded = layout_at(LegendPosition::OutsideRight);
        let band_legend = banded.legend.as_ref().expect("legend");
        assert!(band_legend.bounds.x > banded.viewport.right());

        for position in [
            LegendPosition::UpperLeft,
            LegendPosition::LowerRight,
            LegendPosition::Center,
        ] {
            let inside = layout_at(position);
            let legend = inside.legend.as_ref().expect("legend");
            assert!(
                legend.bounds.x >= inside.viewport.x as f32
                    && legend.bounds.right() <= inside.viewport.right(),
                "{position:?} must place the legend inside the viewport, got {:?}",
                legend.bounds
            );
            // No band is reserved for a legend that is not in it.
            assert!(
                inside.viewport.width > banded.viewport.width,
                "{position:?} must give the plotting box the band back"
            );
        }

        // Two different inside corners must actually differ.
        let upper_left = layout_at(LegendPosition::UpperLeft);
        let lower_right = layout_at(LegendPosition::LowerRight);
        assert!(
            upper_left.legend.expect("legend").bounds.x
                < lower_right.legend.expect("legend").bounds.x
        );
    }
}
