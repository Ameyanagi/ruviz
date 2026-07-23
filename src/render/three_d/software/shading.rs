use glam::Vec3;

use crate::plots::SurfaceShading;
use crate::render::Color;

pub(super) fn shade(color: Color, normal: Vec3, shading: SurfaceShading, two_sided: bool) -> Color {
    if shading == SurfaceShading::Unlit {
        return color;
    }

    let normal = if normal.is_finite() && normal.length_squared() > f32::EPSILON {
        normal.normalize()
    } else {
        Vec3::Z
    };
    let light = Vec3::new(0.35, -0.45, 0.82).normalize();
    let diffuse = normal.dot(light);
    let diffuse = if two_sided {
        diffuse.abs()
    } else {
        diffuse.max(0.0)
    };
    let intensity = 0.35 + 0.65 * diffuse;
    Color::new_rgba(
        scale_channel(color.r, intensity),
        scale_channel(color.g, intensity),
        scale_channel(color.b, intensity),
        color.a,
    )
}

fn scale_channel(channel: u8, intensity: f32) -> u8 {
    (f32::from(channel) * intensity.clamp(0.0, 1.0)).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlit_color_is_unchanged() {
        let color = Color::new(120, 180, 240);
        assert_eq!(
            shade(color, Vec3::new(1.0, 0.0, 0.0), SurfaceShading::Unlit, true),
            color
        );
    }

    #[test]
    fn two_sided_lighting_is_symmetric() {
        let color = Color::new(120, 180, 240);
        assert_eq!(
            shade(color, Vec3::Z, SurfaceShading::Smooth, true),
            shade(color, -Vec3::Z, SurfaceShading::Smooth, true)
        );
    }
}
