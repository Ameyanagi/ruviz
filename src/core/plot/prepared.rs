//! Prepared plot runtime for repeated frame rendering.

use super::{
    Image, InteractivePlotSession, Plot,
    data::{ReactiveTeardown, SharedReactiveCallback},
};
use crate::core::Result;
use std::fmt;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreparedFrameKey {
    size_px: (u32, u32),
    scale_bits: u32,
    time_bits: Option<u64>,
    versions: Vec<u64>,
}

#[derive(Clone, Debug)]
struct PreparedFrameCache {
    key: PreparedFrameKey,
    image: Image,
}

/// Active subscriptions to the push-based reactive inputs of a plot.
///
/// Dropping this value unsubscribes all registered listeners.
#[derive(Default)]
pub struct ReactiveSubscription {
    teardowns: Vec<ReactiveTeardown>,
}

impl fmt::Debug for ReactiveSubscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReactiveSubscription")
            .field("subscription_count", &self.teardowns.len())
            .finish()
    }
}

impl Drop for ReactiveSubscription {
    fn drop(&mut self) {
        for teardown in &mut self.teardowns {
            teardown();
        }
    }
}

impl ReactiveSubscription {
    /// Returns `true` when no push-based subscriptions were registered.
    pub fn is_empty(&self) -> bool {
        self.teardowns.is_empty()
    }
}

/// Reusable runtime for repeatedly rendering the same plot into frames.
///
/// `PreparedPlot` caches the last rendered image together with the plot's
/// reactive dependency versions, render size, scale factor, and temporal key.
#[derive(Debug)]
pub struct PreparedPlot {
    plot: Plot,
    cache: Mutex<Option<PreparedFrameCache>>,
}

impl Clone for PreparedPlot {
    fn clone(&self) -> Self {
        let cache = self
            .cache
            .lock()
            .expect("PreparedPlot cache lock poisoned")
            .clone();
        Self {
            plot: self.plot.clone(),
            cache: Mutex::new(cache),
        }
    }
}

impl PreparedPlot {
    pub(crate) fn new(plot: Plot) -> Self {
        Self {
            plot,
            cache: Mutex::new(None),
        }
    }

    /// Borrow the underlying declarative plot.
    pub fn plot(&self) -> &Plot {
        &self.plot
    }

    /// Drop any cached frame so the next render recomputes it.
    pub fn invalidate(&self) {
        *self.cache.lock().expect("PreparedPlot cache lock poisoned") = None;
    }

    /// Check whether the requested frame differs from the cached one.
    pub fn is_dirty(&self, size_px: (u32, u32), scale_factor: f32, time: f64) -> bool {
        let key = self.frame_key(size_px, scale_factor, time);
        self.cache
            .lock()
            .expect("PreparedPlot cache lock poisoned")
            .as_ref()
            .is_none_or(|cached| cached.key != key)
    }

    /// Render a frame for the given viewport size, scale factor, and time.
    pub fn render_frame(&self, size_px: (u32, u32), scale_factor: f32, time: f64) -> Result<Image> {
        let key = self.frame_key(size_px, scale_factor, time);

        if let Some(image) = self
            .cache
            .lock()
            .expect("PreparedPlot cache lock poisoned")
            .as_ref()
            .and_then(|cached| (cached.key == key).then(|| cached.image.clone()))
        {
            return Ok(image);
        }

        let image = self
            .plot
            .prepared_frame_plot(size_px, scale_factor, time)
            .render()?;
        self.plot.mark_reactive_sources_rendered();

        *self.cache.lock().expect("PreparedPlot cache lock poisoned") = Some(PreparedFrameCache {
            key,
            image: image.clone(),
        });

        Ok(image)
    }

    /// Subscribe to push-based reactive updates for the underlying plot.
    ///
    /// Temporal `Signal<T>` sources are sampled at render time and therefore do not
    /// participate in this subscription set.
    pub fn subscribe_reactive<F>(&self, callback: F) -> ReactiveSubscription
    where
        F: Fn() + Send + Sync + 'static,
    {
        let callback: SharedReactiveCallback = Arc::new(callback);
        let mut subscription = ReactiveSubscription::default();
        self.plot
            .subscribe_push_updates(callback, &mut subscription.teardowns);
        subscription
    }

    fn frame_key(&self, size_px: (u32, u32), scale_factor: f32, time: f64) -> PreparedFrameKey {
        PreparedFrameKey {
            size_px,
            scale_bits: scale_factor.to_bits(),
            time_bits: self.plot.has_temporal_sources().then_some(time.to_bits()),
            versions: self.plot.collect_reactive_versions(),
        }
    }

    /// Promote this prepared plot into a shared interactive session.
    pub fn into_interactive(self) -> InteractivePlotSession {
        InteractivePlotSession::new(self)
    }
}

impl From<Plot> for PreparedPlot {
    fn from(plot: Plot) -> Self {
        Self::new(plot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Observable, StreamingXY};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[test]
    fn test_prepared_plot_is_dirty_until_first_render() {
        let y = Observable::new(vec![0.0, 1.0, 4.0]);
        let plot: Plot = Plot::new()
            .line_source(vec![0.0, 1.0, 2.0], y.clone())
            .into();
        let prepared = plot.prepare();

        assert!(prepared.is_dirty((320, 240), 1.0, 0.0));

        prepared
            .render_frame((320, 240), 1.0, 0.0)
            .expect("prepared plot should render");

        assert!(!prepared.is_dirty((320, 240), 1.0, 0.0));

        y.set(vec![0.0, 1.0, 9.0]);
        assert!(prepared.is_dirty((320, 240), 1.0, 0.0));
    }

    #[test]
    fn test_prepared_plot_subscribe_reactive_observable() {
        let y = Observable::new(vec![0.0, 1.0, 4.0]);
        let plot: Plot = Plot::new()
            .line_source(vec![0.0, 1.0, 2.0], y.clone())
            .into();
        let prepared = plot.prepare();
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_for_callback = Arc::clone(&hits);
        let _subscription = prepared.subscribe_reactive(move || {
            hits_for_callback.fetch_add(1, Ordering::Relaxed);
        });

        y.set(vec![0.0, 1.0, 9.0]);

        assert_eq!(hits.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_prepared_plot_subscribe_reactive_streaming() {
        let stream = StreamingXY::new(32);
        let plot: Plot = Plot::new().line_streaming(&stream).into();
        let prepared = plot.prepare();
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_for_callback = Arc::clone(&hits);
        let _subscription = prepared.subscribe_reactive(move || {
            hits_for_callback.fetch_add(1, Ordering::Relaxed);
        });

        stream.push(1.0, 1.0);

        assert_eq!(hits.load(Ordering::Relaxed), 2);
    }
}
