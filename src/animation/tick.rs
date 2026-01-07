//! Tick-based animation timing system
//!
//! Provides deterministic frame timing for animation recording,
//! modeled after Makie.jl's Tick events.

use std::time::Instant;

/// Animation timing context for each frame
///
/// The `Tick` struct provides timing information that animation callbacks
/// can use to synchronize their updates. This follows Makie.jl's pattern
/// of providing `{count, time, delta_time, state}` for each frame.
///
/// # Example
///
/// ```rust
/// use ruviz::animation::{Tick, TickState};
///
/// let tick = Tick {
///     count: 30,
///     time: 1.0,
///     delta_time: 1.0 / 30.0,
///     state: TickState::Recording,
/// };
///
/// // Use tick.time for time-based animations
/// let phase = tick.time * 2.0 * std::f64::consts::PI;
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Tick {
    /// Frame number since animation start (0-indexed)
    pub count: u64,
    /// Total elapsed time in seconds since animation start
    pub time: f64,
    /// Time since last tick in seconds
    pub delta_time: f64,
    /// Current tick state
    pub state: TickState,
}

impl Tick {
    /// Create a new tick with the given parameters
    pub fn new(count: u64, time: f64, delta_time: f64, state: TickState) -> Self {
        Self {
            count,
            time,
            delta_time,
            state,
        }
    }

    /// Create the initial tick (frame 0)
    pub fn initial(framerate: f64) -> Self {
        Self {
            count: 0,
            time: 0.0,
            delta_time: 1.0 / framerate,
            state: TickState::Recording,
        }
    }

    // ========== Time Interpolation Helpers ==========
    // These methods simplify creating smooth animations without
    // needing AnimatedObservable for simple cases.

    /// Get normalized progress (0.0 to 1.0) within a time range
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::animation::{Tick, TickState};
    ///
    /// let tick = Tick::new(15, 0.5, 1.0/30.0, TickState::Recording);
    /// let progress = tick.progress(0.0, 1.0);  // 0.5 (halfway through 0-1s)
    /// ```
    #[inline]
    pub fn progress(&self, start_time: f64, end_time: f64) -> f64 {
        if end_time <= start_time {
            return 1.0;
        }
        ((self.time - start_time) / (end_time - start_time)).clamp(0.0, 1.0)
    }

    /// Linear interpolation between two values based on time progress
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::animation::{Tick, TickState};
    ///
    /// let tick = Tick::new(15, 0.5, 1.0/30.0, TickState::Recording);
    /// let x = tick.lerp(0.0, 100.0, 0.0, 1.0);  // 50.0 (halfway)
    /// ```
    #[inline]
    pub fn lerp(&self, start: f64, end: f64, start_time: f64, end_time: f64) -> f64 {
        let t = self.progress(start_time, end_time);
        start + (end - start) * t
    }

    /// Interpolation with easing function
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::animation::{Tick, TickState, easing};
    ///
    /// let tick = Tick::new(15, 0.5, 1.0/30.0, TickState::Recording);
    /// let x = tick.ease(easing::ease_out_quad, 0.0, 100.0, 0.0, 1.0);
    /// ```
    #[inline]
    pub fn ease(
        &self,
        easing_fn: fn(f64) -> f64,
        start: f64,
        end: f64,
        start_time: f64,
        end_time: f64,
    ) -> f64 {
        let t = self.progress(start_time, end_time);
        let eased_t = easing_fn(t);
        start + (end - start) * eased_t
    }

    /// Interpolation over the full animation duration (0 to `duration`)
    ///
    /// Convenience method when animation starts at t=0.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::animation::{Tick, TickState};
    ///
    /// let tick = Tick::new(30, 1.0, 1.0/30.0, TickState::Recording);
    /// let x = tick.lerp_over(0.0, 100.0, 2.0);  // 50.0 (halfway through 2s)
    /// ```
    #[inline]
    pub fn lerp_over(&self, start: f64, end: f64, duration: f64) -> f64 {
        self.lerp(start, end, 0.0, duration)
    }

    /// Eased interpolation over the full animation duration
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::animation::{Tick, TickState, easing};
    ///
    /// let tick = Tick::new(30, 1.0, 1.0/30.0, TickState::Recording);
    /// let x = tick.ease_over(easing::ease_out_elastic, 0.0, 100.0, 2.0);
    /// ```
    #[inline]
    pub fn ease_over(&self, easing_fn: fn(f64) -> f64, start: f64, end: f64, duration: f64) -> f64 {
        self.ease(easing_fn, start, end, 0.0, duration)
    }
}

/// State of the current tick
///
/// Used to distinguish between different phases of animation recording.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TickState {
    /// Standard recording tick
    Recording,
    /// Real-time playback tick
    Playback,
    /// Render skipped (no changes detected)
    Skipped,
    /// Final frame before save
    Finalizing,
}

/// Generates ticks at a fixed framerate
///
/// `TickGenerator` produces a sequence of `Tick` values for animation
/// recording. It can operate in two modes:
///
/// - **Immediate mode** (`tick_immediate`): Generates ticks as fast as possible
///   for batch recording
/// - **Real-time mode** (`next_tick`): Waits for the appropriate frame time
///   for live playback
///
/// # Example
///
/// ```rust
/// use ruviz::animation::TickGenerator;
///
/// let mut ticker = TickGenerator::new(30.0); // 30 FPS
///
/// // Generate ticks immediately for recording
/// let tick1 = ticker.tick_immediate();
/// let tick2 = ticker.tick_immediate();
///
/// assert_eq!(tick1.count, 0);
/// assert_eq!(tick2.count, 1);
/// ```
pub struct TickGenerator {
    framerate: f64,
    count: u64,
    start_time: Option<Instant>,
    last_tick_time: Option<Instant>,
    frame_duration: f64,
}

impl TickGenerator {
    /// Create a new tick generator with the given framerate
    ///
    /// # Arguments
    ///
    /// * `framerate` - Frames per second (e.g., 30.0, 60.0)
    ///
    /// # Panics
    ///
    /// Panics if framerate is not positive.
    pub fn new(framerate: f64) -> Self {
        assert!(framerate > 0.0, "Framerate must be positive");
        Self {
            framerate,
            count: 0,
            start_time: None,
            last_tick_time: None,
            frame_duration: 1.0 / framerate,
        }
    }

    /// Get the configured framerate
    pub fn framerate(&self) -> f64 {
        self.framerate
    }

    /// Get the frame duration in seconds
    pub fn frame_duration(&self) -> f64 {
        self.frame_duration
    }

    /// Generate the next tick immediately (for batch recording)
    ///
    /// This generates ticks as fast as possible without waiting,
    /// suitable for recording animations to file.
    pub fn tick_immediate(&mut self) -> Tick {
        let tick = Tick {
            count: self.count,
            time: self.count as f64 * self.frame_duration,
            delta_time: self.frame_duration,
            state: TickState::Recording,
        };
        self.count += 1;
        tick
    }

    /// Generate the next tick with real-time waiting (for playback)
    ///
    /// This waits until the appropriate time has passed since the last tick,
    /// suitable for real-time animation playback.
    pub fn next_tick(&mut self) -> Tick {
        let now = Instant::now();

        // Initialize timing on first call
        if self.start_time.is_none() {
            self.start_time = Some(now);
            self.last_tick_time = Some(now);
        }

        let start = self.start_time.unwrap();
        let target_time = start
            + std::time::Duration::from_secs_f64((self.count + 1) as f64 * self.frame_duration);

        // Wait if we're ahead of schedule
        if now < target_time {
            std::thread::sleep(target_time - now);
        }

        let actual_now = Instant::now();
        let actual_delta = actual_now
            .duration_since(self.last_tick_time.unwrap())
            .as_secs_f64();

        let tick = Tick {
            count: self.count,
            time: actual_now.duration_since(start).as_secs_f64(),
            delta_time: actual_delta,
            state: TickState::Playback,
        };

        self.last_tick_time = Some(actual_now);
        self.count += 1;
        tick
    }

    /// Generate a finalizing tick (for the last frame)
    pub fn tick_finalize(&mut self) -> Tick {
        let mut tick = self.tick_immediate();
        tick.state = TickState::Finalizing;
        tick
    }

    /// Reset the generator to frame 0
    pub fn reset(&mut self) {
        self.count = 0;
        self.start_time = None;
        self.last_tick_time = None;
    }

    /// Get the current frame count
    pub fn current_count(&self) -> u64 {
        self.count
    }
}

impl Iterator for TickGenerator {
    type Item = Tick;

    fn next(&mut self) -> Option<Tick> {
        // Iterator always produces ticks (infinite iterator)
        Some(self.tick_immediate())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_initial() {
        let tick = Tick::initial(30.0);
        assert_eq!(tick.count, 0);
        assert_eq!(tick.time, 0.0);
        assert!((tick.delta_time - 1.0 / 30.0).abs() < 1e-10);
        assert_eq!(tick.state, TickState::Recording);
    }

    #[test]
    fn test_tick_generator_immediate() {
        let mut ticker = TickGenerator::new(30.0);

        let tick0 = ticker.tick_immediate();
        assert_eq!(tick0.count, 0);
        assert_eq!(tick0.time, 0.0);

        let tick1 = ticker.tick_immediate();
        assert_eq!(tick1.count, 1);
        assert!((tick1.time - 1.0 / 30.0).abs() < 1e-10);

        let tick2 = ticker.tick_immediate();
        assert_eq!(tick2.count, 2);
        assert!((tick2.time - 2.0 / 30.0).abs() < 1e-10);
    }

    #[test]
    fn test_tick_generator_reset() {
        let mut ticker = TickGenerator::new(60.0);

        ticker.tick_immediate();
        ticker.tick_immediate();
        assert_eq!(ticker.current_count(), 2);

        ticker.reset();
        assert_eq!(ticker.current_count(), 0);

        let tick = ticker.tick_immediate();
        assert_eq!(tick.count, 0);
    }

    #[test]
    fn test_tick_generator_iterator() {
        let ticker = TickGenerator::new(24.0);
        let ticks: Vec<Tick> = ticker.take(5).collect();

        assert_eq!(ticks.len(), 5);
        for (i, tick) in ticks.iter().enumerate() {
            assert_eq!(tick.count, i as u64);
        }
    }

    #[test]
    fn test_tick_generator_finalize() {
        let mut ticker = TickGenerator::new(30.0);
        ticker.tick_immediate();
        ticker.tick_immediate();

        let final_tick = ticker.tick_finalize();
        assert_eq!(final_tick.count, 2);
        assert_eq!(final_tick.state, TickState::Finalizing);
    }

    #[test]
    #[should_panic(expected = "Framerate must be positive")]
    fn test_invalid_framerate() {
        TickGenerator::new(0.0);
    }

    #[test]
    fn test_tick_state_equality() {
        assert_eq!(TickState::Recording, TickState::Recording);
        assert_ne!(TickState::Recording, TickState::Playback);
    }

    #[test]
    fn test_frame_duration() {
        let ticker = TickGenerator::new(60.0);
        assert!((ticker.frame_duration() - 1.0 / 60.0).abs() < 1e-10);
    }

    // ========== Interpolation Helper Tests ==========

    #[test]
    fn test_progress_midpoint() {
        let tick = Tick::new(15, 0.5, 1.0 / 30.0, TickState::Recording);
        let progress = tick.progress(0.0, 1.0);
        assert!((progress - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_progress_clamping_above() {
        let tick = Tick::new(60, 2.0, 1.0 / 30.0, TickState::Recording);
        let progress = tick.progress(0.0, 1.0);
        assert!((progress - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_progress_clamping_below() {
        let tick = Tick::new(0, 0.0, 1.0 / 30.0, TickState::Recording);
        let progress = tick.progress(0.5, 1.0);
        assert!((progress - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_progress_invalid_range() {
        let tick = Tick::new(15, 0.5, 1.0 / 30.0, TickState::Recording);
        // end <= start should return 1.0
        let progress = tick.progress(1.0, 0.5);
        assert!((progress - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_lerp_midpoint() {
        let tick = Tick::new(15, 0.5, 1.0 / 30.0, TickState::Recording);
        let value = tick.lerp(0.0, 100.0, 0.0, 1.0);
        assert!((value - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_lerp_start() {
        let tick = Tick::new(0, 0.0, 1.0 / 30.0, TickState::Recording);
        let value = tick.lerp(10.0, 20.0, 0.0, 2.0);
        assert!((value - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_lerp_end() {
        let tick = Tick::new(60, 2.0, 1.0 / 30.0, TickState::Recording);
        let value = tick.lerp(10.0, 20.0, 0.0, 2.0);
        assert!((value - 20.0).abs() < 1e-10);
    }

    #[test]
    fn test_ease_with_ease_out_quad() {
        use crate::animation::easing;
        let tick = Tick::new(15, 0.5, 1.0 / 30.0, TickState::Recording);
        let value = tick.ease(easing::ease_out_quad, 0.0, 100.0, 0.0, 1.0);
        // ease_out_quad(0.5) = 1 - (1-0.5)^2 = 1 - 0.25 = 0.75
        assert!((value - 75.0).abs() < 1e-10);
    }

    #[test]
    fn test_ease_with_ease_in_quad() {
        use crate::animation::easing;
        let tick = Tick::new(15, 0.5, 1.0 / 30.0, TickState::Recording);
        let value = tick.ease(easing::ease_in_quad, 0.0, 100.0, 0.0, 1.0);
        // ease_in_quad(0.5) = 0.5^2 = 0.25
        assert!((value - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_lerp_over_convenience() {
        let tick = Tick::new(30, 1.0, 1.0 / 30.0, TickState::Recording);
        let value = tick.lerp_over(0.0, 100.0, 2.0);
        // At t=1.0 with duration=2.0, we're at 50%
        assert!((value - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_ease_over_convenience() {
        use crate::animation::easing;
        let tick = Tick::new(30, 1.0, 1.0 / 30.0, TickState::Recording);
        let value = tick.ease_over(easing::linear, 0.0, 100.0, 2.0);
        // Linear at 50% = 50
        assert!((value - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_ease_over_with_elastic() {
        use crate::animation::easing;
        let tick = Tick::new(60, 2.0, 1.0 / 30.0, TickState::Recording);
        let value = tick.ease_over(easing::ease_out_elastic, 0.0, 100.0, 2.0);
        // At progress=1.0, all easings should return 1.0
        assert!((value - 100.0).abs() < 1e-10);
    }
}
