//! GIF encoder implementation
//!
//! Provides animated GIF encoding using the `gif` crate.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use gif::{Encoder as GifEncoderInner, Frame, Repeat};

use super::{Encoder, Quality};
use crate::core::{PlottingError, Result};

/// Animated GIF encoder
///
/// Encodes frames into an animated GIF file using color quantization
/// to reduce each frame to 256 colors.
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::animation::encoders::{GifEncoder, Encoder, Quality};
///
/// let mut encoder = GifEncoder::new("output.gif", Quality::Medium)?;
/// encoder.init(800, 600)?;
///
/// // Encode frames...
/// encoder.encode_frame(&rgb_data, 0)?;
/// encoder.encode_frame(&rgb_data, 33)?;
///
/// Box::new(encoder).finalize()?;
/// ```
pub struct GifEncoder {
    encoder: Option<GifEncoderInner<BufWriter<File>>>,
    width: u16,
    height: u16,
    quality: Quality,
    frame_delay: u16,
    initialized: bool,
    path: std::path::PathBuf,
}

impl GifEncoder {
    /// Create a new GIF encoder for the given output path
    ///
    /// # Arguments
    ///
    /// * `path` - Output file path
    /// * `quality` - Encoding quality (affects color quantization speed)
    pub fn new<P: AsRef<Path>>(path: P, quality: Quality) -> Result<Self> {
        Ok(Self {
            encoder: None,
            width: 0,
            height: 0,
            quality,
            frame_delay: 3, // ~33ms default (30 FPS)
            initialized: false,
            path: path.as_ref().to_path_buf(),
        })
    }

    /// Set the frame delay in centiseconds (1/100th of a second)
    ///
    /// For 30 FPS, use delay of 3 (33ms).
    /// For 60 FPS, use delay of 2 (17ms, rounded up).
    pub fn with_frame_delay(mut self, delay_cs: u16) -> Self {
        self.frame_delay = delay_cs;
        self
    }

    /// Set frame delay from framerate
    pub fn with_framerate(mut self, fps: f64) -> Self {
        // Convert FPS to centiseconds delay
        // 30 FPS = 33.3ms = 3.33 centiseconds ≈ 3
        self.frame_delay = ((100.0 / fps).round() as u16).max(1);
        self
    }

    /// Quantize RGB data to indexed color
    fn quantize_frame(&self, rgb_data: &[u8]) -> (Vec<u8>, Vec<u8>) {
        // Convert RGB to RGBA for color_quant (expects RGBA format)
        let rgba_data: Vec<u8> = rgb_data
            .chunks(3)
            .flat_map(|rgb| vec![rgb[0], rgb[1], rgb[2], 255])
            .collect();

        // Use color_quant for palette generation
        let nq = color_quant::NeuQuant::new(self.quality.to_gif_speed(), 256, &rgba_data);

        // Build palette (256 * 3 bytes) from RGBA color map
        let palette: Vec<u8> = (0..256)
            .flat_map(|i| {
                if let Some(color) = nq.lookup(i) {
                    vec![color[0], color[1], color[2]]
                } else {
                    vec![0, 0, 0] // Fallback for unused palette entries
                }
            })
            .collect();

        // Map pixels to palette indices (using RGBA pixels)
        let indices: Vec<u8> = rgba_data
            .chunks(4)
            .map(|pixel| nq.index_of(pixel) as u8)
            .collect();

        (palette, indices)
    }
}

/// Convert GIF encoding errors to PlottingError
fn gif_error_to_plotting_error(err: gif::EncodingError) -> PlottingError {
    PlottingError::RenderError(format!("GIF encoding error: {}", err))
}

impl Encoder for GifEncoder {
    fn init(&mut self, width: u32, height: u32) -> Result<()> {
        if self.initialized {
            return Err(PlottingError::RenderError(
                "GIF encoder already initialized".into(),
            ));
        }

        self.width = width as u16;
        self.height = height as u16;

        let file = File::create(&self.path)?;
        let writer = BufWriter::new(file);

        let mut encoder = GifEncoderInner::new(
            writer,
            self.width,
            self.height,
            &[], // Global palette (empty, use local per-frame)
        )
        .map_err(gif_error_to_plotting_error)?;

        encoder
            .set_repeat(Repeat::Infinite)
            .map_err(gif_error_to_plotting_error)?;
        self.encoder = Some(encoder);
        self.initialized = true;

        Ok(())
    }

    fn encode_frame(&mut self, rgb_data: &[u8], _timestamp_ms: u64) -> Result<()> {
        if !self.initialized {
            return Err(PlottingError::RenderError(
                "GIF encoder not initialized".into(),
            ));
        }

        let expected_len = self.width as usize * self.height as usize * 3;
        if rgb_data.len() != expected_len {
            return Err(PlottingError::RenderError(format!(
                "Invalid frame data: expected {} bytes, got {}",
                expected_len,
                rgb_data.len()
            )));
        }

        // Quantize to indexed color
        let (palette, indices) = self.quantize_frame(rgb_data);

        // Create frame with struct initialization
        let frame = Frame {
            width: self.width,
            height: self.height,
            delay: self.frame_delay,
            palette: Some(palette),
            buffer: std::borrow::Cow::Owned(indices),
            ..Frame::default()
        };

        // Write frame
        self.encoder
            .as_mut()
            .unwrap()
            .write_frame(&frame)
            .map_err(gif_error_to_plotting_error)?;

        Ok(())
    }

    fn finalize(self: Box<Self>) -> Result<()> {
        // Encoder is finalized when dropped
        drop(self.encoder);
        Ok(())
    }

    fn extensions(&self) -> &[&str] {
        &["gif"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_gif_encoder_creation() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.gif");

        let encoder = GifEncoder::new(&path, Quality::Medium);
        assert!(encoder.is_ok());
    }

    #[test]
    fn test_gif_encoder_with_framerate() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.gif");

        let encoder = GifEncoder::new(&path, Quality::Medium)
            .unwrap()
            .with_framerate(30.0);

        assert_eq!(encoder.frame_delay, 3);

        let encoder60 = GifEncoder::new(&path, Quality::Medium)
            .unwrap()
            .with_framerate(60.0);

        assert_eq!(encoder60.frame_delay, 2);
    }

    #[test]
    fn test_gif_encoder_init() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.gif");

        let mut encoder = GifEncoder::new(&path, Quality::Medium).unwrap();
        assert!(encoder.init(100, 100).is_ok());
        assert!(encoder.initialized);
    }

    #[test]
    fn test_gif_encoder_double_init() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.gif");

        let mut encoder = GifEncoder::new(&path, Quality::Medium).unwrap();
        encoder.init(100, 100).unwrap();

        let result = encoder.init(100, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_gif_encoder_encode_without_init() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.gif");

        let mut encoder = GifEncoder::new(&path, Quality::Medium).unwrap();
        let rgb_data = vec![0u8; 100 * 100 * 3];

        let result = encoder.encode_frame(&rgb_data, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_gif_encoder_full_workflow() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.gif");

        let mut encoder = GifEncoder::new(&path, Quality::Low).unwrap();
        encoder.init(10, 10).unwrap();

        // Create a simple gradient frame
        let rgb_data: Vec<u8> = (0..10 * 10)
            .flat_map(|i| {
                let v = ((i * 255) / 100) as u8;
                vec![v, v, v]
            })
            .collect();

        encoder.encode_frame(&rgb_data, 0).unwrap();
        encoder.encode_frame(&rgb_data, 33).unwrap();

        Box::new(encoder).finalize().unwrap();

        // Verify file was created
        assert!(path.exists());
    }

    #[test]
    fn test_gif_encoder_invalid_frame_size() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.gif");

        let mut encoder = GifEncoder::new(&path, Quality::Medium).unwrap();
        encoder.init(100, 100).unwrap();

        // Wrong size data
        let rgb_data = vec![0u8; 50 * 50 * 3];
        let result = encoder.encode_frame(&rgb_data, 0);
        assert!(result.is_err());
    }
}
