//! Image representation for rendered plots

use std::borrow::Cow;

/// Alpha representation used by an RGBA pixel buffer.
///
/// The distinction matters for translucent pixels. In [`Straight`](Self::Straight)
/// alpha, the RGB channels retain their unattenuated colour. In
/// [`Premultiplied`](Self::Premultiplied) alpha, each RGB channel has already
/// been multiplied by the alpha channel.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlphaMode {
    /// Straight (non-premultiplied) RGBA, as used by PNG and most image APIs.
    #[default]
    Straight,
    /// Premultiplied RGBA, as used by tiny-skia and many native compositors.
    Premultiplied,
}

/// In-memory image representation
///
/// `Image` has one canonical representation: straight-alpha RGBA. Keeping that
/// invariant lets existing public `Image { width, height, pixels }` literals
/// remain source-compatible while adapters can request premultiplied bytes
/// explicitly with [`pixels_in_alpha_mode`](Self::pixels_in_alpha_mode).
#[derive(Debug, Clone)]
pub struct Image {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Pixel data in straight-alpha RGBA byte order.
    pub pixels: Vec<u8>,
}

impl Image {
    /// Create an image from straight-alpha RGBA pixels.
    ///
    /// This preserves the historical `Image::new` contract and does not alter
    /// the supplied bytes. Producers that return native premultiplied pixels
    /// should use [`from_premultiplied_rgba`](Self::from_premultiplied_rgba).
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        Self::from_straight_rgba(width, height, pixels)
    }

    /// Create an image from straight-alpha RGBA pixels without altering them.
    pub fn from_straight_rgba(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        Self {
            width,
            height,
            pixels,
        }
    }

    /// Create an image from premultiplied-alpha RGBA pixels.
    ///
    /// The supplied pixels are normalized to this type's canonical
    /// straight-alpha representation.
    pub fn from_premultiplied_rgba(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        Self::with_alpha_mode(width, height, pixels, AlphaMode::Premultiplied)
    }

    /// Create an image from pixels with an explicit source alpha representation.
    ///
    /// Premultiplied input is normalized to straight alpha.
    pub fn with_alpha_mode(
        width: u32,
        height: u32,
        mut pixels: Vec<u8>,
        alpha_mode: AlphaMode,
    ) -> Self {
        convert_rgba_alpha_mode(&mut pixels, alpha_mode, AlphaMode::Straight);
        Self {
            width,
            height,
            pixels,
        }
    }

    /// Get image width
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get image height
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Return the alpha representation used by the canonical pixel buffer.
    pub fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Straight
    }

    /// Borrow straight-alpha pixels or return an owned premultiplied conversion.
    pub fn pixels_in_alpha_mode(&self, alpha_mode: AlphaMode) -> Cow<'_, [u8]> {
        if alpha_mode == AlphaMode::Straight {
            Cow::Borrowed(&self.pixels)
        } else {
            let mut pixels = self.pixels.clone();
            convert_rgba_alpha_mode(&mut pixels, AlphaMode::Straight, alpha_mode);
            Cow::Owned(pixels)
        }
    }

    /// Encode the image as PNG bytes.
    pub fn encode_png(&self) -> crate::core::Result<Vec<u8>> {
        crate::export::encode_rgba_png(self)
    }
}

/// Composite one straight-alpha RGBA source pixel over a destination pixel.
///
/// Both inputs and the returned pixel use unassociated (straight) colour
/// channels. The destination alpha is included in the colour calculation, so
/// this remains correct over transparent and translucent backgrounds.
pub fn source_over_straight_rgba(destination: [u8; 4], source: [u8; 4]) -> [u8; 4] {
    let source_alpha = u64::from(source[3]);
    if source_alpha == 0 {
        return destination;
    }
    if source_alpha == 255 {
        return source;
    }

    let destination_alpha = u64::from(destination[3]);
    let inverse_source_alpha = 255 - source_alpha;
    let output_alpha_numerator = source_alpha * 255 + destination_alpha * inverse_source_alpha;
    if output_alpha_numerator == 0 {
        return [0; 4];
    }

    let mut output = [0; 4];
    for channel in 0..3 {
        let color_numerator = u64::from(source[channel]) * source_alpha * 255
            + u64::from(destination[channel]) * destination_alpha * inverse_source_alpha;
        output[channel] = ((color_numerator + output_alpha_numerator / 2) / output_alpha_numerator)
            .min(255) as u8;
    }
    output[3] = ((output_alpha_numerator + 127) / 255).min(255) as u8;
    output
}

pub(crate) fn convert_rgba_alpha_mode(pixels: &mut [u8], from: AlphaMode, to: AlphaMode) {
    match (from, to) {
        (AlphaMode::Straight, AlphaMode::Premultiplied) => {
            for pixel in pixels.chunks_exact_mut(4) {
                let alpha = u32::from(pixel[3]);
                for channel in &mut pixel[..3] {
                    *channel = ((u32::from(*channel) * alpha + 127) / 255) as u8;
                }
            }
        }
        (AlphaMode::Premultiplied, AlphaMode::Straight) => {
            for pixel in pixels.chunks_exact_mut(4) {
                let alpha = u32::from(pixel[3]);
                if alpha == 0 {
                    pixel[..3].fill(0);
                    continue;
                }
                for channel in &mut pixel[..3] {
                    *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{AlphaMode, Image, source_over_straight_rgba};
    use std::borrow::Cow;

    #[test]
    fn new_preserves_pixels_and_defaults_to_straight_alpha() {
        let pixels = vec![20, 40, 60, 128];
        let image = Image::new(1, 1, pixels.clone());

        assert_eq!(image.pixels, pixels);
        assert_eq!(image.alpha_mode(), AlphaMode::Straight);
    }

    #[test]
    fn premultiplied_constructor_normalizes_to_canonical_straight_alpha() {
        let image = Image::from_premultiplied_rgba(1, 1, vec![64, 32, 16, 128]);

        assert_eq!(image.pixels, vec![128, 64, 32, 128]);
        assert_eq!(image.alpha_mode(), AlphaMode::Straight);
    }

    #[test]
    fn requested_premultiplied_pixels_convert_without_mutating_image() {
        let image = Image::from_straight_rgba(2, 1, vec![128, 64, 32, 128, 7, 9, 11, 255]);
        let premultiplied = image.pixels_in_alpha_mode(AlphaMode::Premultiplied);

        assert_eq!(premultiplied.as_ref(), &[64, 32, 16, 128, 7, 9, 11, 255]);
        assert_eq!(image.pixels, vec![128, 64, 32, 128, 7, 9, 11, 255]);
    }

    #[test]
    fn normalizing_zero_alpha_discards_unrecoverable_rgb() {
        let image = Image::from_premultiplied_rgba(1, 1, vec![99, 88, 77, 0]);

        assert_eq!(image.pixels, vec![0, 0, 0, 0]);
    }

    #[test]
    fn matching_alpha_mode_returns_borrowed_pixels() {
        let image = Image::new(1, 1, vec![1, 2, 3, 4]);

        assert!(matches!(
            image.pixels_in_alpha_mode(AlphaMode::Straight),
            Cow::Borrowed(_)
        ));
        assert!(matches!(
            image.pixels_in_alpha_mode(AlphaMode::Premultiplied),
            Cow::Owned(_)
        ));
    }

    #[test]
    fn png_encoding_uses_canonical_straight_pixels() {
        let image = Image::from_premultiplied_rgba(1, 1, vec![64, 32, 16, 128]);
        let encoded = image.encode_png().expect("PNG should encode");
        let decoded = ::image::load_from_memory(&encoded)
            .expect("PNG should decode")
            .to_rgba8();

        assert_eq!(decoded.as_raw(), &[128, 64, 32, 128]);
        assert_eq!(image.pixels, vec![128, 64, 32, 128]);
        assert_eq!(image.alpha_mode(), AlphaMode::Straight);
    }

    #[test]
    fn public_struct_literal_remains_source_compatible_and_straight() {
        let image = Image {
            width: 1,
            height: 1,
            pixels: vec![10, 20, 30, 128],
        };

        assert_eq!(image.alpha_mode(), AlphaMode::Straight);
    }

    #[test]
    fn straight_source_over_preserves_transparent_destination_color_math() {
        assert_eq!(
            source_over_straight_rgba([0, 0, 255, 0], [255, 0, 0, 128]),
            [255, 0, 0, 128]
        );
    }

    #[test]
    fn straight_source_over_includes_translucent_destination_alpha() {
        assert_eq!(
            source_over_straight_rgba([0, 0, 255, 128], [255, 0, 0, 128]),
            [170, 0, 85, 192]
        );
    }
}
