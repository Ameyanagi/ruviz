//! Frame capture and video streaming
//!
//! Provides frame capture from Plot rendering and buffered video encoding.

use std::path::Path;

use super::encoders::{Codec, Encoder, Quality, create_encoder};
use super::tick::Tick;
use crate::core::{Plot, PlottingError, Result};

/// Configuration for video output
#[derive(Clone, Debug)]
pub struct VideoConfig {
    /// Output width in pixels
    pub width: u32,
    /// Output height in pixels
    pub height: u32,
    /// Frames per second
    pub framerate: u32,
    /// Encoding quality
    pub quality: Quality,
    /// Video codec
    pub codec: Codec,
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            framerate: 30,
            quality: Quality::Medium,
            codec: Codec::Auto,
        }
    }
}

impl VideoConfig {
    /// Create a new video config with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set output dimensions
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set framerate
    pub fn framerate(mut self, fps: u32) -> Self {
        self.framerate = fps;
        self
    }

    /// Set quality preset
    pub fn quality(mut self, quality: Quality) -> Self {
        self.quality = quality;
        self
    }

    /// Set codec
    pub fn codec(mut self, codec: Codec) -> Self {
        self.codec = codec;
        self
    }

    /// Detect config from output path
    ///
    /// Infers codec from file extension.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Self {
        let ext = path
            .as_ref()
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("gif");

        let codec = Codec::from_extension(ext).unwrap_or(Codec::Gif);

        Self {
            codec,
            ..Default::default()
        }
    }

    /// Get frame delay in centiseconds (for GIF)
    pub fn frame_delay_cs(&self) -> u16 {
        ((100.0 / self.framerate as f64).round() as u16).max(1)
    }

    /// Get frame duration in seconds
    pub fn frame_duration(&self) -> f64 {
        1.0 / self.framerate as f64
    }
}

/// Captures rendered frames from plots
///
/// `FrameCapture` maintains a reusable buffer for efficient frame capture,
/// avoiding allocations on each frame.
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::animation::FrameCapture;
/// use ruviz::prelude::*;
///
/// let mut capture = FrameCapture::new(800, 600);
///
/// let plot = Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]);
/// let frame_data = capture.capture(&plot)?;
/// ```
pub struct FrameCapture {
    width: u32,
    height: u32,
    /// Reusable RGB buffer
    buffer: Vec<u8>,
}

impl FrameCapture {
    /// Create a new frame capture with the given dimensions
    pub fn new(width: u32, height: u32) -> Self {
        let buffer_size = (width * height * 3) as usize;
        Self {
            width,
            height,
            buffer: vec![0u8; buffer_size],
        }
    }

    /// Get the capture dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Resize the capture buffer
    pub fn resize(&mut self, width: u32, height: u32) {
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            let buffer_size = (width * height * 3) as usize;
            self.buffer.resize(buffer_size, 0);
        }
    }

    /// Capture a frame from the plot
    ///
    /// Renders the plot to an internal buffer and returns a reference
    /// to the RGB pixel data.
    pub fn capture(&mut self, plot: &Plot) -> Result<&[u8]> {
        // Create a temporary plot with the desired size
        let sized_plot = plot.clone().size_px(self.width, self.height);

        // Render plot to RGBA buffer
        let image = sized_plot.render()?;
        let rgba_data = &image.pixels;

        // Convert RGBA to RGB
        let pixels = (self.width * self.height) as usize;
        for i in 0..pixels {
            self.buffer[i * 3] = rgba_data[i * 4]; // R
            self.buffer[i * 3 + 1] = rgba_data[i * 4 + 1]; // G
            self.buffer[i * 3 + 2] = rgba_data[i * 4 + 2]; // B
            // Alpha is discarded
        }

        Ok(&self.buffer)
    }

    /// Capture a frame with figure-preserving mode
    ///
    /// When `figure_size` is provided, renders the plot with that figure size
    /// and automatically calculates DPI to achieve the target pixel dimensions.
    /// This produces animation frames with the same visual styling as static plots.
    ///
    /// # Arguments
    ///
    /// * `plot` - The plot to capture
    /// * `figure_size` - Optional (width, height) in inches. If provided, uses figure-preserving mode.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Without figure preservation (lighter styling)
    /// let frame = capture.capture(&plot)?;
    ///
    /// // With figure preservation (matches static plot styling)
    /// let frame = capture.capture_with_figure(&plot, Some((6.4, 4.8)))?;
    /// ```
    pub fn capture_with_figure(
        &mut self,
        plot: &Plot,
        figure_size: Option<(f32, f32, u32)>, // (width, height, dpi)
    ) -> Result<&[u8]> {
        let sized_plot = if let Some((fig_width, fig_height, dpi)) = figure_size {
            // Use pre-calculated DPI for consistent dimensions
            plot.clone().size(fig_width, fig_height).dpi(dpi)
        } else {
            // Standard behavior: just set pixel dimensions
            plot.clone().size_px(self.width, self.height)
        };

        // Render plot to RGBA buffer
        let image = sized_plot.render()?;
        let rgba_data = &image.pixels;

        // Use actual rendered dimensions
        let actual_pixels = (image.width * image.height) as usize;

        // Resize buffer if needed (safety check - dimensions should match)
        let required_size = actual_pixels * 3;
        if self.buffer.len() != required_size {
            self.buffer.resize(required_size, 0);
            self.width = image.width;
            self.height = image.height;
        }

        // Convert RGBA to RGB
        for i in 0..actual_pixels {
            self.buffer[i * 3] = rgba_data[i * 4]; // R
            self.buffer[i * 3 + 1] = rgba_data[i * 4 + 1]; // G
            self.buffer[i * 3 + 2] = rgba_data[i * 4 + 2]; // B
        }

        Ok(&self.buffer)
    }

    /// Capture with explicit dimensions
    ///
    /// Resizes if necessary before capturing.
    pub fn capture_sized(&mut self, plot: &Plot, width: u32, height: u32) -> Result<&[u8]> {
        self.resize(width, height);
        self.capture(plot)
    }

    /// Get a copy of the buffer (for async encoding)
    pub fn buffer_copy(&self) -> Vec<u8> {
        self.buffer.clone()
    }
}

/// Buffers frames and streams them to an encoder
///
/// `VideoStream` manages the frame-by-frame encoding process,
/// handling encoder initialization and finalization.
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::animation::{VideoStream, VideoConfig, Tick};
///
/// let config = VideoConfig::new().dimensions(800, 600).framerate(30);
/// let mut stream = VideoStream::new("output.gif", config)?;
///
/// // Record frames
/// for tick in ticks {
///     let frame = capture.capture(&plot)?;
///     stream.record_frame(frame, &tick)?;
/// }
///
/// stream.save()?;
/// ```
pub struct VideoStream {
    encoder: Box<dyn Encoder>,
    config: VideoConfig,
    frame_count: u64,
    initialized: bool,
}

impl VideoStream {
    /// Create a new video stream with the given output path and config
    pub fn new<P: AsRef<Path>>(path: P, config: VideoConfig) -> Result<Self> {
        let encoder = create_encoder(path.as_ref(), config.quality)?;

        Ok(Self {
            encoder,
            config,
            frame_count: 0,
            initialized: false,
        })
    }

    /// Create a video stream with auto-detected settings from path
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config = VideoConfig::from_path(&path);
        Self::new(path, config)
    }

    /// Get the video configuration
    pub fn config(&self) -> &VideoConfig {
        &self.config
    }

    /// Get the number of frames recorded
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Record a frame
    ///
    /// On the first call, initializes the encoder with the frame dimensions.
    pub fn record_frame(&mut self, rgb_data: &[u8], tick: &Tick) -> Result<()> {
        // Initialize encoder on first frame
        if !self.initialized {
            self.encoder.init(self.config.width, self.config.height)?;
            self.initialized = true;
        }

        // Calculate timestamp in milliseconds
        let timestamp_ms = (tick.time * 1000.0) as u64;

        self.encoder.encode_frame(rgb_data, timestamp_ms)?;
        self.frame_count += 1;

        Ok(())
    }

    /// Record a frame with explicit dimensions
    ///
    /// Updates the config dimensions if they differ.
    pub fn record_frame_sized(
        &mut self,
        rgb_data: &[u8],
        width: u32,
        height: u32,
        tick: &Tick,
    ) -> Result<()> {
        if !self.initialized {
            self.config.width = width;
            self.config.height = height;
        }
        self.record_frame(rgb_data, tick)
    }

    /// Finalize and save the video
    ///
    /// This must be called to produce valid output.
    pub fn save(self) -> Result<()> {
        if self.frame_count == 0 {
            return Err(PlottingError::RenderError("No frames recorded".into()));
        }
        self.encoder.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_config_default() {
        let config = VideoConfig::default();
        assert_eq!(config.width, 800);
        assert_eq!(config.height, 600);
        assert_eq!(config.framerate, 30);
    }

    #[test]
    fn test_video_config_builder() {
        let config = VideoConfig::new()
            .dimensions(1920, 1080)
            .framerate(60)
            .quality(Quality::High);

        assert_eq!(config.width, 1920);
        assert_eq!(config.height, 1080);
        assert_eq!(config.framerate, 60);
        assert_eq!(config.quality, Quality::High);
    }

    #[test]
    fn test_video_config_from_path() {
        let config = VideoConfig::from_path("test.gif");
        assert_eq!(config.codec, Codec::Gif);

        let config = VideoConfig::from_path("test.mp4");
        assert_eq!(config.codec, Codec::Av1);
    }

    #[test]
    fn test_frame_delay() {
        let config = VideoConfig::new().framerate(30);
        assert_eq!(config.frame_delay_cs(), 3);

        let config = VideoConfig::new().framerate(60);
        assert_eq!(config.frame_delay_cs(), 2);

        let config = VideoConfig::new().framerate(10);
        assert_eq!(config.frame_delay_cs(), 10);
    }

    #[test]
    fn test_frame_capture_new() {
        let capture = FrameCapture::new(100, 50);
        assert_eq!(capture.dimensions(), (100, 50));
        assert_eq!(capture.buffer.len(), 100 * 50 * 3);
    }

    #[test]
    fn test_frame_capture_resize() {
        let mut capture = FrameCapture::new(100, 100);
        capture.resize(200, 150);
        assert_eq!(capture.dimensions(), (200, 150));
        assert_eq!(capture.buffer.len(), 200 * 150 * 3);
    }
}
