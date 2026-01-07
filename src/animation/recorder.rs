//! Animation recording API
//!
//! Provides the main `record()` function and related utilities for
//! creating animated plots.
//!
//! # Simplified API
//!
//! The simplified API reduces boilerplate for common animation tasks:
//!
//! ```rust,ignore
//! use ruviz::animation::{record_simple, DurationExt};
//!
//! // Record 60 frames with just time
//! record_simple("out.gif", 60, |t| {
//!     let x = t.lerp_over(0.0, 10.0, 2.0);
//!     Plot::new().scatter(&[x], &[0.0])
//! })?;
//!
//! // Or use duration syntax
//! record_simple("out.gif", 2.0.secs(), |t| {
//!     Plot::new().line(&[0.0, t.time], &[0.0, t.time])
//! })?;
//! ```

use std::ops::Range;
use std::path::Path;
use std::time::Duration;

use super::encoders::Quality;
use super::stream::{FrameCapture, VideoConfig, VideoStream};
use super::tick::{Tick, TickGenerator};
use crate::core::{Plot, Result};

// ============================================================================
// Simplified API Types
// ============================================================================

/// Trait for types that can specify animation duration/frame count
///
/// Allows `record_simple()` to accept frame counts, durations, or ranges.
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::animation::{record_simple, DurationExt};
///
/// // All of these work:
/// record_simple("out.gif", 60, |t| ...)?;           // 60 frames
/// record_simple("out.gif", 0..60, |t| ...)?;        // Range of frames
/// record_simple("out.gif", 2.0.secs(), |t| ...)?;   // 2 second duration
/// ```
pub trait IntoFrameCount {
    /// Convert to frame count given a framerate
    fn into_frame_count(self, framerate: u32) -> usize;
}

impl IntoFrameCount for usize {
    #[inline]
    fn into_frame_count(self, _framerate: u32) -> usize {
        self
    }
}

impl IntoFrameCount for u32 {
    #[inline]
    fn into_frame_count(self, _framerate: u32) -> usize {
        self as usize
    }
}

impl IntoFrameCount for i32 {
    #[inline]
    fn into_frame_count(self, _framerate: u32) -> usize {
        self.max(0) as usize
    }
}

impl IntoFrameCount for Range<usize> {
    #[inline]
    fn into_frame_count(self, _framerate: u32) -> usize {
        self.len()
    }
}

impl IntoFrameCount for Duration {
    #[inline]
    fn into_frame_count(self, framerate: u32) -> usize {
        (self.as_secs_f64() * framerate as f64).ceil() as usize
    }
}

/// Extension trait for ergonomic duration creation
///
/// # Example
///
/// ```rust
/// use ruviz::animation::DurationExt;
///
/// let duration = 2.5.secs();
/// assert_eq!(duration.as_secs_f64(), 2.5);
/// ```
pub trait DurationExt {
    /// Convert to Duration (seconds)
    fn secs(self) -> Duration;
}

impl DurationExt for f64 {
    #[inline]
    fn secs(self) -> Duration {
        Duration::from_secs_f64(self)
    }
}

impl DurationExt for f32 {
    #[inline]
    fn secs(self) -> Duration {
        Duration::from_secs_f64(self as f64)
    }
}

impl DurationExt for u64 {
    #[inline]
    fn secs(self) -> Duration {
        Duration::from_secs(self)
    }
}

impl DurationExt for i32 {
    #[inline]
    fn secs(self) -> Duration {
        Duration::from_secs(self.max(0) as u64)
    }
}

// ============================================================================
// Original API (preserved for backwards compatibility)
// ============================================================================

/// Configuration for animation recording
#[derive(Clone, Debug)]
pub struct RecordConfig {
    /// Output width in pixels
    pub width: u32,
    /// Output height in pixels
    pub height: u32,
    /// Frames per second
    pub framerate: u32,
    /// Encoding quality
    pub quality: Quality,
    /// Show progress during recording
    pub progress: bool,
    /// Automatically update plot limits per frame
    pub update_limits: bool,
}

impl Default for RecordConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            framerate: 30,
            quality: Quality::Medium,
            progress: false,
            update_limits: false,
        }
    }
}

impl RecordConfig {
    /// Create a new record config with default settings
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

    /// Enable progress reporting
    pub fn with_progress(mut self) -> Self {
        self.progress = true;
        self
    }

    /// Enable automatic limit updates per frame
    pub fn with_auto_limits(mut self) -> Self {
        self.update_limits = true;
        self
    }

    /// Convert to VideoConfig
    fn to_video_config(&self) -> VideoConfig {
        VideoConfig {
            width: self.width,
            height: self.height,
            framerate: self.framerate,
            quality: self.quality,
            ..Default::default()
        }
    }
}

/// Record an animation by iterating over frames
///
/// This is the primary API for creating animations. It iterates over the
/// provided frames, calling the frame function for each to generate a plot,
/// then captures and encodes each frame.
///
/// # Type Parameters
///
/// * `P` - Output path type (implements `AsRef<Path>`)
/// * `I` - Iterator type for frame data
/// * `F` - Frame function type
///
/// # Arguments
///
/// * `path` - Output file path (format detected from extension)
/// * `frames` - Iterator of frame data (e.g., `0..60` for 60 frames)
/// * `frame_fn` - Function that produces a Plot for each frame
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::prelude::*;
/// use ruviz::animation::record;
///
/// let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
///
/// record("wave.gif", 0..60, |frame, tick| {
///     let phase = tick.time * 2.0 * std::f64::consts::PI;
///     let y: Vec<f64> = x.iter().map(|&xi| (xi + phase).sin()).collect();
///     #[allow(deprecated)]
///     Plot::new()
///         .line(&x, &y)
///         .end_series()
///         .title(format!("Frame {}", frame))
/// }).unwrap();
/// ```
#[deprecated(
    since = "0.9.0",
    note = "Use the record! macro instead: `record!(path, frames, |t| plot)`"
)]
pub fn record<P, I, F, R>(path: P, frames: I, mut frame_fn: F) -> Result<()>
where
    P: AsRef<Path>,
    I: IntoIterator,
    F: FnMut(I::Item, &Tick) -> R,
    R: Into<Plot>,
{
    let config = RecordConfig::default();
    record_with_config(path, frames, config, frame_fn)
}

/// Record an animation with explicit configuration
///
/// Like `record()`, but allows specifying dimensions, framerate, quality, etc.
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::prelude::*;
/// use ruviz::animation::{record_with_config, RecordConfig, Quality};
///
/// let config = RecordConfig::new()
///     .dimensions(1920, 1080)
///     .framerate(60)
///     .quality(Quality::High);
///
/// record_with_config("output.gif", 0..120, config, |frame, tick| {
///     #[allow(deprecated)]
///     Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]).end_series()
/// }).unwrap();
/// ```
#[deprecated(
    since = "0.9.0",
    note = "Use the record! macro instead: `record!(path, frames, config: cfg, |t| plot)`"
)]
pub fn record_with_config<P, I, F, R>(
    path: P,
    frames: I,
    config: RecordConfig,
    mut frame_fn: F,
) -> Result<()>
where
    P: AsRef<Path>,
    I: IntoIterator,
    F: FnMut(I::Item, &Tick) -> R,
    R: Into<Plot>,
{
    let video_config = config.to_video_config();
    let mut stream = VideoStream::new(&path, video_config)?;
    let mut capture = FrameCapture::new(config.width, config.height);
    let mut ticker = TickGenerator::new(config.framerate as f64);

    let frames_iter = frames.into_iter();

    // Try to get size hint for progress reporting
    let _size_hint = frames_iter.size_hint();

    for item in frames_iter {
        let tick = ticker.tick_immediate();
        let plot: Plot = frame_fn(item, &tick).into();
        let frame_data = capture.capture(&plot)?;
        stream.record_frame(frame_data, &tick)?;
    }

    stream.save()
}

/// Record an animation for a specific duration
///
/// Generates frames at the specified framerate for the given duration.
///
/// # Arguments
///
/// * `path` - Output file path
/// * `duration_secs` - Total animation duration in seconds
/// * `framerate` - Frames per second
/// * `frame_fn` - Function that produces a Plot for each tick
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::prelude::*;
/// use ruviz::animation::record_duration;
///
/// record_duration("sim.gif", 5.0, 30, |tick| {
///     let t = tick.time;
///     let y = (t * 2.0 * std::f64::consts::PI).sin();
///     #[allow(deprecated)]
///     Plot::new().line(&[0.0, t], &[0.0, y]).end_series()
/// }).unwrap();
/// ```
#[deprecated(
    since = "0.9.0",
    note = "Use the record! macro instead: `record!(path, duration secs, |t| plot)`"
)]
pub fn record_duration<P, F, R>(
    path: P,
    duration_secs: f64,
    framerate: u32,
    mut frame_fn: F,
) -> Result<()>
where
    P: AsRef<Path>,
    F: FnMut(&Tick) -> R,
    R: Into<Plot>,
{
    let total_frames = (duration_secs * framerate as f64).ceil() as usize;

    record(path, 0..total_frames, |_, tick| frame_fn(tick))
}

/// Record an animation with duration and explicit config
#[deprecated(
    since = "0.9.0",
    note = "Use the record! macro instead: `record!(path, duration secs, config: cfg, |t| plot)`"
)]
pub fn record_duration_with_config<P, F, R>(
    path: P,
    duration_secs: f64,
    config: RecordConfig,
    mut frame_fn: F,
) -> Result<()>
where
    P: AsRef<Path>,
    F: FnMut(&Tick) -> R,
    R: Into<Plot>,
{
    let total_frames = (duration_secs * config.framerate as f64).ceil() as usize;

    record_with_config(path, 0..total_frames, config, |_, tick| frame_fn(tick))
}

/// Record an animation driven by AnimatedObservable changes
///
/// This function records frames while any animation in the group is active,
/// advancing by the frame delta each iteration. Useful for recording smooth
/// value transitions.
///
/// # Arguments
///
/// * `path` - Output file path
/// * `animations` - Animation group containing animated observables
/// * `max_frames` - Maximum number of frames to record (safety limit)
/// * `frame_fn` - Function that produces a Plot for each tick
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::prelude::*;
/// use ruviz::animation::{record_animated, AnimatedObservable, AnimationGroup};
///
/// let x = AnimatedObservable::new(0.0);
/// let mut group = AnimationGroup::new();
/// group.add(&x);
///
/// x.animate_to(100.0, 2000); // 2 second animation
///
/// record_animated("output.gif", &group, 120, |tick| {
///     let val = x.get();
///     Plot::new().line(&[0.0, val], &[0.0, val]).end_series()
/// }).unwrap();
/// ```
#[deprecated(
    since = "0.9.0",
    note = "Use record_simple() or the record! macro with Signal-based animations instead"
)]
pub fn record_animated<'a, P, F, R>(
    path: P,
    animations: &super::observable_ext::AnimationGroup<'a>,
    max_frames: usize,
    frame_fn: F,
) -> Result<()>
where
    P: AsRef<Path>,
    F: FnMut(&Tick) -> R,
    R: Into<Plot>,
{
    let config = RecordConfig::default();
    record_animated_with_config(path, animations, max_frames, config, frame_fn)
}

/// Record animated observables with explicit configuration
#[deprecated(
    since = "0.9.0",
    note = "Use record_simple_with_config() or the record! macro with Signal-based animations instead"
)]
pub fn record_animated_with_config<'a, P, F, R>(
    path: P,
    animations: &super::observable_ext::AnimationGroup<'a>,
    max_frames: usize,
    config: RecordConfig,
    mut frame_fn: F,
) -> Result<()>
where
    P: AsRef<Path>,
    F: FnMut(&Tick) -> R,
    R: Into<Plot>,
{
    let video_config = config.to_video_config();
    let mut stream = VideoStream::new(&path, video_config)?;
    let mut capture = FrameCapture::new(config.width, config.height);
    let mut ticker = TickGenerator::new(config.framerate as f64);

    let delta_time = 1.0 / config.framerate as f64;
    let mut frame_count = 0;

    // Record frames while animations are active (or until max_frames)
    loop {
        // Stop if we've hit the frame limit
        if frame_count >= max_frames {
            break;
        }

        // Tick all animations
        let still_animating = animations.tick(delta_time);

        // Generate tick for this frame
        let tick = ticker.tick_immediate();

        // Render frame
        let plot: Plot = frame_fn(&tick).into();
        let frame_data = capture.capture(&plot)?;
        stream.record_frame(frame_data, &tick)?;

        frame_count += 1;

        // Stop if all animations have completed
        if !still_animating {
            break;
        }
    }

    stream.save()
}

// ============================================================================
// Simplified API Functions
// ============================================================================

/// Simplified animation recording with minimal boilerplate
///
/// This is the recommended API for most animation use cases. It accepts
/// frame counts, ranges, or durations, and provides tick helpers for
/// easy value interpolation.
///
/// # Arguments
///
/// * `path` - Output file path (format detected from extension)
/// * `frames` - Frame count, range, or duration (via `IntoFrameCount`)
/// * `frame_fn` - Function that produces a Plot for each tick
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::animation::{record_simple, DurationExt, easing};
///
/// // Using frame count
/// record_simple("bounce.gif", 60, |t| {
///     let y = t.ease_over(easing::ease_out_bounce, 100.0, 0.0, 2.0);
///     Plot::new().scatter(&[0.0], &[y])
/// })?;
///
/// // Using duration
/// record_simple("wave.gif", 2.0.secs(), |t| {
///     let phase = t.time * 2.0 * std::f64::consts::PI;
///     let y: Vec<f64> = (0..100).map(|i| {
///         let x = i as f64 * 0.1;
///         (x + phase).sin()
///     }).collect();
///     let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
///     Plot::new().line(&x, &y)
/// })?;
/// ```
#[deprecated(
    since = "0.9.0",
    note = "Use the record! macro instead: `record!(path, duration secs, |t| plot)`"
)]
pub fn record_simple<P, D, F, R>(path: P, frames: D, mut frame_fn: F) -> Result<()>
where
    P: AsRef<Path>,
    D: IntoFrameCount,
    F: FnMut(&Tick) -> R,
    R: Into<Plot>,
{
    let config = RecordConfig::default();
    record_simple_with_config(path, frames, config, frame_fn)
}

/// Simplified animation recording with configuration
///
/// Like `record_simple()`, but allows specifying dimensions, framerate, etc.
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::animation::{record_simple_with_config, RecordConfig, DurationExt};
///
/// let config = RecordConfig::new()
///     .dimensions(1920, 1080)
///     .framerate(60);
///
/// record_simple_with_config("hd.gif", 3.0.secs(), config, |t| {
///     Plot::new().line(&[0.0, t.time], &[0.0, t.time])
/// })?;
/// ```
#[deprecated(
    since = "0.9.0",
    note = "Use the record! macro instead: `record!(path, duration secs, config: cfg, |t| plot)`"
)]
pub fn record_simple_with_config<P, D, F, R>(
    path: P,
    frames: D,
    config: RecordConfig,
    mut frame_fn: F,
) -> Result<()>
where
    P: AsRef<Path>,
    D: IntoFrameCount,
    F: FnMut(&Tick) -> R,
    R: Into<Plot>,
{
    let total_frames = frames.into_frame_count(config.framerate);

    let video_config = config.to_video_config();
    let mut stream = VideoStream::new(&path, video_config)?;
    let mut capture = FrameCapture::new(config.width, config.height);
    let mut ticker = TickGenerator::new(config.framerate as f64);

    for _ in 0..total_frames {
        let tick = ticker.tick_immediate();
        let plot: Plot = frame_fn(&tick).into();
        let frame_data = capture.capture(&plot)?;
        stream.record_frame(frame_data, &tick)?;
    }

    stream.save()
}

// ============================================================================
// Reactive Plot Recording
// ============================================================================

/// Record a reactive plot to an animation file.
///
/// Unlike `record_simple` which creates a new Plot per frame via closure,
/// this function takes a pre-built Plot and renders it at each frame time.
/// Reactive data (Signal/Observable) in the plot is resolved at each frame.
///
/// # Arguments
///
/// * `path` - Output file path (format detected from extension)
/// * `plot` - The plot to record (can contain reactive data)
/// * `duration` - Total animation duration in seconds
/// * `fps` - Frames per second
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::prelude::*;
/// use ruviz::animation::{record_plot, signal};
///
/// // Create reactive plot ONCE
/// let y_signal = signal::of(|t| (0..100).map(|i| (i as f64 * 0.1 + t).sin()).collect());
/// let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
///
/// let plot = Plot::new()
///     .line(&x, y_signal)
///     .title("Reactive Wave");
///
/// // Record - plot structure unchanged, only data resolved each frame
/// record_plot("wave.gif", &plot, 2.0, 30)?;
/// ```
pub fn record_plot<P: AsRef<Path>>(path: P, plot: &Plot, duration: f64, fps: u32) -> Result<()> {
    let config = RecordConfig::default().framerate(fps);
    record_plot_with_config(path, plot, duration, config)
}

/// Record a reactive plot with explicit configuration.
///
/// Like `record_plot()`, but allows specifying dimensions, quality, etc.
pub fn record_plot_with_config<P: AsRef<Path>>(
    path: P,
    plot: &Plot,
    duration: f64,
    config: RecordConfig,
) -> Result<()> {
    let frame_count = (duration * config.framerate as f64).ceil() as usize;
    if frame_count == 0 {
        return Err(crate::core::PlottingError::InvalidInput(
            "Duration too short for any frames".to_string(),
        ));
    }

    let frame_duration = 1.0 / config.framerate as f64;

    let video_config = config.to_video_config();
    let mut stream = VideoStream::new(&path, video_config)?;
    let (width, height) = (config.width, config.height);
    let mut capture = FrameCapture::new(width, height);
    let mut ticker = TickGenerator::new(config.framerate as f64);

    for frame in 0..frame_count {
        let time = frame as f64 * frame_duration;
        let tick = ticker.tick_immediate();

        // Render plot at this time (resolves reactive data)
        let sized_plot = plot.clone().size_px(width, height);
        let image = sized_plot.render_at(time)?;

        // Convert to frame data and record
        stream.record_frame_sized(&image.pixels, width, height, &tick)?;
    }

    stream.save()
}

// ============================================================================
// Internal Functions for record! Macro
// ============================================================================

/// Internal: Record animation with frame count (for record! macro)
#[doc(hidden)]
pub fn _record_frames<P, F, R>(path: P, frames: impl IntoFrameCount, frame_fn: F) -> Result<()>
where
    P: AsRef<Path>,
    F: FnMut(&Tick) -> R,
    R: Into<Plot>,
{
    record_simple(path, frames, frame_fn)
}

/// Internal: Record animation with duration in seconds (for record! macro)
#[doc(hidden)]
pub fn _record_duration<P, F, R>(path: P, secs: f64, frame_fn: F) -> Result<()>
where
    P: AsRef<Path>,
    F: FnMut(&Tick) -> R,
    R: Into<Plot>,
{
    let config = RecordConfig::default();
    let frames = (secs * config.framerate as f64).ceil() as usize;
    record_simple(path, frames, frame_fn)
}

/// Internal: Record animation with duration and custom fps (for record! macro)
#[doc(hidden)]
pub fn _record_duration_fps<P, F, R>(path: P, secs: f64, fps: u32, frame_fn: F) -> Result<()>
where
    P: AsRef<Path>,
    F: FnMut(&Tick) -> R,
    R: Into<Plot>,
{
    let config = RecordConfig::default().framerate(fps);
    let frames = (secs * fps as f64).ceil() as usize;
    record_simple_with_config(path, frames, config, frame_fn)
}

/// Internal: Record animation with frame count and config (for record! macro)
#[doc(hidden)]
pub fn _record_frames_config<P, F, R>(
    path: P,
    frames: impl IntoFrameCount,
    config: RecordConfig,
    frame_fn: F,
) -> Result<()>
where
    P: AsRef<Path>,
    F: FnMut(&Tick) -> R,
    R: Into<Plot>,
{
    record_simple_with_config(path, frames, config, frame_fn)
}

/// Internal: Record reactive plot (for record! macro)
#[doc(hidden)]
pub fn _record_reactive<P: AsRef<Path>>(path: P, plot: &Plot, secs: f64, fps: u32) -> Result<()> {
    record_plot(path, plot, secs, fps)
}

/// Internal: Record reactive plot with config (for record! macro)
#[doc(hidden)]
pub fn _record_reactive_config<P: AsRef<Path>>(
    path: P,
    plot: &Plot,
    secs: f64,
    config: RecordConfig,
) -> Result<()> {
    record_plot_with_config(path, plot, secs, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_record_config_default() {
        let config = RecordConfig::default();
        assert_eq!(config.width, 800);
        assert_eq!(config.height, 600);
        assert_eq!(config.framerate, 30);
    }

    #[test]
    fn test_record_config_builder() {
        let config = RecordConfig::new()
            .dimensions(1920, 1080)
            .framerate(60)
            .quality(Quality::High)
            .with_progress()
            .with_auto_limits();

        assert_eq!(config.width, 1920);
        assert_eq!(config.height, 1080);
        assert_eq!(config.framerate, 60);
        assert!(config.progress);
        assert!(config.update_limits);
    }

    #[test]
    fn test_record_basic() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.gif");

        let result = record(&path, 0..3, |frame, _tick| {
            #[allow(deprecated)]
            Plot::new()
                .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 0.5])
                .end_series()
                .title(format!("Frame {}", frame))
        });

        assert!(result.is_ok(), "Recording failed: {:?}", result.err());
        assert!(path.exists(), "Output file not created");
    }

    #[test]
    fn test_record_with_config() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_config.gif");

        let config = RecordConfig::new()
            .dimensions(200, 150)
            .framerate(10)
            .quality(Quality::Low);

        let result = record_with_config(&path, 0..2, config, |_, _| {
            #[allow(deprecated)]
            Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]).end_series()
        });

        assert!(result.is_ok());
        assert!(path.exists());
    }

    #[test]
    fn test_record_duration() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_duration.gif");

        let result = record_duration(&path, 0.1, 10, |tick| {
            #[allow(deprecated)]
            Plot::new()
                .line(&[0.0, tick.time], &[0.0, tick.time])
                .end_series()
        });

        assert!(result.is_ok());
        assert!(path.exists());
    }

    #[test]
    fn test_record_empty_frames() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_empty.gif");

        // Empty iterator should produce error (no frames)
        let result = record(&path, 0..0, |_, _| {
            #[allow(deprecated)]
            Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]).end_series()
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_tick_values_in_record() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_ticks.gif");

        let mut observed_ticks = Vec::new();

        let _ = record(&path, 0..5, |frame, tick| {
            observed_ticks.push((frame, tick.count, tick.time));
            #[allow(deprecated)]
            Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]).end_series()
        });

        // Check tick values are correct
        assert_eq!(observed_ticks.len(), 5);
        for (i, (frame, count, _time)) in observed_ticks.iter().enumerate() {
            assert_eq!(*frame, i);
            assert_eq!(*count, i as u64);
        }
    }

    // ========== Simplified API Tests ==========

    #[test]
    fn test_into_frame_count_usize() {
        assert_eq!(60_usize.into_frame_count(30), 60);
    }

    #[test]
    fn test_into_frame_count_u32() {
        assert_eq!(60_u32.into_frame_count(30), 60);
    }

    #[test]
    fn test_into_frame_count_range() {
        assert_eq!((0..60_usize).into_frame_count(30), 60);
    }

    #[test]
    fn test_into_frame_count_duration() {
        use std::time::Duration;
        // 2 seconds at 30 FPS = 60 frames
        assert_eq!(Duration::from_secs(2).into_frame_count(30), 60);
        // 2.5 seconds at 30 FPS = 75 frames (ceiling)
        assert_eq!(Duration::from_secs_f64(2.5).into_frame_count(30), 75);
    }

    #[test]
    fn test_duration_ext_f64() {
        let d = 2.5.secs();
        assert!((d.as_secs_f64() - 2.5).abs() < 1e-10);
    }

    #[test]
    fn test_duration_ext_u64() {
        let d = 3_u64.secs();
        assert_eq!(d.as_secs(), 3);
    }

    #[test]
    fn test_record_simple_with_frame_count() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_simple.gif");

        let mut frame_count = 0;
        let result = record_simple(&path, 5_usize, |_tick| {
            frame_count += 1;
            #[allow(deprecated)]
            Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]).end_series()
        });

        assert!(result.is_ok());
        assert!(path.exists());
        assert_eq!(frame_count, 5);
    }

    #[test]
    fn test_record_simple_with_duration() {
        use std::time::Duration;
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_simple_duration.gif");

        let config = RecordConfig::new().framerate(10);
        let mut frame_count = 0;

        // 0.3 seconds at 10 FPS = 3 frames (ceiling)
        let result =
            record_simple_with_config(&path, Duration::from_secs_f64(0.3), config, |_tick| {
                frame_count += 1;
                #[allow(deprecated)]
                Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]).end_series()
            });

        assert!(result.is_ok());
        assert!(path.exists());
        assert_eq!(frame_count, 3);
    }

    #[test]
    fn test_record_simple_with_tick_helpers() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_simple_helpers.gif");

        let config = RecordConfig::new().framerate(10);
        let mut values = Vec::new();

        let result = record_simple_with_config(&path, 10_usize, config, |tick| {
            let x = tick.lerp_over(0.0, 100.0, 1.0);
            values.push(x);
            #[allow(deprecated)]
            Plot::new().scatter(&[x], &[0.0]).end_series()
        });

        assert!(result.is_ok());
        // At 10 FPS, 10 frames = 1 second
        // Values should go from 0 to 90 (at t=0.9)
        assert_eq!(values.len(), 10);
        assert!((values[0] - 0.0).abs() < 1e-10);
        assert!((values[5] - 50.0).abs() < 1e-10);
    }
}
