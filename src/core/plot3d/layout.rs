use glam::{Vec2, Vec3};

use crate::core::{PlottingError, Result};
use crate::render::skia::{format_tick_labels, generate_ticks};

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

        let viewport = axis_viewport(frame, canvas_width, canvas_height);
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

fn axis_viewport(frame: &ResolvedFrame3D, canvas_width: u32, canvas_height: u32) -> Viewport3D {
    let width = canvas_width as f32;
    let height = canvas_height as f32;
    let dpi_scale = frame.figure.dpi / 72.0;
    let left = (width * 0.14).max(42.0 * dpi_scale);
    let right = (width * 0.10).max(24.0 * dpi_scale);
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
    let left_half_width = left.text.chars().count() as f32 * font_size * 0.31;
    let right_half_width = right.text.chars().count() as f32 * font_size * 0.31;
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
}
