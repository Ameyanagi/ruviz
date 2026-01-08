//! Reactive data types for plot series
//!
//! This module provides `PlotData` and `PlotText` enums that allow plot series
//! and text attributes to hold either static data or reactive data sources
//! (Signal or Observable).
//!
//! # Overview
//!
//! - `PlotData` - Wraps numeric data (`Vec<f64>`) that can be static or reactive
//! - `PlotText` - Wraps text (String) that can be static or reactive
//! - `IntoPlotData` - Trait for converting various types into PlotData
//!
//! # Examples
//!
//! ```rust,ignore
//! use ruviz::animation::signal;
//! use ruviz::core::plot::data::{PlotData, IntoPlotData};
//!
//! // Static data (existing behavior)
//! let static_data: PlotData = vec![1.0, 2.0, 3.0].into_plot_data();
//!
//! // Reactive data via Signal
//! let y_signal = signal::of(|t| (0..100).map(|i| (i as f64 * 0.1 + t).sin()).collect());
//! let reactive_data: PlotData = y_signal.into_plot_data();
//!
//! // Resolve at a specific time
//! let data_at_0 = static_data.resolve(0.0);
//! let data_at_1 = reactive_data.resolve(1.0);
//! ```

use crate::data::Observable;
use crate::data::signal::Signal;

// ============================================================================
// PlotData Enum
// ============================================================================

/// Data source for plot series - can be static or reactive.
///
/// `PlotData` allows plot series to hold either:
/// - Static data (`Vec<f64>`) - resolved immediately
/// - Temporal data (`Signal<Vec<f64>>`) - evaluated at render time based on animation time
/// - Reactive data (`Observable<Vec<f64>>`) - current value retrieved at render time
///
/// This enables the "create plot once, render at different times" pattern for
/// efficient animations where the plot structure doesn't change, only the data.
#[derive(Clone)]
pub enum PlotData {
    /// Concrete data (owned, static) - the current behavior
    Static(Vec<f64>),

    /// Time-varying data (pull-based, for animation recording)
    /// The signal is evaluated at render time with the current animation time.
    Temporal(Signal<Vec<f64>>),

    /// Push-based reactive data (for live/interactive updates)
    /// The current value is retrieved at render time.
    Reactive(Observable<Vec<f64>>),
}

impl PlotData {
    /// Resolve the data to a concrete `Vec<f64>` at the given time.
    ///
    /// - `Static` - Returns a clone of the stored data
    /// - `Temporal` - Evaluates the signal at the given time
    /// - `Reactive` - Returns the current value of the observable
    ///
    /// # Arguments
    ///
    /// * `time` - The time at which to resolve (in seconds). Only used for `Temporal`.
    #[inline]
    pub fn resolve(&self, time: f64) -> Vec<f64> {
        match self {
            PlotData::Static(data) => data.clone(),
            PlotData::Temporal(signal) => signal.at(time),
            PlotData::Reactive(obs) => obs.get(),
        }
    }

    /// Check if this data is static (no resolution needed).
    ///
    /// Returns `true` for `PlotData::Static`, `false` otherwise.
    #[inline]
    pub fn is_static(&self) -> bool {
        matches!(self, PlotData::Static(_))
    }

    /// Check if this data is reactive (Signal or Observable).
    #[inline]
    pub fn is_reactive(&self) -> bool {
        !self.is_static()
    }

    /// Get a reference to static data if available.
    ///
    /// Returns `Some(&Vec<f64>)` for static data, `None` for reactive.
    #[inline]
    pub fn as_static(&self) -> Option<&Vec<f64>> {
        match self {
            PlotData::Static(data) => Some(data),
            _ => None,
        }
    }

    /// Get the length of the data.
    ///
    /// For static data, returns the length directly.
    /// For reactive data, resolves at t=0 and returns that length.
    pub fn len(&self) -> usize {
        match self {
            PlotData::Static(data) => data.len(),
            PlotData::Temporal(signal) => signal.at(0.0).len(),
            PlotData::Reactive(obs) => obs.get().len(),
        }
    }

    /// Check if the data is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl std::fmt::Debug for PlotData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlotData::Static(data) => f
                .debug_tuple("Static")
                .field(&format!(
                    "[{}; {}]",
                    if data.is_empty() {
                        "".to_string()
                    } else {
                        format!("{:.2}...", data[0])
                    },
                    data.len()
                ))
                .finish(),
            PlotData::Temporal(_) => f
                .debug_tuple("Temporal")
                .field(&"Signal<Vec<f64>>")
                .finish(),
            PlotData::Reactive(_) => f
                .debug_tuple("Reactive")
                .field(&"Observable<Vec<f64>>")
                .finish(),
        }
    }
}

// ============================================================================
// IntoPlotData Trait
// ============================================================================

/// Trait for converting various types into `PlotData`.
///
/// This trait enables plot methods to accept multiple data source types
/// with a unified API, maintaining backward compatibility with existing code.
///
/// # Implemented For
///
/// - `Vec<f64>` → `PlotData::Static`
/// - `&[f64]` → `PlotData::Static` (clones the slice)
/// - `Signal<Vec<f64>>` → `PlotData::Temporal`
/// - `Observable<Vec<f64>>` → `PlotData::Reactive`
/// - `PlotData` → `PlotData` (identity)
pub trait IntoPlotData {
    /// Convert self into PlotData
    fn into_plot_data(self) -> PlotData;
}

impl IntoPlotData for Vec<f64> {
    #[inline]
    fn into_plot_data(self) -> PlotData {
        PlotData::Static(self)
    }
}

impl IntoPlotData for &[f64] {
    #[inline]
    fn into_plot_data(self) -> PlotData {
        PlotData::Static(self.to_vec())
    }
}

impl<const N: usize> IntoPlotData for &[f64; N] {
    #[inline]
    fn into_plot_data(self) -> PlotData {
        PlotData::Static(self.to_vec())
    }
}

impl IntoPlotData for Signal<Vec<f64>> {
    #[inline]
    fn into_plot_data(self) -> PlotData {
        PlotData::Temporal(self)
    }
}

impl IntoPlotData for Observable<Vec<f64>> {
    #[inline]
    fn into_plot_data(self) -> PlotData {
        PlotData::Reactive(self)
    }
}

impl IntoPlotData for PlotData {
    #[inline]
    fn into_plot_data(self) -> PlotData {
        self
    }
}

// ============================================================================
// PlotText Enum
// ============================================================================

/// Text attribute that can be static or reactive.
///
/// Used for plot title, axis labels, and other text attributes that
/// may need to update during animation.
#[derive(Clone)]
pub enum PlotText {
    /// Static text
    Static(String),

    /// Time-varying text (for animation)
    Temporal(Signal<String>),

    /// Push-based reactive text
    Reactive(Observable<String>),
}

impl PlotText {
    /// Resolve the text to a concrete String at the given time.
    #[inline]
    pub fn resolve(&self, time: f64) -> String {
        match self {
            PlotText::Static(s) => s.clone(),
            PlotText::Temporal(signal) => signal.at(time),
            PlotText::Reactive(obs) => obs.get(),
        }
    }

    /// Check if this text is static.
    #[inline]
    pub fn is_static(&self) -> bool {
        matches!(self, PlotText::Static(_))
    }

    /// Get a reference to static text if available.
    #[inline]
    pub fn as_static(&self) -> Option<&str> {
        match self {
            PlotText::Static(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

impl Default for PlotText {
    fn default() -> Self {
        PlotText::Static(String::new())
    }
}

impl std::fmt::Debug for PlotText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlotText::Static(s) => f.debug_tuple("Static").field(s).finish(),
            PlotText::Temporal(_) => f.debug_tuple("Temporal").field(&"Signal<String>").finish(),
            PlotText::Reactive(_) => f
                .debug_tuple("Reactive")
                .field(&"Observable<String>")
                .finish(),
        }
    }
}

impl From<String> for PlotText {
    fn from(s: String) -> Self {
        PlotText::Static(s)
    }
}

impl From<&str> for PlotText {
    fn from(s: &str) -> Self {
        PlotText::Static(s.to_string())
    }
}

impl From<Signal<String>> for PlotText {
    fn from(signal: Signal<String>) -> Self {
        PlotText::Temporal(signal)
    }
}

impl From<Observable<String>> for PlotText {
    fn from(obs: Observable<String>) -> Self {
        PlotText::Reactive(obs)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::signal;

    #[test]
    fn test_plot_data_static() {
        let data = PlotData::Static(vec![1.0, 2.0, 3.0]);
        assert!(data.is_static());
        assert!(!data.is_reactive());
        assert_eq!(data.resolve(0.0), vec![1.0, 2.0, 3.0]);
        assert_eq!(data.len(), 3);
    }

    #[test]
    fn test_plot_data_temporal() {
        let signal = signal::of(|t| vec![t, t * 2.0, t * 3.0]);
        let data = PlotData::Temporal(signal);
        assert!(!data.is_static());
        assert!(data.is_reactive());
        assert_eq!(data.resolve(1.0), vec![1.0, 2.0, 3.0]);
        assert_eq!(data.resolve(2.0), vec![2.0, 4.0, 6.0]);
    }

    #[test]
    fn test_plot_data_reactive() {
        let obs = Observable::new(vec![1.0, 2.0, 3.0]);
        let data = PlotData::Reactive(obs.clone());
        assert!(!data.is_static());
        assert!(data.is_reactive());
        assert_eq!(data.resolve(0.0), vec![1.0, 2.0, 3.0]);

        obs.set(vec![4.0, 5.0, 6.0]);
        assert_eq!(data.resolve(0.0), vec![4.0, 5.0, 6.0]);
    }

    #[test]
    fn test_into_plot_data_vec() {
        let data: PlotData = vec![1.0, 2.0].into_plot_data();
        assert!(data.is_static());
    }

    #[test]
    fn test_into_plot_data_slice() {
        let arr = [1.0, 2.0, 3.0];
        let data: PlotData = arr.as_slice().into_plot_data();
        assert!(data.is_static());
        assert_eq!(data.resolve(0.0), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_into_plot_data_array_ref() {
        let arr = [1.0, 2.0, 3.0];
        let data: PlotData = (&arr).into_plot_data();
        assert!(data.is_static());
    }

    #[test]
    fn test_into_plot_data_signal() {
        let signal = signal::constant(vec![1.0, 2.0]);
        let data: PlotData = signal.into_plot_data();
        assert!(!data.is_static());
    }

    #[test]
    fn test_plot_text_static() {
        let text = PlotText::Static("Hello".to_string());
        assert!(text.is_static());
        assert_eq!(text.resolve(0.0), "Hello");
    }

    #[test]
    fn test_plot_text_temporal() {
        let signal = signal::of(|t| format!("t={:.2}", t));
        let text = PlotText::Temporal(signal);
        assert!(!text.is_static());
        assert_eq!(text.resolve(1.5), "t=1.50");
    }

    #[test]
    fn test_plot_text_from_string() {
        let text: PlotText = "Hello".into();
        assert!(text.is_static());
        assert_eq!(text.as_static(), Some("Hello"));
    }

    #[test]
    fn test_plot_text_from_owned_string() {
        let text: PlotText = String::from("World").into();
        assert!(text.is_static());
    }
}
