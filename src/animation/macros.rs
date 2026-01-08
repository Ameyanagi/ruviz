//! Unified record! macro for animation recording
//!
//! This module provides the `record!` macro which unifies all animation
//! recording patterns into a single, ergonomic API.
//!
//! # Patterns
//!
//! ```rust,ignore
//! use ruviz::record;
//!
//! // Frame count
//! record!("out.gif", 60, |t| Plot::new().line(&x, &y))?;
//!
//! // Duration in seconds
//! record!("out.gif", 2 secs, |t| Plot::new().scatter(&x, &y))?;
//!
//! // Duration in milliseconds
//! record!("out.gif", 500 ms, |t| Plot::new().bar(&x, &y))?;
//!
//! // Custom framerate
//! record!("out.gif", 2 secs @ 60 fps, |t| Plot::new().line(&x, &y))?;
//!
//! // With configuration
//! let config = RecordConfig::new().dimensions(1920, 1080);
//! record!("out.gif", 60, config: config, |t| Plot::new().line(&x, &y))?;
//!
//! // Reactive plot (no closure needed)
//! record!("out.gif", &reactive_plot, 2 secs)?;
//! record!("out.gif", &reactive_plot, 2 secs @ 30 fps)?;
//! ```

/// Unified macro for animation recording.
///
/// This macro provides a single entry point for all animation recording patterns,
/// replacing the multiple `record_*` functions with a cleaner, more consistent API.
///
/// # Patterns
///
/// ## Closure-based (Plot created per frame)
///
/// ```rust,ignore
/// // Frame count (60 frames at default 30fps)
/// record!("out.gif", 60, |t| Plot::new().line(&x, &y))?;
///
/// // Duration in seconds (default 30fps)
/// record!("out.gif", 2 secs, |t| Plot::new().scatter(&x, &y))?;
///
/// // Duration in milliseconds
/// record!("out.gif", 500 ms, |t| Plot::new().bar(&x, &y))?;
///
/// // Custom framerate
/// record!("out.gif", 2 secs @ 60 fps, |t| Plot::new().line(&x, &y))?;
///
/// // With configuration
/// let config = RecordConfig::new().dimensions(1920, 1080);
/// record!("out.gif", 60, config: config, |t| Plot::new().line(&x, &y))?;
/// ```
///
/// ## Reactive (Plot created once)
///
/// ```rust,ignore
/// let y_signal = signal::of(|t| compute_y(t));
/// let plot = Plot::new().line(&x, y_signal);
///
/// // Reactive with duration
/// record!("out.gif", &plot, 2 secs)?;
///
/// // Reactive with custom fps
/// record!("out.gif", &plot, 2 secs @ 30 fps)?;
///
/// // Reactive with config
/// let config = RecordConfig::new().dimensions(1920, 1080);
/// record!("out.gif", &plot, 2 secs, config: config)?;
/// ```
#[macro_export]
macro_rules! record {
    // =========== Closure-based (creates Plot per frame) ===========

    // Frame count: record!("out.gif", 60, |t| plot)
    ($path:expr, $frames:expr, $closure:expr) => {
        $crate::animation::_record_frames($path, $frames, $closure)
    };

    // Duration literal (seconds): record!("out.gif", 2 secs, |t| plot)
    ($path:expr, $duration:tt secs, $closure:expr) => {
        $crate::animation::_record_duration($path, $duration as f64, $closure)
    };

    // Duration literal (milliseconds): record!("out.gif", 500 ms, |t| plot)
    ($path:expr, $duration:tt ms, $closure:expr) => {
        $crate::animation::_record_duration($path, ($duration as f64) / 1000.0, $closure)
    };

    // With fps: record!("out.gif", 2 secs @ 60 fps, |t| plot)
    ($path:expr, $duration:tt secs @ $fps:tt fps, $closure:expr) => {
        $crate::animation::_record_duration_fps($path, $duration as f64, $fps, $closure)
    };

    // With config (frames): record!("out.gif", 60, config: $cfg, |t| plot)
    ($path:expr, $frames:expr, config: $config:expr, $closure:expr) => {
        $crate::animation::_record_frames_config($path, $frames, $config, $closure)
    };

    // With config (duration): record!("out.gif", 2 secs, config: $cfg, |t| plot)
    ($path:expr, $duration:tt secs, config: $config:expr, $closure:expr) => {{
        let cfg: $crate::animation::RecordConfig = $config;
        let frames = ($duration as f64 * cfg.framerate as f64).ceil() as usize;
        $crate::animation::_record_frames_config($path, frames, cfg, $closure)
    }};

    // =========== Reactive plot (Plot created once) ===========

    // Reactive with duration: record!("out.gif", &plot, 2 secs)
    ($path:expr, & $plot:expr, $duration:tt secs) => {
        $crate::animation::_record_reactive($path, &$plot, $duration as f64, 30)
    };

    // Reactive with fps: record!("out.gif", &plot, 2 secs @ 30 fps)
    ($path:expr, & $plot:expr, $duration:tt secs @ $fps:tt fps) => {
        $crate::animation::_record_reactive($path, &$plot, $duration as f64, $fps)
    };

    // Reactive with config: record!("out.gif", &plot, 2 secs, config: $cfg)
    ($path:expr, & $plot:expr, $duration:tt secs, config: $config:expr) => {
        $crate::animation::_record_reactive_config($path, &$plot, $duration as f64, $config)
    };
}

#[cfg(test)]
mod tests {
    // Tests will use the macro from the crate root
    // For now, just verify the internal functions are accessible
    #[allow(unused)]
    use crate::animation::*;

    #[test]
    fn test_internal_functions_accessible() {
        // Just verify these exist and are accessible via type assertions
        // The reactive functions only need the path type parameter
        fn assert_reactive_exists<P: AsRef<std::path::Path>>() {
            let _ = _record_reactive::<P>
                as fn(P, &crate::core::Plot, f64, u32) -> crate::core::Result<()>;
            let _ = _record_reactive_config::<P>
                as fn(P, &crate::core::Plot, f64, RecordConfig) -> crate::core::Result<()>;
        }
        assert_reactive_exists::<&str>();
    }
}
