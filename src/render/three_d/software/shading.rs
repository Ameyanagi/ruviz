use glam::Vec3;

use crate::plots::SurfaceShading;
use crate::render::Color;
use crate::render::three_d::color::{lambert_intensity, scale_srgb_channel};

/// Shade a surface colour with the shared single-light model.
///
/// The intensity and the colour space both come from
/// [`crate::render::three_d::color`], so this stays bit-comparable with
/// `shaders/mesh.wgsl` instead of drifting into an sRGB-space approximation.
pub(super) fn shade(color: Color, normal: Vec3, shading: SurfaceShading, two_sided: bool) -> Color {
    if shading == SurfaceShading::Unlit {
        return color;
    }

    let intensity = lambert_intensity(normal, two_sided);
    Color::from_rgba(
        scale_srgb_channel(color.r, intensity),
        scale_srgb_channel(color.g, intensity),
        scale_srgb_channel(color.b, intensity),
        color.a,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::three_d::color::{linear_to_srgb, srgb_to_linear};

    #[test]
    fn unlit_color_is_unchanged() {
        let color = Color::from_rgb(120, 180, 240);
        assert_eq!(
            shade(color, Vec3::new(1.0, 0.0, 0.0), SurfaceShading::Unlit, true),
            color
        );
    }

    #[test]
    fn two_sided_lighting_is_symmetric() {
        let color = Color::from_rgb(120, 180, 240);
        assert_eq!(
            shade(color, Vec3::Z, SurfaceShading::Smooth, true),
            shade(color, -Vec3::Z, SurfaceShading::Smooth, true)
        );
    }

    #[test]
    fn lighting_is_applied_in_linear_space_like_the_gpu_shader() {
        // The GPU multiplies a linear colour and lets the sRGB target re-encode
        // it. Reproduce that by hand and require the CPU to match exactly.
        let color = Color::from_rgb(120, 180, 240);
        let normal = Vec3::new(1.0, 0.0, 0.0);
        let shaded = shade(color, normal, SurfaceShading::Smooth, false);
        let intensity = lambert_intensity(normal, false);
        for (actual, source) in [
            (shaded.r, color.r),
            (shaded.g, color.g),
            (shaded.b, color.b),
        ] {
            let linear = srgb_to_linear(f32::from(source) / 255.0) * intensity;
            let expected = (linear_to_srgb(linear.clamp(0.0, 1.0)) * 255.0).round() as u8;
            assert_eq!(actual, expected);
        }

        // The old sRGB-space scaling was materially darker; keep the
        // regression visible instead of asserting only self-consistency.
        let srgb_space = (f32::from(color.r) * intensity).round() as u8;
        assert!(
            shaded.r > srgb_space,
            "linear shading must be brighter than the sRGB-space scaling it replaced"
        );
    }
}
