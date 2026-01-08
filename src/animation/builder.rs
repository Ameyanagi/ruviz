//! Animation builder for declarative multi-value animations
//!
//! Provides `Animation::build()` for creating complex animations
//! without manual Observable management.
//!
//! # Example
//!
//! ```rust,ignore
//! use ruviz::animation::{Animation, easing};
//!
//! Animation::build()
//!     .value("x", 0.0).to(100.0).duration_secs(2.0)
//!     .value("y", 0.0).to(50.0).ease(easing::ease_out_bounce)
//!     .record("output.gif", |values, tick| {
//!         let x = values["x"];
//!         let y = values["y"];
//!         Plot::new().scatter(&[x], &[y])
//!     })?;
//! ```

use std::collections::HashMap;
use std::ops::Index;
use std::path::Path;

use super::interpolation::{EasingFn, easing};
#[allow(deprecated)]
use super::recorder::{IntoFrameCount, RecordConfig, record_simple_with_config};
use super::tick::Tick;
use crate::core::{Plot, Result};

/// A single animated value configuration
#[derive(Clone)]
pub struct AnimatedValue {
    name: String,
    start: f64,
    end: f64,
    duration_secs: f64,
    easing: EasingFn,
}

impl AnimatedValue {
    fn new(name: String, start: f64) -> Self {
        Self {
            name,
            start,
            end: start,
            duration_secs: 1.0,
            easing: easing::linear,
        }
    }

    /// Get current value at given time
    fn value_at(&self, time: f64) -> f64 {
        let progress = (time / self.duration_secs).clamp(0.0, 1.0);
        let eased = (self.easing)(progress);
        self.start + (self.end - self.start) * eased
    }
}

/// Runtime access to animated values during recording
///
/// Provides both `get()` method and `Index<&str>` for convenient access.
///
/// # Example
///
/// ```rust,ignore
/// // Using get()
/// let x = values.get("x");
///
/// // Using index operator
/// let y = values["y"];
/// ```
pub struct AnimationValues {
    values: HashMap<String, f64>,
}

impl AnimationValues {
    fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    fn set(&mut self, name: &str, value: f64) {
        self.values.insert(name.to_string(), value);
    }

    /// Get value by name, returns 0.0 if not found
    pub fn get(&self, name: &str) -> f64 {
        self.values.get(name).copied().unwrap_or(0.0)
    }
}

impl Index<&str> for AnimationValues {
    type Output = f64;

    fn index(&self, name: &str) -> &Self::Output {
        self.values.get(name).unwrap_or(&0.0)
    }
}

/// Builder for a single animated value
///
/// Created via `AnimationBuilder::value()`. All methods return `Self` for chaining.
/// When done configuring the value, call another `value()` or `build()`/`record()`.
pub struct ValueBuilder<'a> {
    builder: &'a mut AnimationBuilder,
    index: usize,
}

impl<'a> ValueBuilder<'a> {
    /// Set the target value
    pub fn to(self, end: f64) -> Self {
        self.builder.values[self.index].end = end;
        self
    }

    /// Set custom duration for this value
    pub fn duration_secs(self, secs: f64) -> Self {
        self.builder.values[self.index].duration_secs = secs;
        self
    }

    /// Set easing function for this value
    pub fn ease(self, easing_fn: EasingFn) -> Self {
        self.builder.values[self.index].easing = easing_fn;
        self
    }

    /// Add another animated value
    pub fn value(self, name: &str, start: f64) -> ValueBuilder<'a> {
        self.builder
            .values
            .push(AnimatedValue::new(name.to_string(), start));
        let index = self.builder.values.len() - 1;
        ValueBuilder {
            builder: self.builder,
            index,
        }
    }

    /// Set recording configuration
    pub fn config(self, config: RecordConfig) -> ValueBuilder<'a> {
        self.builder.config = Some(config);
        self
    }

    /// Build the Animation
    pub fn build(self) -> Animation {
        Animation {
            values: std::mem::take(&mut self.builder.values),
            config: self.builder.config.take().unwrap_or_default(),
        }
    }

    /// Direct record without explicit build
    pub fn record<P, F, R>(self, path: P, frame_fn: F) -> Result<()>
    where
        P: AsRef<Path>,
        F: FnMut(&AnimationValues, &Tick) -> R,
        R: Into<Plot>,
    {
        self.build().record(path, frame_fn)
    }
}

/// Builder for creating multi-value animations
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::animation::{Animation, easing};
///
/// let anim = Animation::build()
///     .value("x", 0.0).to(100.0).duration_secs(2.0)
///     .value("opacity", 1.0).to(0.0).ease(easing::ease_out_quad)
///     .build();
/// ```
#[derive(Default)]
pub struct AnimationBuilder {
    values: Vec<AnimatedValue>,
    config: Option<RecordConfig>,
}

impl AnimationBuilder {
    /// Create a new animation builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an animated value
    pub fn value(&mut self, name: &str, start: f64) -> ValueBuilder<'_> {
        self.values
            .push(AnimatedValue::new(name.to_string(), start));
        let index = self.values.len() - 1;
        ValueBuilder {
            builder: self,
            index,
        }
    }

    /// Set recording configuration
    pub fn config(mut self, config: RecordConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Build the Animation
    pub fn build(self) -> Animation {
        Animation {
            values: self.values,
            config: self.config.unwrap_or_default(),
        }
    }

    /// Direct record without explicit build
    ///
    /// Convenience method that builds and records in one call.
    pub fn record<P, F>(self, path: P, frame_fn: F) -> Result<()>
    where
        P: AsRef<Path>,
        F: FnMut(&AnimationValues, &Tick) -> Plot,
    {
        self.build().record(path, frame_fn)
    }
}

/// Completed animation ready for recording
pub struct Animation {
    values: Vec<AnimatedValue>,
    config: RecordConfig,
}

impl Animation {
    /// Start building an animation
    pub fn build() -> AnimationBuilder {
        AnimationBuilder::new()
    }

    /// Get the total duration (longest value animation)
    pub fn total_duration(&self) -> f64 {
        self.values
            .iter()
            .map(|v| v.duration_secs)
            .fold(0.0, f64::max)
    }

    /// Record the animation to a file
    ///
    /// # Arguments
    ///
    /// * `path` - Output file path
    /// * `frame_fn` - Function receiving current values and tick, returning a Plot
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Animation::build()
    ///     .value("x", 0.0).to(100.0)
    ///     .build()
    ///     .record("anim.gif", |values, tick| {
    ///         Plot::new().scatter(&[values["x"]], &[0.0])
    ///     })?;
    /// ```
    #[allow(deprecated)]
    pub fn record<P, F, R>(self, path: P, mut frame_fn: F) -> Result<()>
    where
        P: AsRef<Path>,
        F: FnMut(&AnimationValues, &Tick) -> R,
        R: Into<Plot>,
    {
        let duration = self.total_duration();
        let values_config = self.values.clone();
        let config = self.config.clone();

        // Calculate frame count from duration
        let frame_count = (duration * config.framerate as f64).ceil() as usize;

        // Ensure at least 1 frame
        let frame_count = frame_count.max(1);

        record_simple_with_config(&path, frame_count, config, |tick| {
            // Compute all animated values at current time
            let mut anim_values = AnimationValues::new();
            for v in &values_config {
                anim_values.set(&v.name, v.value_at(tick.time));
            }

            frame_fn(&anim_values, tick)
        })
    }

    /// Record with explicit frame count or duration
    #[allow(deprecated)]
    pub fn record_frames<P, D, F, R>(self, path: P, frames: D, mut frame_fn: F) -> Result<()>
    where
        P: AsRef<Path>,
        D: IntoFrameCount,
        F: FnMut(&AnimationValues, &Tick) -> R,
        R: Into<Plot>,
    {
        let values_config = self.values.clone();
        let config = self.config.clone();
        let frame_count = frames.into_frame_count(config.framerate);

        record_simple_with_config(&path, frame_count, config, |tick| {
            let mut anim_values = AnimationValues::new();
            for v in &values_config {
                anim_values.set(&v.name, v.value_at(tick.time));
            }

            frame_fn(&anim_values, tick)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_animated_value_linear() {
        let val = AnimatedValue {
            name: "x".to_string(),
            start: 0.0,
            end: 100.0,
            duration_secs: 1.0,
            easing: easing::linear,
        };

        assert!((val.value_at(0.0) - 0.0).abs() < 1e-10);
        assert!((val.value_at(0.5) - 50.0).abs() < 1e-10);
        assert!((val.value_at(1.0) - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_animated_value_clamping() {
        let val = AnimatedValue {
            name: "x".to_string(),
            start: 0.0,
            end: 100.0,
            duration_secs: 1.0,
            easing: easing::linear,
        };

        // Should clamp at boundaries
        assert!((val.value_at(-1.0) - 0.0).abs() < 1e-10);
        assert!((val.value_at(2.0) - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_animated_value_with_easing() {
        let val = AnimatedValue {
            name: "x".to_string(),
            start: 0.0,
            end: 100.0,
            duration_secs: 1.0,
            easing: easing::ease_in_quad,
        };

        // ease_in_quad(0.5) = 0.25
        assert!((val.value_at(0.5) - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_animation_values_get() {
        let mut values = AnimationValues::new();
        values.set("x", 42.0);
        values.set("y", 100.0);

        assert!((values.get("x") - 42.0).abs() < 1e-10);
        assert!((values.get("y") - 100.0).abs() < 1e-10);
        assert!((values.get("z") - 0.0).abs() < 1e-10); // default
    }

    #[test]
    fn test_animation_values_index() {
        let mut values = AnimationValues::new();
        values.set("x", 42.0);

        assert!((values["x"] - 42.0).abs() < 1e-10);
    }

    #[test]
    fn test_animation_builder_single_value() {
        let anim = Animation::build().value("x", 0.0).to(100.0).build();

        assert_eq!(anim.values.len(), 1);
        assert_eq!(anim.values[0].name, "x");
        assert!((anim.values[0].start - 0.0).abs() < 1e-10);
        assert!((anim.values[0].end - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_animation_builder_multiple_values() {
        let anim = Animation::build()
            .value("x", 0.0)
            .to(100.0)
            .value("y", 10.0)
            .to(20.0)
            .value("opacity", 1.0)
            .to(0.0)
            .build();

        assert_eq!(anim.values.len(), 3);
        assert_eq!(anim.values[0].name, "x");
        assert_eq!(anim.values[1].name, "y");
        assert_eq!(anim.values[2].name, "opacity");
    }

    #[test]
    fn test_animation_builder_with_duration() {
        let anim = Animation::build()
            .value("x", 0.0)
            .to(100.0)
            .duration_secs(2.0)
            .build();

        assert!((anim.values[0].duration_secs - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_animation_builder_with_easing() {
        let anim = Animation::build()
            .value("x", 0.0)
            .to(100.0)
            .ease(easing::ease_out_quad)
            .build();

        // Verify easing is applied correctly
        let val = &anim.values[0];
        // ease_out_quad(0.5) = 0.75
        assert!((val.value_at(0.5) - 75.0).abs() < 1e-10);
    }

    #[test]
    fn test_animation_total_duration() {
        let anim = Animation::build()
            .value("x", 0.0)
            .to(100.0)
            .duration_secs(2.0)
            .value("y", 0.0)
            .to(50.0)
            .duration_secs(1.0)
            .build();

        assert!((anim.total_duration() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_animation_record() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_builder.gif");

        let mut observed_values = Vec::new();

        let result = Animation::build()
            .value("x", 0.0)
            .to(100.0)
            .duration_secs(0.1)
            .config(RecordConfig::new().framerate(10))
            .record(&path, |values, _tick| {
                observed_values.push(values["x"]);
                #[allow(deprecated)]
                Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]).end_series()
            });

        assert!(result.is_ok(), "Recording failed: {:?}", result.err());
        assert!(path.exists());

        // At 10 FPS for 0.1 seconds, we should have 1 frame (ceiling)
        assert!(!observed_values.is_empty());
    }

    #[test]
    fn test_animation_multi_value_record() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_multi.gif");

        let mut x_values = Vec::new();
        let mut y_values = Vec::new();

        let result = Animation::build()
            .value("x", 0.0)
            .to(100.0)
            .duration_secs(0.3)
            .value("y", 50.0)
            .to(0.0)
            .duration_secs(0.3)
            .config(RecordConfig::new().framerate(10))
            .record(&path, |values, _tick| {
                x_values.push(values["x"]);
                y_values.push(values["y"]);
                #[allow(deprecated)]
                Plot::new()
                    .scatter(&[values["x"]], &[values["y"]])
                    .end_series()
            });

        assert!(result.is_ok());
        assert!(path.exists());

        // Check that x increases and y decreases
        if x_values.len() > 1 {
            assert!(x_values.last() > x_values.first());
            assert!(y_values.last() < y_values.first());
        }
    }

    #[test]
    fn test_animation_with_different_easings() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_easings.gif");

        let result = Animation::build()
            .value("bounce", 100.0)
            .to(0.0)
            .ease(easing::ease_out_bounce)
            .duration_secs(0.2)
            .value("linear", 0.0)
            .to(100.0)
            .duration_secs(0.2)
            .config(RecordConfig::new().framerate(10))
            .record(&path, |values, _tick| {
                #[allow(deprecated)]
                Plot::new()
                    .scatter(&[values["linear"]], &[values["bounce"]])
                    .end_series()
            });

        assert!(result.is_ok());
    }
}
