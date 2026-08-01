//! Video and animation encoders
//!
//! This module provides the `Encoder` trait and implementations for
//! various output formats.
//!
//! # Available Encoders
//!
//! - `GifEncoder` - Animated GIF (always available with `animation` feature)
//!
//! AV1/container output is represented by [`Codec::Av1`] for API compatibility,
//! but is not currently implemented.

mod gif;

pub use gif::GifEncoder;

use crate::core::{PlottingError, Result};
use std::path::Path;

/// Video quality preset
///
/// Controls the trade-off between encoding speed and output quality/size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Quality {
    /// Fast encoding, larger file size
    Low,
    /// Balanced speed and quality
    #[default]
    Medium,
    /// Slower encoding, better quality
    High,
    /// Maximum quality (not available for all formats)
    Lossless,
}

impl Quality {
    /// Convert to the historical rav1e speed preset (0-10, higher = faster).
    ///
    /// Ruviz does not currently ship an AV1 encoder. This method remains so
    /// code compiled with the compatibility `animation-video` feature keeps
    /// building while the container/encoder contract is redesigned.
    #[cfg(feature = "animation-video")]
    #[deprecated(note = "AV1 video output is not currently implemented")]
    pub fn to_rav1e_speed(self) -> u8 {
        match self {
            Quality::Low => 10,
            Quality::Medium => 6,
            Quality::High => 2,
            Quality::Lossless => 0,
        }
    }

    /// Convert to GIF encoding speed (1-30, higher = faster)
    pub fn to_gif_speed(self) -> i32 {
        match self {
            Quality::Low => 30,
            Quality::Medium => 10,
            Quality::High => 1,
            Quality::Lossless => 1,
        }
    }
}

/// Video codec selection
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Codec {
    /// Animated GIF
    Gif,
    /// AV1 codec (reserved; currently unsupported)
    Av1,
    /// Auto-detect from file extension
    #[default]
    Auto,
}

impl Codec {
    /// Detect codec from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "gif" => Some(Codec::Gif),
            "mp4" | "webm" | "mkv" => Some(Codec::Av1),
            _ => None,
        }
    }

    /// Get the default file extension for this codec
    pub fn default_extension(&self) -> &'static str {
        match self {
            Codec::Gif => "gif",
            Codec::Av1 => "mp4",
            Codec::Auto => "gif",
        }
    }
}

/// Trait for video/animation encoders
///
/// Implementors of this trait can encode frames into video files.
/// The encoding process has three phases:
///
/// 1. **Initialization** (`init`): Set up the encoder with frame dimensions
/// 2. **Encoding** (`encode_frame`): Add frames one at a time
/// 3. **Finalization** (`finalize`): Flush buffers and close the file
///
/// # Example Implementation
///
/// ```rust,ignore
/// struct MyEncoder { /* ... */ }
///
/// impl Encoder for MyEncoder {
///     fn init(&mut self, width: u32, height: u32) -> Result<()> {
///         // Set up encoder for given dimensions
///         Ok(())
///     }
///
///     fn encode_frame(&mut self, rgb_data: &[u8], timestamp_ms: u64) -> Result<()> {
///         // Encode one frame
///         Ok(())
///     }
///
///     fn finalize(self: Box<Self>) -> Result<()> {
///         // Finish encoding and write file
///         Ok(())
///     }
///
///     fn extensions(&self) -> &[&str] {
///         &["mp4", "webm"]
///     }
/// }
/// ```
pub trait Encoder: Send {
    /// Initialize the encoder with frame dimensions
    ///
    /// Must be called before `encode_frame`. The width and height
    /// must remain constant for all frames.
    fn init(&mut self, width: u32, height: u32) -> Result<()>;

    /// Encode a single frame
    ///
    /// # Arguments
    ///
    /// * `rgb_data` - Raw RGB pixel data (width * height * 3 bytes)
    /// * `timestamp_ms` - Frame timestamp in milliseconds
    fn encode_frame(&mut self, rgb_data: &[u8], timestamp_ms: u64) -> Result<()>;

    /// Finalize encoding and write the output file
    ///
    /// This consumes the encoder and must be called to produce valid output.
    fn finalize(self: Box<Self>) -> Result<()>;

    /// Get supported file extensions for this encoder
    fn extensions(&self) -> &[&str];

    /// Check if this encoder supports the given file extension
    fn supports_extension(&self, ext: &str) -> bool {
        let ext_lower = ext.to_lowercase();
        self.extensions()
            .iter()
            .any(|e| e.to_lowercase() == ext_lower)
    }
}

/// Create an encoder for the given output path
///
/// Automatically selects the appropriate encoder based on file extension.
///
/// # Arguments
///
/// * `path` - Output file path
/// * `quality` - Encoding quality preset
///
/// # Returns
///
/// A boxed encoder ready for initialization, or an error if the format
/// is not supported.
pub fn create_encoder(path: &Path, quality: Quality) -> Result<Box<dyn Encoder>> {
    let Some(extension) = path.extension() else {
        return Ok(Box::new(GifEncoder::new(path, quality)?));
    };
    let ext = extension
        .to_str()
        .ok_or_else(|| PlottingError::UnsupportedFormat("non-UTF-8 extension".into()))?;

    match Codec::from_extension(ext) {
        Some(Codec::Gif) => Ok(Box::new(GifEncoder::new(path, quality)?)),
        Some(Codec::Av1) => unsupported_av1(),
        Some(Codec::Auto) => {
            // Default to GIF
            Ok(Box::new(GifEncoder::new(path, quality)?))
        }
        None => Err(PlottingError::UnsupportedFormat(ext.to_string())),
    }
}

pub(super) fn create_encoder_for_codec(
    path: &Path,
    quality: Quality,
    codec: Codec,
) -> Result<Box<dyn Encoder>> {
    match codec {
        Codec::Auto => create_encoder(path, quality),
        Codec::Gif => Ok(Box::new(GifEncoder::new(path, quality)?)),
        Codec::Av1 => unsupported_av1(),
    }
}

fn unsupported_av1<T>() -> Result<T> {
    Err(PlottingError::UnsupportedOperation {
        operation: "AV1 animation encoding",
        reason: "no AV1 container/muxing implementation is currently shipped".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codec_from_extension() {
        assert_eq!(Codec::from_extension("gif"), Some(Codec::Gif));
        assert_eq!(Codec::from_extension("GIF"), Some(Codec::Gif));
        assert_eq!(Codec::from_extension("mp4"), Some(Codec::Av1));
        assert_eq!(Codec::from_extension("webm"), Some(Codec::Av1));
        assert_eq!(Codec::from_extension("unknown"), None);
    }

    #[test]
    fn test_quality_gif_speed() {
        assert_eq!(Quality::Low.to_gif_speed(), 30);
        assert_eq!(Quality::Medium.to_gif_speed(), 10);
        assert_eq!(Quality::High.to_gif_speed(), 1);
    }

    #[test]
    fn test_codec_default_extension() {
        assert_eq!(Codec::Gif.default_extension(), "gif");
        assert_eq!(Codec::Av1.default_extension(), "mp4");
    }

    #[test]
    fn unknown_extensions_are_rejected_instead_of_writing_gif_data() {
        let result = create_encoder(Path::new("animation.unknown"), Quality::Medium);
        assert!(
            matches!(result, Err(PlottingError::UnsupportedFormat(extension)) if extension == "unknown")
        );
    }

    #[test]
    fn explicit_gif_codec_does_not_depend_on_the_path_extension() {
        assert!(
            create_encoder_for_codec(Path::new("animation.custom"), Quality::Medium, Codec::Gif)
                .is_ok()
        );
    }

    #[test]
    fn av1_is_reported_as_an_unsupported_operation() {
        let result =
            create_encoder_for_codec(Path::new("animation.webm"), Quality::Medium, Codec::Av1);
        assert!(matches!(
            result,
            Err(PlottingError::UnsupportedOperation {
                operation: "AV1 animation encoding",
                ..
            })
        ));
    }
}
