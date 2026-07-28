//! The one place both 3D rasterizers agree on colour space, lighting, and alpha.
//!
//! The CPU raster path and the wgpu path used to each carry their own copy of
//! the sRGB transfer function, the key light, and the alpha convention. Three
//! copies meant three chances to drift, and they had: lighting was applied to
//! sRGB bytes on the CPU and to linear values on the GPU, so the same figure
//! was up to twice as bright depending on which adapter happened to exist.
//!
//! Everything below is shared, so a change lands on both backends at once.
//!
//! # Alpha convention
//!
//! Both backends hand the compositor a [`crate::core::plot::Image`] whose
//! pixels are **straight (non-premultiplied) alpha**, matching PNG and every
//! `SkiaRenderer` entry point. Coverage antialiasing naturally produces
//! *premultiplied* values (an uncovered sample contributes nothing to either
//! the colour sum or the alpha sum), so both backends resolve in premultiplied
//! space and then divide the colour back out exactly once, here.

use glam::Vec3;

#[cfg(feature = "gpu")]
use crate::render::Color;

/// Direction the single key light points from, in aspect-scaled scene space.
///
/// Mirrored verbatim by `shaders/mesh.wgsl`; the parity test in
/// `tests/three_d_parity_test.rs` is what keeps the two in step.
pub(crate) const KEY_LIGHT: Vec3 = Vec3::new(0.35, -0.45, 0.82);

/// Constant ambient term of the shared single-light model.
pub(crate) const AMBIENT_INTENSITY: f32 = 0.35;

/// Lambertian weight of the shared single-light model.
pub(crate) const DIFFUSE_INTENSITY: f32 = 0.65;

/// Convert one sRGB-encoded channel in `0..=1` to linear light.
pub(crate) fn srgb_to_linear(channel: f32) -> f32 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

/// Convert one linear-light channel in `0..=1` back to sRGB encoding.
pub(crate) fn linear_to_srgb(channel: f32) -> f32 {
    if channel <= 0.003_130_8 {
        channel * 12.92
    } else {
        1.055 * channel.powf(1.0 / 2.4) - 0.055
    }
}

/// Convert a stored sRGB colour to linear RGB with straight alpha.
///
/// This is the exact value the GPU uniform buffers carry, so CPU shading and
/// GPU shading start from the same numbers.
#[cfg(feature = "gpu")]
pub(crate) fn linear_color(color: Color) -> [f32; 4] {
    [
        srgb_to_linear(f32::from(color.r) / 255.0),
        srgb_to_linear(f32::from(color.g) / 255.0),
        srgb_to_linear(f32::from(color.b) / 255.0),
        f32::from(color.a) / 255.0,
    ]
}

/// Correct a surface normal for the non-uniform `axis_aspect` scale applied to
/// positions.
///
/// Positions are drawn as `p * aspect`, so a normal has to be transformed by
/// the inverse transpose of that scale — for a diagonal scale, `n / aspect`.
/// Without this a plot squashed with `AxisAspect3D::fixed(1.0, 1.0, 0.2)` shades
/// a visually flat plateau as though it still had its full relief.
///
/// The result is deliberately **not** normalized, matching `shaders/mesh.wgsl`:
/// both backends interpolate the raw corrected normal across the triangle and
/// normalize once per fragment, so they agree everywhere, not only at vertices.
pub(crate) fn aspect_corrected_normal(normal: Vec3, axis_aspect: Vec3) -> Vec3 {
    let corrected = normal / axis_aspect;
    if corrected.is_finite() && corrected.length_squared() > f32::EPSILON {
        corrected
    } else {
        Vec3::Z
    }
}

fn normalized_or_z(normal: Vec3) -> Vec3 {
    if normal.is_finite() && normal.length_squared() > f32::EPSILON {
        normal.normalize()
    } else {
        Vec3::Z
    }
}

/// Shared single-light intensity for a (possibly unnormalized) normal.
pub(crate) fn lambert_intensity(normal: Vec3, two_sided: bool) -> f32 {
    let normal = normalized_or_z(normal);
    let diffuse = normal.dot(KEY_LIGHT.normalize());
    let diffuse = if two_sided {
        diffuse.abs()
    } else {
        diffuse.max(0.0)
    };
    (AMBIENT_INTENSITY + DIFFUSE_INTENSITY * diffuse).clamp(0.0, 1.0)
}

/// Scale one sRGB-encoded channel by a linear-light intensity.
///
/// The GPU multiplies linear values and lets the sRGB render target re-encode;
/// doing anything else on the CPU is what made the two backends disagree.
pub(crate) fn scale_srgb_channel(channel: u8, intensity: f32) -> u8 {
    let linear = srgb_to_linear(f32::from(channel) / 255.0) * intensity.clamp(0.0, 1.0);
    (linear_to_srgb(linear.clamp(0.0, 1.0)) * 255.0).round() as u8
}

/// Divide coverage back out of a premultiplied RGBA buffer that was resolved in
/// sRGB-encoded space (the CPU rasterizer's sample accumulator).
pub(crate) fn unpremultiply_encoded(pixel: [u32; 4], sample_count: u32) -> [u8; 4] {
    let [red, green, blue, alpha] = pixel;
    if alpha == 0 {
        return [0, 0, 0, 0];
    }
    [
        divide_out(red, alpha),
        divide_out(green, alpha),
        divide_out(blue, alpha),
        (alpha / sample_count.max(1)).min(255) as u8,
    ]
}

fn divide_out(weighted: u32, alpha: u32) -> u8 {
    // `weighted` is a sum of `channel * sample_alpha`, `alpha` the matching sum
    // of `sample_alpha`, so the quotient is the coverage-weighted mean colour.
    ((weighted + alpha / 2) / alpha).min(255) as u8
}

/// Divide coverage back out of a premultiplied RGBA buffer whose colour
/// channels were resolved in **linear** space and then sRGB-encoded — that is,
/// anything read back from a `Rgba8UnormSrgb` render target.
#[cfg(feature = "gpu")]
pub(crate) fn unpremultiply_linear_srgb_bytes(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = f32::from(pixel[3]) / 255.0;
        if alpha <= 0.0 {
            pixel[0] = 0;
            pixel[1] = 0;
            pixel[2] = 0;
            continue;
        }
        for channel in &mut pixel[..3] {
            let linear = srgb_to_linear(f32::from(*channel) / 255.0) / alpha;
            *channel = (linear_to_srgb(linear.clamp(0.0, 1.0)) * 255.0).round() as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srgb_transfer_round_trips_every_byte() {
        for byte in 0..=255_u8 {
            let value = f32::from(byte) / 255.0;
            let round_tripped = (linear_to_srgb(srgb_to_linear(value)) * 255.0).round() as u8;
            assert_eq!(round_tripped, byte);
        }
    }

    #[test]
    fn linear_shading_is_brighter_than_the_old_srgb_scaling() {
        // Halving light in linear space is a much smaller perceptual change
        // than halving the stored byte; the CPU used to do the latter.
        let scaled = scale_srgb_channel(255, 0.5);
        assert!(
            scaled > 180,
            "linear half-light encodes near 188, got {scaled}"
        );
        assert!(scaled < 195);
    }

    #[test]
    fn aspect_correction_matches_the_analytic_normal_of_the_scaled_surface() {
        // z = 0.5 * x has normal (-0.5, 0, 1). Squashing z by 0.2 turns the
        // surface into z' = 0.1 * x, whose normal is (-0.1, 0, 1).
        let aspect = Vec3::new(1.0, 1.0, 0.2);
        let corrected = aspect_corrected_normal(Vec3::new(-0.5, 0.0, 1.0), aspect).normalize();
        let expected = Vec3::new(-0.1, 0.0, 1.0).normalize();
        assert!(
            (corrected - expected).length() < 1.0e-6,
            "{corrected:?} != {expected:?}"
        );
        // A plateau that is flat in scene space stays flat however the box is
        // squashed, so its shading must not change with the aspect.
        let plateau = lambert_intensity(aspect_corrected_normal(Vec3::Z, aspect), false);
        assert!((plateau - lambert_intensity(Vec3::Z, false)).abs() < 1.0e-6);
        // ...while a genuine slope is shaded as the *flattened* slope, not the
        // steeper one the raw data implies.
        let flattened = lambert_intensity(
            aspect_corrected_normal(Vec3::new(-0.5, 0.0, 1.0), aspect),
            false,
        );
        let unflattened = lambert_intensity(Vec3::new(-0.5, 0.0, 1.0), false);
        assert_ne!(flattened, unflattened);
        assert!((flattened - lambert_intensity(expected, false)).abs() < 1.0e-6);
    }

    #[test]
    fn uniform_aspect_leaves_normals_alone() {
        let normal = Vec3::new(0.3, -0.4, 0.86).normalize();
        let corrected = aspect_corrected_normal(normal, Vec3::ONE);
        assert!((corrected - normal).length() < 1.0e-6);
    }

    #[test]
    fn half_coverage_resolves_to_the_source_colour_with_half_alpha() {
        // Two of four samples covered by opaque (200, 100, 50).
        let accumulated = [200 * 255 * 2, 100 * 255 * 2, 50 * 255 * 2, 255 * 2];
        assert_eq!(unpremultiply_encoded(accumulated, 4), [200, 100, 50, 127]);
    }

    #[test]
    fn zero_coverage_resolves_to_fully_transparent_black() {
        assert_eq!(unpremultiply_encoded([0, 0, 0, 0], 4), [0, 0, 0, 0]);
    }
}
