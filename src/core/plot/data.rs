//! Reactive data types for plot series
//!
//! This module provides a generic [`ReactiveValue`] abstraction plus
//! plot-specific wrappers for numeric series and text attributes.
//!
//! # Overview
//!
//! - `ReactiveValue<T>` - Generic static/temporal/reactive wrapper
//! - `PlotData` - Numeric plot payloads, including live streaming buffers
//! - `PlotText` - Text payloads used by titles and labels
//! - `IntoPlotData` - Trait for converting common numeric sources into `PlotData`

use crate::data::signal::Signal;
use crate::data::{Observable, StreamingBuffer, StreamingRenderState};
use std::borrow::Cow;
use std::sync::Arc;

// ============================================================================
// ReactiveValue
// ============================================================================

/// Generic source wrapper for static, temporal, or push-based reactive values.
#[derive(Clone)]
pub enum ReactiveValue<T> {
    /// Concrete static value.
    Static(T),
    /// Time-varying value evaluated at render time.
    Temporal(Signal<T>),
    /// Push-based reactive value read at render time.
    Reactive(Observable<T>),
}

pub(crate) type SharedReactiveCallback = Arc<dyn Fn() + Send + Sync + 'static>;
pub(crate) type ReactiveTeardown = Box<dyn FnMut() + Send + 'static>;

impl<T> ReactiveValue<T> {
    /// Check if this value is static.
    #[inline]
    pub fn is_static(&self) -> bool {
        matches!(self, Self::Static(_))
    }

    /// Check if this value is temporal.
    #[inline]
    pub fn is_temporal(&self) -> bool {
        matches!(self, Self::Temporal(_))
    }

    /// Check if this value is reactive (temporal or observable).
    #[inline]
    pub fn is_reactive(&self) -> bool {
        !self.is_static()
    }

    /// Get the static value, if present.
    #[inline]
    pub fn as_static(&self) -> Option<&T> {
        match self {
            Self::Static(value) => Some(value),
            _ => None,
        }
    }

    /// Get the current observable version if this source is push-based.
    #[inline]
    pub fn current_version(&self) -> Option<u64> {
        match self {
            Self::Reactive(obs) => Some(obs.version()),
            _ => None,
        }
    }

    pub(crate) fn subscribe_push_updates(
        &self,
        callback: SharedReactiveCallback,
        teardowns: &mut Vec<ReactiveTeardown>,
    ) where
        T: Send + Sync + 'static,
    {
        if let Self::Reactive(obs) = self {
            let obs = obs.clone();
            let callback = Arc::clone(&callback);
            let id = obs.subscribe(move || callback());
            teardowns.push(Box::new(move || {
                obs.unsubscribe(id);
            }));
        }
    }
}

impl<T> From<T> for ReactiveValue<T> {
    fn from(value: T) -> Self {
        ReactiveValue::Static(value)
    }
}

impl<T> From<Signal<T>> for ReactiveValue<T> {
    fn from(signal: Signal<T>) -> Self {
        ReactiveValue::Temporal(signal)
    }
}

impl<T> From<Observable<T>> for ReactiveValue<T> {
    fn from(obs: Observable<T>) -> Self {
        ReactiveValue::Reactive(obs)
    }
}

impl<T: Clone> ReactiveValue<T> {
    /// Resolve the value at the given time.
    #[inline]
    pub fn resolve(&self, time: f64) -> T {
        match self {
            Self::Static(value) => value.clone(),
            Self::Temporal(signal) => signal.at(time),
            Self::Reactive(obs) => obs.get(),
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for ReactiveValue<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static(value) => f.debug_tuple("Static").field(value).finish(),
            Self::Temporal(_) => f.debug_tuple("Temporal").field(&"Signal<_>").finish(),
            Self::Reactive(_) => f.debug_tuple("Reactive").field(&"Observable<_>").finish(),
        }
    }
}

/// Generic public name for plot-facing reactive sources.
pub type PlotSource<T> = ReactiveValue<T>;

// ============================================================================
// PlotData
// ============================================================================

/// Data source for plot series.
///
/// `PlotData` supports:
/// - static vectors
/// - `Signal<Vec<f64>>` temporal sources
/// - `Observable<Vec<f64>>` push-based sources
/// - `StreamingBuffer<f64>` live streaming sources
#[derive(Clone)]
pub enum PlotData {
    /// Concrete static data.
    Static(Vec<f64>),
    /// Time-varying data evaluated at render time.
    Temporal(Signal<Vec<f64>>),
    /// Push-based reactive data read at render time.
    Reactive(Observable<Vec<f64>>),
    /// Live streaming data from a ring buffer.
    Streaming(StreamingBuffer<f64>),
}

impl PlotData {
    /// Resolve the data to a borrowed-or-owned slice at the given time.
    #[inline]
    pub fn resolve_cow(&self, time: f64) -> Cow<'_, [f64]> {
        match self {
            Self::Static(data) => Cow::Borrowed(data.as_slice()),
            Self::Temporal(signal) => Cow::Owned(signal.at(time)),
            Self::Reactive(obs) => Cow::Owned(obs.get()),
            Self::Streaming(buffer) => Cow::Owned(buffer.read()),
        }
    }

    /// Resolve the data to a concrete vector at the given time.
    #[inline]
    pub fn resolve(&self, time: f64) -> Vec<f64> {
        self.resolve_cow(time).into_owned()
    }

    /// Check if this data is static.
    #[inline]
    pub fn is_static(&self) -> bool {
        matches!(self, Self::Static(_))
    }

    /// Check if this data is temporal.
    #[inline]
    pub fn is_temporal(&self) -> bool {
        matches!(self, Self::Temporal(_))
    }

    /// Check if this data is reactive.
    #[inline]
    pub fn is_reactive(&self) -> bool {
        !self.is_static()
    }

    /// Get a reference to the static data if available.
    #[inline]
    pub fn as_static(&self) -> Option<&Vec<f64>> {
        match self {
            Self::Static(data) => Some(data),
            _ => None,
        }
    }

    /// Get the current data length.
    pub fn len(&self) -> usize {
        match self {
            Self::Static(data) => data.len(),
            Self::Temporal(signal) => signal.at(0.0).len(),
            Self::Reactive(obs) => obs.get().len(),
            Self::Streaming(buffer) => buffer.len(),
        }
    }

    /// Check if the data is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the current version for push-based sources.
    pub fn current_version(&self) -> Option<u64> {
        match self {
            Self::Reactive(obs) => Some(obs.version()),
            Self::Streaming(buffer) => Some(buffer.version()),
            _ => None,
        }
    }

    /// Mark streaming data as rendered.
    pub fn mark_rendered(&self) {
        if let Self::Streaming(buffer) = self {
            buffer.mark_rendered();
        }
    }

    /// Describe the incremental rendering state for streaming sources.
    pub fn streaming_render_state(&self) -> Option<StreamingRenderState> {
        match self {
            Self::Streaming(buffer) => Some(buffer.render_state()),
            _ => None,
        }
    }

    /// Check whether a streaming source can use append-only rendering.
    pub fn can_partial_render(&self) -> bool {
        self.streaming_render_state()
            .is_some_and(StreamingRenderState::can_incrementally_render)
    }

    /// Get the number of values appended since the last rendered mark.
    pub fn appended_count(&self) -> usize {
        match self {
            Self::Streaming(buffer) => buffer.appended_since_mark(),
            _ => 0,
        }
    }

    /// Read only newly appended data for streaming sources.
    pub fn resolve_appended(&self) -> Option<Vec<f64>> {
        match self {
            Self::Streaming(buffer) => Some(buffer.read_appended()),
            _ => None,
        }
    }

    pub(crate) fn subscribe_push_updates(
        &self,
        callback: SharedReactiveCallback,
        teardowns: &mut Vec<ReactiveTeardown>,
    ) {
        match self {
            Self::Reactive(obs) => {
                let obs = obs.clone();
                let callback = Arc::clone(&callback);
                let id = obs.subscribe(move || callback());
                teardowns.push(Box::new(move || {
                    obs.unsubscribe(id);
                }));
            }
            Self::Streaming(buffer) => {
                let buffer = buffer.clone();
                let callback = Arc::clone(&callback);
                let id = buffer.subscribe(move || callback());
                teardowns.push(Box::new(move || {
                    buffer.unsubscribe(id);
                }));
            }
            Self::Static(_) | Self::Temporal(_) => {}
        }
    }
}

impl std::fmt::Debug for PlotData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static(data) => f
                .debug_tuple("Static")
                .field(&format!(
                    "[{}; {}]",
                    if data.is_empty() {
                        String::new()
                    } else {
                        format!("{:.2}...", data[0])
                    },
                    data.len()
                ))
                .finish(),
            Self::Temporal(_) => f
                .debug_tuple("Temporal")
                .field(&"Signal<Vec<f64>>")
                .finish(),
            Self::Reactive(_) => f
                .debug_tuple("Reactive")
                .field(&"Observable<Vec<f64>>")
                .finish(),
            Self::Streaming(_) => f
                .debug_tuple("Streaming")
                .field(&"StreamingBuffer<f64>")
                .finish(),
        }
    }
}

// ============================================================================
// IntoPlotData
// ============================================================================

/// Trait for converting various types into `PlotData`.
pub trait IntoPlotData {
    /// Convert self into `PlotData`.
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

impl IntoPlotData for StreamingBuffer<f64> {
    #[inline]
    fn into_plot_data(self) -> PlotData {
        PlotData::Streaming(self)
    }
}

impl IntoPlotData for PlotData {
    #[inline]
    fn into_plot_data(self) -> PlotData {
        self
    }
}

// ============================================================================
// PlotText
// ============================================================================

/// Text attribute that can be static or reactive.
pub type PlotText = ReactiveValue<String>;

impl Default for ReactiveValue<String> {
    fn default() -> Self {
        ReactiveValue::Static(String::new())
    }
}

impl ReactiveValue<String> {
    /// Get a reference to the static string, if present.
    #[inline]
    pub fn as_static_str(&self) -> Option<&str> {
        self.as_static().map(String::as_str)
    }
}

impl From<&str> for ReactiveValue<String> {
    fn from(s: &str) -> Self {
        ReactiveValue::Static(s.to_string())
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
    fn test_plot_data_streaming() {
        let buffer = StreamingBuffer::new(8);
        buffer.push_many([1.0, 2.0, 3.0]);
        let data = PlotData::Streaming(buffer.clone());
        assert!(data.is_reactive());
        assert_eq!(data.resolve(0.0), vec![1.0, 2.0, 3.0]);
        assert_eq!(data.current_version(), Some(buffer.version()));
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
    fn test_into_plot_data_streaming_buffer() {
        let buffer = StreamingBuffer::new(16);
        buffer.push_many([1.0, 2.0, 3.0]);
        let data: PlotData = buffer.into_plot_data();
        assert!(data.is_reactive());
        assert_eq!(data.resolve(0.0), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_plot_text_static() {
        let text = PlotText::Static("Hello".to_string());
        assert!(text.is_static());
        assert_eq!(text.resolve(0.0), "Hello");
        assert_eq!(text.as_static_str(), Some("Hello"));
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
        assert_eq!(text.as_static_str(), Some("Hello"));
    }

    #[test]
    fn test_plot_text_from_owned_string() {
        let text: PlotText = String::from("World").into();
        assert!(text.is_static());
    }
}
