//! Analytic sphere geometry shared by software rendering and picking.

use glam::{Mat4, Vec2, Vec3};

use crate::core::plot3d::layout::Axis3Layout;
use crate::core::plot3d::spheres::SphereStyle3D;
use crate::render::Color;

use super::color::{linear_to_srgb, srgb_to_linear};

/// The normalized scene may stretch a data-space sphere into an ellipsoid.
/// Solve in unit-sphere coordinates while retaining scene-space ray distance.
pub(crate) fn intersect(origin: Vec3, direction: Vec3, center: Vec3, radii: Vec3) -> Option<f32> {
    let o = (origin - center) / radii;
    let d = direction / radii;
    let a = d.dot(d);
    let b = o.dot(d);
    let c = o.dot(o) - 1.0;
    let discriminant = b * b - a * c;
    if discriminant < 0.0 || !discriminant.is_finite() || a <= 0.0 {
        return None;
    }
    let root = discriminant.sqrt();
    let near = (-b - root) / a;
    let far = (-b + root) / a;
    let t = if near >= 0.0 { near } else { far };
    (t >= 0.0 && t.is_finite()).then_some(t)
}

#[derive(Clone, Copy)]
pub(crate) struct SphereCamera3D {
    pub(crate) matrix: Mat4,
    pub(crate) inverse: Mat4,
    pub(crate) normal_to_view: Mat4,
    /// Viewport x, y, width, height in physical pixels.
    pub(crate) viewport: [f32; 4],
}

impl SphereCamera3D {
    pub(crate) fn new(layout: &Axis3Layout) -> Self {
        let aspect = layout.camera.axis_aspect;
        let matrix = layout.camera.view_projection * Mat4::from_scale(aspect);
        Self {
            matrix,
            inverse: matrix.inverse(),
            normal_to_view: layout.camera.view * Mat4::from_scale(aspect.recip()),
            viewport: [
                layout.viewport.x as f32,
                layout.viewport.y as f32,
                layout.viewport.width as f32,
                layout.viewport.height as f32,
            ],
        }
    }

    pub(crate) fn ray(self, pixel: Vec2) -> (Vec3, Vec3) {
        let [x, y, w, h] = self.viewport;
        let ndc = Vec2::new((pixel.x - x) / w * 2.0 - 1.0, 1.0 - (pixel.y - y) / h * 2.0);
        let near = self.inverse.project_point3(ndc.extend(0.0));
        let far = self.inverse.project_point3(ndc.extend(1.0));
        (near, (far - near).normalize())
    }

    /// Conservative projected box. An eye-plane crossing uses the viewport;
    /// the exact ray intersection still rejects every uncovered fragment.
    pub(crate) fn screen_bounds(self, center: Vec3, radii: Vec3) -> (Vec2, Vec2) {
        let mut low = Vec2::splat(f32::INFINITY);
        let mut high = Vec2::splat(f32::NEG_INFINITY);
        let [x, y, w, h] = self.viewport;
        for i in 0..8 {
            let sign = Vec3::new(
                if i & 1 == 0 { -1.0 } else { 1.0 },
                if i & 2 == 0 { -1.0 } else { 1.0 },
                if i & 4 == 0 { -1.0 } else { 1.0 },
            );
            let clip = self.matrix * (center + radii * sign).extend(1.0);
            if clip.w <= 1e-6 {
                return (Vec2::new(x, y), Vec2::new(x + w, y + h));
            }
            let ndc = clip.truncate() / clip.w;
            let screen = Vec2::new(x + (ndc.x * 0.5 + 0.5) * w, y + (0.5 - ndc.y * 0.5) * h);
            low = low.min(screen);
            high = high.max(screen);
        }
        (low, high)
    }
}

/// Camera-relative upper-left light, evaluated in linear RGB. A fixed view
/// vector keeps the default highlight restrained and stable during orbit.
pub(crate) fn shade(color: Color, normal_view: Vec3, style: SphereStyle3D) -> Color {
    if !style.shaded {
        return color;
    }
    let normal = normal_view.normalize();
    let light = Vec3::new(-0.35, 0.45, 0.82).normalize();
    let diffuse = normal.dot(light).max(0.0);
    let intensity = 0.3 + 0.7 * diffuse;
    let highlight = if diffuse > 0.0 {
        style.specular
            * normal
                .dot((light + Vec3::Z).normalize())
                .max(0.0)
                .powf(style.gloss)
    } else {
        0.0
    };
    let channels = [color.r, color.g, color.b].map(|c| {
        let linear = srgb_to_linear(f32::from(c) / 255.0) * intensity + highlight;
        (linear_to_srgb(linear.clamp(0.0, 1.0)) * 255.0).round() as u8
    });
    Color::from_rgba(channels[0], channels[1], channels[2], color.a)
}
