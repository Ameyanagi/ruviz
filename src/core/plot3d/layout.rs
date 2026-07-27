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

/// Length of a tick mark, in points at 72 dpi.
const TICK_MARK_LENGTH_PT: f32 = 4.0;

/// Gap between the end of a tick mark and the near edge of its label.
const TICK_LABEL_GAP_PT: f32 = 5.0;

/// Gap between the outermost tick label and the axis label beyond it.
const AXIS_LABEL_GAP_PT: f32 = 6.0;

/// Breathing room between the outermost label and the edge of the canvas.
const LABEL_EDGE_PAD_PT: f32 = 6.0;

/// Distance from the top of the canvas to the centre of the title.
const TITLE_CENTER_PT: f32 = 8.0;

/// Largest share of the canvas one margin may claim.
///
/// A pathological label — a 40-character unit string on a small figure — must
/// not shrink the scene to nothing.
const MAX_MARGIN_FRACTION: f32 = 0.30;

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

        let line_scale = frame.figure.dpi / 72.0;
        // Ticks are located once and then used both to fit the viewport and to
        // draw the labels, so the frame always reserves room for the text it
        // actually prints.
        let ticks = axis_ticks(frame);
        let decorations = decoration_sources(frame);
        let decoration_width = decoration_band_width(frame, &decorations, canvas_width)?;
        let title = title_overlay(frame, canvas_width, line_scale);
        let limits = InkLimits3D::new(
            canvas_width,
            canvas_height,
            decoration_width,
            title_band_height(frame, line_scale),
            LABEL_EDGE_PAD_PT * line_scale,
        );
        let scene = fit_scene(frame, &ticks, &limits, canvas_width, canvas_height)?;
        let Scene3D {
            viewport,
            camera,
            panes,
            grid_lines,
            box_edges,
            tick_marks,
            tick_labels,
            axis_labels,
            ..
        } = scene;
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
}

/// Everything the axes contribute to one frame, laid out at one viewport.
///
/// Producing this is a pure function of the viewport, which is what lets the
/// viewport be *measured* rather than guessed: [`fit_scene`] lays the scene out,
/// looks at where the labels actually landed, and hands back the room they
/// turned out to need.
struct Scene3D {
    viewport: Viewport3D,
    camera: PreparedCamera3D,
    panes: Vec<[Vec2; 4]>,
    grid_lines: Vec<OverlayLine3D>,
    box_edges: Vec<OverlayLine3D>,
    tick_marks: Vec<OverlayLine3D>,
    tick_labels: Vec<OverlayText3D>,
    axis_labels: Vec<OverlayText3D>,
    /// Bounding box of every line and glyph above, in canvas pixels.
    ink: InkBox3D,
}

/// `(min_x, min_y, max_x, max_y)` of everything drawn for the axes.
#[derive(Clone, Copy, Debug)]
struct InkBox3D {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl InkBox3D {
    fn empty() -> Self {
        Self {
            min_x: f32::INFINITY,
            min_y: f32::INFINITY,
            max_x: f32::NEG_INFINITY,
            max_y: f32::NEG_INFINITY,
        }
    }

    fn add_point(&mut self, point: Vec2) {
        if !point.is_finite() {
            return;
        }
        self.min_x = self.min_x.min(point.x);
        self.min_y = self.min_y.min(point.y);
        self.max_x = self.max_x.max(point.x);
        self.max_y = self.max_y.max(point.y);
    }

    /// A centred glyph run straddles its position, so it reaches half its
    /// measured size in every direction. This is the *same* measurement the
    /// label offsets are computed from, so the fit can never reserve room for
    /// text of a different size than the overlay paints.
    fn add_centered_text(&mut self, text: &OverlayText3D, font_size: f32) {
        let half = Vec2::new(
            estimated_label_width(&text.text, font_size) * 0.5,
            font_size * 0.5,
        );
        self.add_point(text.position - half);
        self.add_point(text.position + half);
    }
}

/// Where the axes' ink is allowed to reach on each side of the canvas.
///
/// The title strip and the right-hand decoration band belong to someone else,
/// so the scene stops short of them; everywhere else it may run to within one
/// [`LABEL_EDGE_PAD_PT`] of the canvas edge.
struct InkLimits3D {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
    /// Smallest each viewport margin is allowed to be: zero, except where a
    /// structural band already occupies the space.
    floors: Margins3D,
    max_horizontal: f32,
    max_vertical: f32,
}

impl InkLimits3D {
    fn new(
        canvas_width: u32,
        canvas_height: u32,
        decoration_width: f32,
        title_band: f32,
        pad: f32,
    ) -> Self {
        let width = canvas_width as f32;
        let height = canvas_height as f32;
        Self {
            left: pad,
            right: width - decoration_width - pad,
            top: title_band,
            bottom: height - pad,
            floors: Margins3D {
                left: 0.0,
                right: decoration_width,
                top: title_band,
                bottom: 0.0,
            },
            max_horizontal: (width * MAX_MARGIN_FRACTION).max(1.0),
            max_vertical: (height * MAX_MARGIN_FRACTION).max(1.0),
        }
    }
}

/// The four viewport margins, in canvas pixels.
#[derive(Clone, Copy, Debug)]
struct Margins3D {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

impl Margins3D {
    fn viewport(self, canvas_width: u32, canvas_height: u32) -> Viewport3D {
        let width = canvas_width as f32;
        let height = canvas_height as f32;
        let x = self.left.floor().clamp(0.0, width - 1.0) as u32;
        let y = self.top.floor().clamp(0.0, height - 1.0) as u32;
        let viewport_width = (width - self.left - self.right).floor().max(1.0) as u32;
        let viewport_height = (height - self.top - self.bottom).floor().max(1.0) as u32;
        Viewport3D {
            x,
            y,
            width: viewport_width.min(canvas_width.saturating_sub(x).max(1)),
            height: viewport_height.min(canvas_height.saturating_sub(y).max(1)),
        }
    }

    /// The margins this scene turned out to need.
    ///
    /// Two things are asked of each axis at once: give the ink the room it
    /// actually overflowed by, and split whatever is left over evenly so the
    /// figure sits in the middle of its frame. Growing a margin by `d` moves the
    /// box `d/2` the other way, so a single pass halves the error and the fit
    /// converges geometrically.
    fn fitted_to(self, ink: InkBox3D, limits: &InkLimits3D) -> Self {
        let slack_left = ink.min_x - limits.left;
        let slack_right = limits.right - ink.max_x;
        let slack_top = ink.min_y - limits.top;
        let slack_bottom = limits.bottom - ink.max_y;
        // Negative total slack means the scene does not fit and must shrink;
        // unequal slack means it fits but is off-centre.
        let horizontal_deficit = -(slack_left + slack_right).min(0.0);
        let vertical_deficit = -(slack_top + slack_bottom).min(0.0);
        let horizontal_shift = (slack_right - slack_left) * 0.5;
        let vertical_shift = (slack_bottom - slack_top) * 0.5;
        Self {
            left: self.left + horizontal_deficit * 0.5 + horizontal_shift,
            right: self.right + horizontal_deficit * 0.5 - horizontal_shift,
            top: self.top + vertical_deficit * 0.5 + vertical_shift,
            bottom: self.bottom + vertical_deficit * 0.5 - vertical_shift,
        }
        .clamped(limits)
    }

    fn clamped(self, limits: &InkLimits3D) -> Self {
        let floors = limits.floors;
        let clamp = |value: f32, floor: f32, ceiling: f32| {
            if value.is_finite() {
                value.clamp(floor, ceiling.max(floor))
            } else {
                floor
            }
        };
        Self {
            left: clamp(self.left, floors.left, limits.max_horizontal),
            right: clamp(self.right, floors.right, limits.max_horizontal),
            top: clamp(self.top, floors.top, limits.max_vertical),
            bottom: clamp(self.bottom, floors.bottom, limits.max_vertical),
        }
    }

    fn is_close_to(self, other: Self) -> bool {
        const TOLERANCE: f32 = 0.5;
        (self.left - other.left).abs() < TOLERANCE
            && (self.right - other.right).abs() < TOLERANCE
            && (self.top - other.top).abs() < TOLERANCE
            && (self.bottom - other.bottom).abs() < TOLERANCE
    }
}

/// How many times the fit may re-measure itself.
///
/// Each pass halves the remaining error, so a dozen is far more than a
/// hundred-pixel initial overflow needs; the loop normally settles in three or
/// four. Laying the axes out is a few hundred matrix-vector products, which is
/// nothing beside rasterising the scene once.
const MAX_FIT_PASSES: usize = 12;

/// Lay the scene out, see where its labels landed, and repeat until they fit.
///
/// This replaces a *prediction* of the margins — the widest tick label plus the
/// widest axis label, reserved as a rectangle on all four sides — with a
/// measurement of the labels that get drawn. The prediction was a second, cruder
/// description of the offsets in [`lay_out_scene`], and it was wrong in the
/// expensive direction: a projected box is a hexagon, its tick labels sit in the
/// empty triangles at the corners of its bounding rectangle, and a rectangular
/// reservation therefore paid for the same room twice — once as viewport margin
/// and again as the slack the aspect-preserving fit already left.
fn fit_scene(
    frame: &ResolvedFrame3D,
    ticks: &[(Vec<f64>, Vec<String>); 3],
    limits: &InkLimits3D,
    canvas_width: u32,
    canvas_height: u32,
) -> Result<Scene3D> {
    let mut margins = limits.floors.clamped(limits);
    let mut scene = lay_out_scene(frame, margins.viewport(canvas_width, canvas_height), ticks)?;
    for _ in 0..MAX_FIT_PASSES {
        let next = margins.fitted_to(scene.ink, limits);
        if next.is_close_to(margins) {
            break;
        }
        margins = next;
        scene = lay_out_scene(frame, margins.viewport(canvas_width, canvas_height), ticks)?;
    }
    Ok(scene)
}

fn lay_out_scene(
    frame: &ResolvedFrame3D,
    viewport: Viewport3D,
    ticks: &[(Vec<f64>, Vec<String>); 3],
) -> Result<Scene3D> {
    let line_scale = frame.figure.dpi / 72.0;
    let camera = frame
        .camera
        .prepare(viewport.width as f32 / viewport.height as f32, frame.bounds)?;
    let corners = projected_box_corners(camera, viewport)?;
    let anchor_index = outer_anchor_corner(&corners);
    let anchor_signs = corner_signs(anchor_index);
    // x and y run along the bottom edges that meet at the anchor corner, but
    // z is vertical: anchoring it there too put its ticks and its `z` label
    // on the *front* edge of the box, inside the silhouette and on top of
    // the surface. The z axis belongs on a silhouette edge, so it takes the
    // leftmost vertical edge of the projected box instead.
    let axis_anchor_signs = [
        anchor_signs,
        anchor_signs,
        corner_signs(z_axis_anchor_corner(&corners)),
    ];
    let center = project_local(Vec3::ZERO, camera, viewport)?;

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
    let box_edges: Vec<OverlayLine3D> = edge_indices
        .into_iter()
        .map(|[start, end]| OverlayLine3D {
            start: Vec2::new(corners[start].x, corners[start].y),
            end: Vec2::new(corners[end].x, corners[end].y),
        })
        .collect();

    let ranges = axis_ranges(frame);
    let labels = [
        frame.xlabel.as_deref(),
        frame.ylabel.as_deref(),
        frame.zlabel.as_deref(),
    ];
    let tick_font_size = frame.theme.tick_label_font_size * line_scale;
    let axis_font_size = frame.theme.axis_label_font_size * line_scale;
    let tick_mark_length = TICK_MARK_LENGTH_PT * line_scale;
    let tick_label_gap = TICK_LABEL_GAP_PT * line_scale;
    let axis_label_gap = AXIS_LABEL_GAP_PT * line_scale;
    let mut grid_lines = Vec::new();
    let mut tick_marks = Vec::new();
    let mut tick_labels = Vec::new();
    let mut axis_labels = Vec::new();

    for axis in 0..3 {
        let (tick_values, formatted) = &ticks[axis];
        let axis_start = local_corner(axis_anchor_signs[axis]);
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
                end: position + outward * tick_mark_length,
            });
            // A centred label straddles the point it is placed at, so it has
            // to be pushed out by its own half-extent along the offset
            // direction or half of it lands back over the box. Sideways
            // offsets — the z axis, and x/y at some camera angles — are
            // where this shows.
            let half_extent = half_extent_along(
                outward,
                estimated_label_width(text, tick_font_size),
                tick_font_size,
            );
            let offset = tick_mark_length + tick_label_gap + half_extent;
            let candidate = OverlayText3D {
                text: text.clone(),
                position: position + outward * offset,
                centered: true,
            };
            push_text_avoiding_overlap(&mut tick_labels, candidate, outward, tick_font_size);

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
            // Clear of the tick labels rather than at a fixed 28 px: the z
            // label shares its edge with the z ticks, so a constant offset
            // either overlapped them or wasted the room it did not need.
            let widest_tick = formatted
                .iter()
                .map(|text| estimated_label_width(text, tick_font_size))
                .fold(0.0_f32, f32::max);
            let offset = tick_mark_length
                + tick_label_gap
                + 2.0 * half_extent_along(edge_outward, widest_tick, tick_font_size)
                + axis_label_gap
                + half_extent_along(
                    edge_outward,
                    estimated_label_width(label, axis_font_size),
                    axis_font_size,
                );
            axis_labels.push(OverlayText3D {
                text: label.to_string(),
                position: edge_midpoint + edge_outward * offset,
                centered: true,
            });
        }
    }

    let mut ink = InkBox3D::empty();
    for edge in box_edges.iter().chain(&tick_marks) {
        ink.add_point(edge.start);
        ink.add_point(edge.end);
    }
    for label in &tick_labels {
        ink.add_centered_text(label, tick_font_size);
    }
    for label in &axis_labels {
        ink.add_centered_text(label, axis_font_size);
    }

    Ok(Scene3D {
        viewport,
        camera,
        panes,
        grid_lines,
        box_edges,
        tick_marks,
        tick_labels,
        axis_labels,
        ink,
    })
}

/// The title, positioned in the strip [`title_band_height`] reserves for it.
///
/// The overlay draws what this returns and the fit reserves what
/// [`title_band_height`] says, both from the same two numbers, so the strip
/// cannot be sized for a title drawn somewhere else.
fn title_overlay(
    frame: &ResolvedFrame3D,
    canvas_width: u32,
    line_scale: f32,
) -> Option<OverlayText3D> {
    frame
        .title
        .as_ref()
        .filter(|title| !title.is_empty())
        .map(|title| OverlayText3D {
            text: title.clone(),
            position: Vec2::new(canvas_width as f32 * 0.5, TITLE_CENTER_PT * line_scale),
            centered: true,
        })
}

/// Height of the strip at the top of the canvas the scene may not enter.
///
/// It is exactly the title's own extent plus one edge pad — measured, like every
/// other margin — rather than a share of the canvas height. With no title
/// nothing at all is drawn above the box, so the strip is just the pad.
fn title_band_height(frame: &ResolvedFrame3D, line_scale: f32) -> f32 {
    let pad = LABEL_EDGE_PAD_PT * line_scale;
    match frame.title.as_deref().filter(|title| !title.is_empty()) {
        Some(_) => {
            (TITLE_CENTER_PT * line_scale) + frame.theme.title_font_size * line_scale * 0.5 + pad
        }
        None => pad,
    }
}

impl Axis3Layout {
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

/// Data range of each axis, in axis order.
fn axis_ranges(frame: &ResolvedFrame3D) -> [(f64, f64); 3] {
    [
        (frame.bounds.min.x, frame.bounds.max.x),
        (frame.bounds.min.y, frame.bounds.max.y),
        (frame.bounds.min.z, frame.bounds.max.z),
    ]
}

/// Target tick count per 3D axis.
const AXIS_TICK_TARGET: usize = 6;

/// Tick values and their formatted labels for all three axes.
///
/// Located once per frame: the margins are reserved from these strings and the
/// labels are drawn from these strings, so the reservation can never be for
/// different text than the layout prints.
fn axis_ticks(frame: &ResolvedFrame3D) -> [(Vec<f64>, Vec<String>); 3] {
    axis_ranges(frame).map(|(min, max)| {
        let mut values = generate_ticks(min, max, AXIS_TICK_TARGET);
        values.dedup_by(|left, right| left.to_bits() == right.to_bits());
        let labels = format_tick_labels(&values);
        (values, labels)
    })
}

/// Half-extent of a centred label along `outward`.
///
/// A label centred on a point covers `width/2` either side and `height/2` above
/// and below it, so this is how far it reaches in the direction it is pushed.
fn half_extent_along(outward: Vec2, width: f32, height: f32) -> f32 {
    outward.x.abs() * width * 0.5 + outward.y.abs() * height * 0.5
}

/// Foot of the vertical box edge the z axis is drawn on.
///
/// The four vertical edges pair corners that differ only in their z sign; the
/// leftmost of them is on the silhouette, so ticks and the `z` label offset
/// outward from it land in the left margin instead of over the surface. The
/// left is chosen over the equally extremal right because the right of a 3D
/// frame is the decoration band, where the legend and colorbars live.
fn z_axis_anchor_corner(corners: &[ScreenPoint3D; 8]) -> usize {
    let mut selected = 0_usize;
    let mut selected_x = f32::INFINITY;
    for base in 0..4_usize {
        // `base` has its z bit (bit 2) clear, so it is the foot of the edge.
        let edge_x = (corners[base].x + corners[base | 4].x) * 0.5;
        if edge_x < selected_x {
            selected_x = edge_x;
            selected = base;
        }
    }
    selected
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
    use crate::core::plot3d::Camera3D;
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

    /// `(min_x, max_x, min_y, max_y)` of the projected plotting box.
    fn corner_bbox(corners: &[ScreenPoint3D; 8]) -> (f32, f32, f32, f32) {
        let mut bbox = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        for corner in corners {
            bbox.0 = bbox.0.min(corner.x);
            bbox.1 = bbox.1.max(corner.x);
            bbox.2 = bbox.2.min(corner.y);
            bbox.3 = bbox.3.max(corner.y);
        }
        bbox
    }

    fn labelled_surface_layout() -> (ResolvedFrame3D, Axis3Layout) {
        let frame = surface(&[0.0, 1.0], &[0.0, 1.0], &[[0.0, 1.0], [2.0, 3.0]])
            .xlabel("x")
            .ylabel("y")
            .zlabel("z")
            .finalize()
            .resolve()
            .expect("frame");
        let layout = Axis3Layout::resolve(&frame).expect("layout");
        (frame, layout)
    }

    /// The z ticks and the `z` label used to be drawn on the *front* vertical
    /// edge — inside the silhouette, on top of the surface — because all three
    /// axes anchored at `outer_anchor_corner`, which is right for x and y only.
    /// They now hang off the leftmost vertical edge of the projected box, so
    /// every one of them is clear of the scene.
    #[test]
    fn the_z_axis_is_labelled_outside_the_silhouette() {
        let (frame, layout) = labelled_surface_layout();
        let corners =
            projected_box_corners(layout.camera, layout.viewport).expect("projected corners");
        let (leftmost, ..) = corner_bbox(&corners);

        assert_eq!(layout.axis_labels.len(), 3);
        let z_label = &layout.axis_labels[2];
        assert_eq!(z_label.text, "z");
        assert!(
            z_label.position.x < leftmost,
            "z label at {} is inside the box (leftmost corner {leftmost})",
            z_label.position.x
        );

        // Tick labels are pushed in axis order, so the z ticks are the last run.
        let z_tick_count = axis_ticks(&frame)[2].0.len();
        assert!(z_tick_count >= 2);
        for label in layout.tick_labels.iter().rev().take(z_tick_count) {
            assert!(
                label.position.x < leftmost,
                "z tick {:?} at {} is inside the box (leftmost corner {leftmost})",
                label.text,
                label.position.x
            );
        }
    }

    /// The scene fills its frame instead of floating in it.
    ///
    /// A fixed `1.8 / zoom` orthographic half-extent plus hardcoded 14/10/14/16%
    /// margins left a 3D box occupying roughly 41% of the frame's width, where a
    /// 2D line plot fills 92%. Both ends are now measured.
    #[test]
    fn the_scene_fills_most_of_its_frame() {
        let (_, layout) = labelled_surface_layout();
        let corners =
            projected_box_corners(layout.camera, layout.viewport).expect("projected corners");
        let (min_x, max_x, min_y, max_y) = corner_bbox(&corners);

        let width_fraction = (max_x - min_x) / layout.canvas_width as f32;
        let height_fraction = (max_y - min_y) / layout.canvas_height as f32;
        assert!(
            width_fraction > 0.68,
            "3D scene only fills {width_fraction:.2} of the frame's width"
        );
        assert!(
            height_fraction > 0.88,
            "3D scene only fills {height_fraction:.2} of the frame's height"
        );

        // ... and it still fits: nothing may spill out of the canvas.
        assert!(min_x >= 0.0 && min_y >= 0.0);
        assert!(max_x <= layout.canvas_width as f32);
        assert!(max_y <= layout.canvas_height as f32);
    }

    /// `(min_x, min_y, max_x, max_y)` of everything the layout draws for the
    /// axes — the same measurement [`fit_scene`] fits against.
    fn layout_ink(frame: &ResolvedFrame3D, layout: &Axis3Layout) -> InkBox3D {
        let line_scale = frame.figure.dpi / 72.0;
        let mut ink = InkBox3D::empty();
        for edge in layout.box_edges.iter().chain(&layout.tick_marks) {
            ink.add_point(edge.start);
            ink.add_point(edge.end);
        }
        for label in &layout.tick_labels {
            ink.add_centered_text(label, frame.theme.tick_label_font_size * line_scale);
        }
        for label in &layout.axis_labels {
            ink.add_centered_text(label, frame.theme.axis_label_font_size * line_scale);
        }
        ink
    }

    /// The fit is measured, so the *labels* are what touch the frame.
    ///
    /// The old fit predicted a rectangular margin on all four sides from the
    /// widest tick string. A projected box is a hexagon whose tick labels live
    /// in the empty triangles at the corners of its bounding rectangle, so that
    /// reservation bought the same room twice — once as viewport margin, and
    /// again as the slack an aspect-preserving fit already leaves — and the
    /// scene paid for it in size. Fitting to the ink instead means the drawn
    /// figure runs to the edge of its canvas on the limiting axis, and whatever
    /// the aspect ratio leaves over is split evenly rather than piled on one
    /// side.
    #[test]
    fn the_fit_puts_the_labels_at_the_frame_edge_and_centres_what_is_left() {
        let (frame, layout) = labelled_surface_layout();
        let ink = layout_ink(&frame, &layout);
        let width = layout.canvas_width as f32;
        let height = layout.canvas_height as f32;
        let pad = LABEL_EDGE_PAD_PT * frame.figure.dpi / 72.0;

        // Nothing is clipped...
        assert!(
            ink.min_x >= 0.0 && ink.min_y >= 0.0 && ink.max_x <= width && ink.max_y <= height,
            "axis ink {ink:?} leaves the {width}x{height} canvas"
        );
        // ... and the limiting axis is used up: this surface is taller than it
        // is wide once projected, so the height is what runs out.
        let ink_height = (ink.max_y - ink.min_y) / height;
        assert!(
            ink_height > 0.93,
            "the labelled scene only fills {ink_height:.2} of the frame's height"
        );
        assert!(ink.min_y <= pad + 1.0, "ink starts at {}", ink.min_y);
        assert!(ink.max_y >= height - pad - 1.0, "ink ends at {}", ink.max_y);

        // The leftover width — the letterbox an isotropic projection of a
        // near-square silhouette necessarily leaves in a 4:3 frame — is split
        // evenly instead of being handed to one margin.
        let left_slack = ink.min_x - pad;
        let right_slack = width - pad - ink.max_x;
        assert!(
            (left_slack - right_slack).abs() < 3.0,
            "the scene is off-centre: {left_slack:.1}px of slack on the left \
             and {right_slack:.1}px on the right"
        );
    }

    /// The frame fill that is left is the camera's, not the layout's.
    ///
    /// Width fill is `height fill x silhouette aspect / canvas aspect`, and the
    /// silhouette aspect is fixed by [`AxisAspect3D`]: a literal cube projects
    /// to a near-square hexagon that cannot fill a 4:3 canvas in both
    /// directions at once, while the crate's default 4:4:3 box projects wider
    /// than it is tall. Both go through the same fit, so the difference between
    /// them is the *only* thing still standing between a 3D figure and 2D
    /// parity — and it is a camera setting, not a margin.
    #[test]
    fn the_default_box_aspect_fills_the_frame_more_than_a_literal_cube() {
        fn width_fill(aspect: crate::core::plot3d::AxisAspect3D) -> f32 {
            let frame = surface(&[0.0, 1.0], &[0.0, 1.0], &[[0.0, 1.0], [2.0, 3.0]])
                .xlabel("x")
                .ylabel("y")
                .zlabel("z")
                .camera(Camera3D::default().axis_aspect(aspect).orthographic())
                .finalize()
                .resolve()
                .expect("frame");
            let layout = Axis3Layout::resolve(&frame).expect("layout");
            let ink = layout_ink(&frame, &layout);
            (ink.max_x - ink.min_x) / layout.canvas_width as f32
        }

        let cube = width_fill(crate::core::plot3d::AxisAspect3D::Equal);
        let scientific = width_fill(crate::core::plot3d::AxisAspect3D::Auto);
        assert!(
            scientific > cube + 0.05,
            "the 4:4:3 box should fill markedly more width than a cube: \
             {scientific:.2} vs {cube:.2}"
        );
        assert!(
            scientific > 0.76,
            "the default box aspect only fills {scientific:.2} of the frame's width"
        );
    }

    /// ...and so does a perspective scene, because both fits are tight.
    ///
    /// "Tight" is the property that matters and it is exactly checkable: the
    /// projected box must *touch* its viewport on the limiting axis. The
    /// perspective frustum used to be sized from the box's circumscribed
    /// sphere, whose radius is `sqrt(3)` for a unit box, so nothing ever touched
    /// anything and the scene floated with empty frame on all four sides.
    #[test]
    fn both_projections_fit_the_box_tightly_to_the_viewport() {
        fn slack(camera: Camera3D) -> (f32, f32, Axis3Layout) {
            let frame = surface(&[0.0, 1.0], &[0.0, 1.0], &[[0.0, 1.0], [2.0, 3.0]])
                .xlabel("x")
                .ylabel("y")
                .zlabel("z")
                .camera(camera)
                .finalize()
                .resolve()
                .expect("frame");
            let layout = Axis3Layout::resolve(&frame).expect("layout");
            let corners =
                projected_box_corners(layout.camera, layout.viewport).expect("projected corners");
            let (min_x, max_x, min_y, max_y) = corner_bbox(&corners);
            let viewport = layout.viewport;
            // Slack on each axis: how much of the viewport the box leaves empty.
            let horizontal = viewport.width as f32 - (max_x - min_x);
            let vertical = viewport.height as f32 - (max_y - min_y);
            // ... and it still fits: nothing may spill out of the canvas.
            assert!(min_x >= 0.0 && min_y >= 0.0);
            assert!(max_x <= layout.canvas_width as f32);
            assert!(max_y <= layout.canvas_height as f32);
            (horizontal, vertical, layout)
        }

        for (name, camera) in [
            ("orthographic", Camera3D::default().orthographic()),
            ("perspective", Camera3D::default().perspective_deg(45.0)),
        ] {
            let (horizontal, vertical, layout) = slack(camera);
            // One axis is limiting and the other carries the aspect difference
            // between a near-square box silhouette and a 4:3 frame; a tight fit
            // means the limiting one has essentially no slack at all.
            assert!(
                horizontal.min(vertical) < 2.0,
                "the {name} box leaves {horizontal:.0}px horizontal and \
                 {vertical:.0}px vertical slack in a {}x{} viewport — the fit is \
                 not touching either edge",
                layout.viewport.width,
                layout.viewport.height,
            );
        }
    }

    /// The margins are the measured labels, not a percentage of the canvas: a
    /// long axis label has to push the box in, and no axis labels at all has to
    /// let it out.
    #[test]
    fn viewport_margins_follow_the_measured_labels() {
        fn viewport_of(zlabel: &str) -> Viewport3D {
            let plot = surface(&[0.0, 1.0], &[0.0, 1.0], &[[0.0, 1.0], [2.0, 3.0]]);
            let plot = if zlabel.is_empty() {
                plot
            } else {
                plot.zlabel(zlabel)
            };
            let frame = plot.finalize().resolve().expect("frame");
            Axis3Layout::resolve(&frame).expect("layout").viewport
        }

        let bare = viewport_of("");
        let labelled = viewport_of("z");
        let verbose = viewport_of("temperature in degrees celsius");
        assert!(labelled.width < bare.width);
        assert!(verbose.width < labelled.width);
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
