//! Prepared plot runtime for repeated frame rendering.

use super::{
    BackendOperation, Image, InteractivePlotSession, Plot, RenderDiagnostics, RenderExecutionMode,
    ResolvedStyle,
    data::{ReactiveTeardown, SharedReactiveCallback},
    raster_batches::SeriesRasterPlan,
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

#[derive(Clone, Debug)]
struct PreparedResolvedPlotCache {
    key: PreparedFrameKey,
    plot: Arc<Plot>,
}

#[derive(Clone, Debug)]
struct PreparedGeometryCache {
    key: PreparedFrameKey,
    plot_area_bits: (u32, u32, u32, u32),
    x_bounds_bits: (u64, u64),
    y_bounds_bits: (u64, u64),
    plans: Arc<[Option<SeriesRasterPlan>]>,
}

type PreparedGeometryPlans = Arc<[Option<SeriesRasterPlan>]>;

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
    resolved_cache: Mutex<Option<PreparedResolvedPlotCache>>,
    geometry_cache: Mutex<Option<PreparedGeometryCache>>,
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
            resolved_cache: Mutex::new(
                self.resolved_cache
                    .lock()
                    .expect("PreparedPlot resolved cache lock poisoned")
                    .clone(),
            ),
            geometry_cache: Mutex::new(
                self.geometry_cache
                    .lock()
                    .expect("PreparedPlot geometry cache lock poisoned")
                    .clone(),
            ),
        }
    }
}

impl PreparedPlot {
    pub(crate) fn new(plot: Plot) -> Self {
        Self {
            plot,
            cache: Mutex::new(None),
            resolved_cache: Mutex::new(None),
            geometry_cache: Mutex::new(None),
        }
    }

    /// Borrow the underlying declarative plot.
    pub fn plot(&self) -> &Plot {
        &self.plot
    }

    /// Drop any cached frame so the next render recomputes it.
    pub fn invalidate(&self) {
        *self.cache.lock().expect("PreparedPlot cache lock poisoned") = None;
        *self
            .resolved_cache
            .lock()
            .expect("PreparedPlot resolved cache lock poisoned") = None;
        *self
            .geometry_cache
            .lock()
            .expect("PreparedPlot geometry cache lock poisoned") = None;
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

        let image = self.render_frame_uncached(size_px, scale_factor, time)?;

        *self.cache.lock().expect("PreparedPlot cache lock poisoned") = Some(PreparedFrameCache {
            key,
            image: image.clone(),
        });

        Ok(image)
    }

    /// Render a frame while bypassing the cached image, but still reusing the
    /// prepared per-frame plot state when the key is unchanged.
    pub fn render_frame_uncached(
        &self,
        size_px: (u32, u32),
        scale_factor: f32,
        time: f64,
    ) -> Result<Image> {
        self.render_frame_uncached_with_diagnostics(size_px, scale_factor, time)
            .map(|(image, _)| image)
    }

    /// Render PNG bytes for the plot's configured output size through the
    /// prepared runtime. Reuses the cached image when the prepared frame key
    /// has not changed.
    pub fn render_png_bytes(&self) -> Result<Vec<u8>> {
        let (width, height) = self.plot.config_canvas_size();
        self.render_frame((width, height), 1.0, 0.0)?.encode_png()
    }

    /// Render PNG bytes while bypassing the cached image, but still reusing the
    /// cached prepared per-frame plot state when possible.
    pub fn render_png_bytes_uncached(&self) -> Result<Vec<u8>> {
        let (width, height) = self.plot.config_canvas_size();
        self.render_frame_uncached((width, height), 1.0, 0.0)?
            .encode_png()
    }

    #[doc(hidden)]
    pub fn render_png_bytes_uncached_with_diagnostics(
        &self,
    ) -> Result<(Vec<u8>, RenderDiagnostics)> {
        let (width, height) = self.plot.config_canvas_size();
        let (image, diagnostics) =
            self.render_frame_uncached_with_diagnostics((width, height), 1.0, 0.0)?;
        Ok((image.encode_png()?, diagnostics))
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
            scale_bits: Plot::sanitize_prepared_scale_factor(scale_factor).to_bits(),
            time_bits: self.plot.has_temporal_sources().then_some(time.to_bits()),
            versions: self.plot.collect_reactive_versions(),
        }
    }

    fn prepared_render_plot(
        &self,
        key: &PreparedFrameKey,
        size_px: (u32, u32),
        scale_factor: f32,
        style: &ResolvedStyle,
    ) -> Result<Arc<Plot>> {
        if let Some(plot) = self
            .resolved_cache
            .lock()
            .expect("PreparedPlot resolved cache lock poisoned")
            .as_ref()
            .and_then(|cached| (cached.key == *key).then(|| Arc::clone(&cached.plot)))
        {
            return Ok(plot);
        }

        let plot = Arc::new(self.plot.prepared_frame_shell_with_style(
            size_px,
            scale_factor,
            style,
        ));
        *self
            .resolved_cache
            .lock()
            .expect("PreparedPlot resolved cache lock poisoned") =
            Some(PreparedResolvedPlotCache {
                key: key.clone(),
                plot: Arc::clone(&plot),
            });
        Ok(plot)
    }

    fn render_frame_uncached_with_diagnostics(
        &self,
        size_px: (u32, u32),
        scale_factor: f32,
        time: f64,
    ) -> Result<(Image, RenderDiagnostics)> {
        let key = self.frame_key(size_px, scale_factor, time);
        self.plot.validate_before_frame_resolution()?;
        let frame = self.plot.resolve_frame(time)?;
        let prepared_plot = self.prepared_render_plot(&key, size_px, scale_factor, &frame.style)?;

        let mode = prepared_plot.render_execution_mode(BackendOperation::RasterImage);
        let result = prepared_plot
            .render_renderer_with_resolved_frame(
                mode,
                &frame,
                |plot,
                 snapshot_series,
                 resolved_series,
                 renderer,
                 plot_area,
                 x_min,
                 x_max,
                 y_min,
                 y_max,
                 _render_scale,
                 mode| {
                    let (prepared_geometry, used_cache, rebuilt_cache) = self
                        .prepared_series_geometry(
                            &key,
                            plot,
                            snapshot_series,
                            resolved_series,
                            plot_area,
                            (x_min, x_max),
                            (y_min, y_max),
                            mode,
                        )?;

                    if rebuilt_cache {
                        renderer.note_rebuilt_prepared_geometry_cache();
                    }
                    if used_cache {
                        renderer.note_prepared_geometry_cache();
                    }

                    for (series_index, (series, resolved)) in
                        snapshot_series.iter().zip(resolved_series).enumerate()
                    {
                        if let Some(plan) =
                            prepared_geometry.get(series_index).and_then(Option::as_ref)
                        {
                            let color = series.color_with_alpha(crate::render::Color::new(0, 0, 0));
                            let line_width =
                                plot.dpi_scaled_line_width(series.line_width.unwrap_or(2.0));
                            let line_style = series
                                .line_style
                                .clone()
                                .unwrap_or(crate::render::LineStyle::Solid);
                            plan.execute(renderer)?;
                            plot.render_series_overlays_after_raster(
                                series,
                                resolved,
                                renderer,
                                plot_area,
                                x_min,
                                x_max,
                                y_min,
                                y_max,
                                color,
                                line_width,
                                &line_style,
                            )?;
                        } else {
                            plot.render_series_normal(
                                series, resolved, renderer, plot_area, x_min, x_max, y_min, y_max,
                                mode,
                            )?;
                        }
                    }

                    Ok(())
                },
            )
            .map(|(renderer, diagnostics)| (renderer.into_image(), diagnostics));
        if result.is_ok() {
            frame.acknowledge_rendered(&self.plot);
        }
        result
    }

    fn prepared_series_geometry(
        &self,
        key: &PreparedFrameKey,
        plot: &Plot,
        snapshot_series: &[super::PlotSeries],
        resolved_series: &[super::ResolvedSeries<'_>],
        plot_area: tiny_skia::Rect,
        x_bounds: (f64, f64),
        y_bounds: (f64, f64),
        mode: RenderExecutionMode,
    ) -> Result<(PreparedGeometryPlans, bool, bool)> {
        if let Some(cached) = self
            .geometry_cache
            .lock()
            .expect("PreparedPlot geometry cache lock poisoned")
            .as_ref()
            .filter(|cached| {
                cached.key == *key
                    && cached.plot_area_bits == rect_bits(plot_area)
                    && cached.x_bounds_bits == bounds_bits(x_bounds)
                    && cached.y_bounds_bits == bounds_bits(y_bounds)
                    && cached.plans.len() == snapshot_series.len()
            })
            .cloned()
        {
            return Ok((cached.plans, true, false));
        }

        let plans = snapshot_series
            .iter()
            .zip(resolved_series)
            .map(|(series, resolved)| {
                plot.build_prepared_series_raster_plan(
                    series, resolved, plot_area, x_bounds.0, x_bounds.1, y_bounds.0, y_bounds.1,
                    mode,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let plans: PreparedGeometryPlans = plans.into();
        *self
            .geometry_cache
            .lock()
            .expect("PreparedPlot geometry cache lock poisoned") = Some(PreparedGeometryCache {
            key: key.clone(),
            plot_area_bits: rect_bits(plot_area),
            x_bounds_bits: bounds_bits(x_bounds),
            y_bounds_bits: bounds_bits(y_bounds),
            plans: Arc::clone(&plans),
        });
        Ok((plans, false, true))
    }

    /// Promote this prepared plot into a shared interactive session.
    pub fn into_interactive(self) -> InteractivePlotSession {
        InteractivePlotSession::new(self)
    }
}

fn rect_bits(rect: tiny_skia::Rect) -> (u32, u32, u32, u32) {
    (
        rect.x().to_bits(),
        rect.y().to_bits(),
        rect.width().to_bits(),
        rect.height().to_bits(),
    )
}

fn bounds_bits(bounds: (f64, f64)) -> (u64, u64) {
    (bounds.0.to_bits(), bounds.1.to_bits())
}

impl From<Plot> for PreparedPlot {
    fn from(plot: Plot) -> Self {
        Self::new(plot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Observable, StreamingRenderState, StreamingXY};
    use crate::render::Color;
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
    fn test_prepared_plot_render_png_bytes_uses_cached_image() {
        let plot: Plot = Plot::new().line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 0.0]).into();
        let prepared = plot.prepare();

        let png_a = prepared
            .render_png_bytes()
            .expect("prepared plot should render cached png");
        let png_b = prepared
            .render_png_bytes()
            .expect("prepared plot should reuse cached png image");

        assert_eq!(png_a, png_b);
        assert!(!prepared.is_dirty(plot.config_canvas_size(), 1.0, 0.0));
    }

    #[test]
    fn test_prepared_plot_render_png_bytes_uncached_matches_cached_path() {
        let plot: Plot = Plot::new()
            .scatter(&[0.0, 1.0, 2.0], &[0.0, 1.0, 0.0])
            .into();
        let prepared = plot.prepare();

        let cached = prepared
            .render_png_bytes()
            .expect("prepared cached png should render");
        let uncached = prepared
            .render_png_bytes_uncached()
            .expect("prepared uncached png should render");

        assert_eq!(cached, uncached);
    }

    #[test]
    fn test_prepared_plot_render_png_bytes_uncached_matches_cached_path_for_line() {
        let x: Vec<f64> = (0..4_096).map(|index| index as f64 * 0.01).collect();
        let y: Vec<f64> = x
            .iter()
            .map(|value| value.sin() + 0.15 * (value * 3.0).cos())
            .collect();
        let plot: Plot = Plot::new().line(&x, &y).into();
        let prepared = plot.prepare();

        let cached = prepared
            .render_png_bytes()
            .expect("prepared cached line png should render");
        let uncached = prepared
            .render_png_bytes_uncached()
            .expect("prepared uncached line png should render");

        assert_eq!(cached, uncached);
    }

    #[test]
    fn test_prepared_plot_render_png_bytes_uncached_matches_cached_path_for_heatmap() {
        let matrix: Vec<Vec<f64>> = (0..64)
            .map(|row| {
                (0..64)
                    .map(|col| {
                        let x = row as f64 / 63.0;
                        let y = col as f64 / 63.0;
                        (x * std::f64::consts::TAU).sin() * (y * std::f64::consts::PI).cos()
                    })
                    .collect()
            })
            .collect();
        let plot: Plot = Plot::new().heatmap(&matrix, None).into();
        let prepared = plot.prepare();

        let cached = prepared
            .render_png_bytes()
            .expect("prepared cached heatmap png should render");
        let uncached = prepared
            .render_png_bytes_uncached()
            .expect("prepared uncached heatmap png should render");

        assert_eq!(cached, uncached);
    }

    #[test]
    fn test_prepared_plot_uncached_geometry_cache_reports_rebuild_then_hit() {
        let x: Vec<f64> = (0..2_048).map(|index| index as f64 * 0.01).collect();
        let y: Vec<f64> = x.iter().map(|value| value.sin()).collect();
        let plot: Plot = Plot::new().scatter(&x, &y).into();
        let prepared = plot.prepare();

        let (_, first) = prepared
            .render_png_bytes_uncached_with_diagnostics()
            .expect("first uncached prepared render should succeed");
        let (_, second) = prepared
            .render_png_bytes_uncached_with_diagnostics()
            .expect("second uncached prepared render should succeed");

        assert!(first.rebuilt_prepared_geometry_cache);
        assert!(!first.used_prepared_geometry_cache);
        assert!(!second.rebuilt_prepared_geometry_cache);
        assert!(second.used_prepared_geometry_cache);
    }

    #[test]
    fn test_prepared_plot_uncached_geometry_cache_rebuilds_on_size_change() {
        let plot: Plot = Plot::new().line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 0.0]).into();
        let prepared = plot.prepare();

        let (_, first) = prepared
            .render_frame_uncached_with_diagnostics((320, 240), 1.0, 0.0)
            .expect("first prepared render should succeed");
        let (_, second) = prepared
            .render_frame_uncached_with_diagnostics((640, 480), 1.0, 0.0)
            .expect("size-changed prepared render should succeed");

        assert!(first.rebuilt_prepared_geometry_cache);
        assert!(!first.used_prepared_geometry_cache);
        assert!(second.rebuilt_prepared_geometry_cache);
        assert!(!second.used_prepared_geometry_cache);
    }

    #[test]
    fn test_prepared_plot_uncached_geometry_cache_rebuilds_on_reactive_change() {
        let y = Observable::new(vec![0.0, 1.0, 4.0, 9.0]);
        let plot: Plot = Plot::new()
            .line_source(vec![0.0, 1.0, 2.0, 3.0], y.clone())
            .into();
        let prepared = plot.prepare();

        let (_, first) = prepared
            .render_png_bytes_uncached_with_diagnostics()
            .expect("first prepared render should succeed");
        let (_, second) = prepared
            .render_png_bytes_uncached_with_diagnostics()
            .expect("second prepared render should hit geometry cache");
        y.set(vec![0.0, 1.0, 8.0, 27.0]);
        let (_, third) = prepared
            .render_png_bytes_uncached_with_diagnostics()
            .expect("reactive update should rebuild geometry cache");

        assert!(first.rebuilt_prepared_geometry_cache);
        assert!(second.used_prepared_geometry_cache);
        assert!(third.rebuilt_prepared_geometry_cache);
        assert!(!third.used_prepared_geometry_cache);
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
    fn test_prepared_plot_is_dirty_after_reactive_color_change() {
        let color = Observable::new(Color::RED);
        let plot: Plot = Plot::new()
            .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 0.5])
            .color_source(color.clone())
            .into();
        let prepared = plot.prepare();

        prepared
            .render_frame((320, 240), 1.0, 0.0)
            .expect("prepared plot should render");

        assert!(!prepared.is_dirty((320, 240), 1.0, 0.0));

        color.set(Color::BLUE);

        assert!(prepared.is_dirty((320, 240), 1.0, 0.0));
    }

    #[test]
    fn test_prepared_plot_subscribe_reactive_color_observable() {
        let color = Observable::new(Color::RED);
        let plot: Plot = Plot::new()
            .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 0.5])
            .color_source(color.clone())
            .into();
        let prepared = plot.prepare();
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_for_callback = Arc::clone(&hits);
        let _subscription = prepared.subscribe_reactive(move || {
            hits_for_callback.fetch_add(1, Ordering::Relaxed);
        });

        color.set(Color::BLUE);

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

        assert_eq!(hits.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_prepared_plot_full_redraw_acknowledges_streaming_replacement() {
        let stream = StreamingXY::new(32);
        stream.push_many([(0.0, 0.0), (1.0, 1.0)]);
        let plot: Plot = Plot::new()
            .line_streaming(&stream)
            .xlim(0.0, 4.0)
            .ylim(0.0, 16.0)
            .into();
        let prepared = plot.prepare();
        prepared
            .render_frame((320, 240), 1.0, 0.0)
            .expect("initial prepared streaming frame should render");

        stream.replace([(2.0, 4.0), (3.0, 9.0), (4.0, 16.0)]);
        assert_eq!(
            stream.render_state(),
            StreamingRenderState::FullRedrawRequired
        );

        prepared
            .render_frame((320, 240), 1.0, 0.0)
            .expect("prepared replacement frame should render");
        assert_eq!(stream.render_state(), StreamingRenderState::Unchanged);

        stream.replace(std::iter::empty());
        assert_eq!(
            stream.render_state(),
            StreamingRenderState::FullRedrawRequired
        );
        prepared
            .render_frame((320, 240), 1.0, 0.0)
            .expect("empty replacement frame should render");
        assert_eq!(stream.render_state(), StreamingRenderState::Unchanged);
    }

    #[test]
    fn test_prepared_plot_renders_empty_streaming_replacement_on_log_axes() {
        let stream = StreamingXY::new(32);
        stream.push_many([(1.0, 1.0), (10.0, 10.0)]);
        let plot: Plot = Plot::new()
            .line_streaming(&stream)
            .xscale(crate::axes::AxisScale::Log)
            .yscale(crate::axes::AxisScale::Log)
            .into();
        let prepared = plot.prepare();
        prepared
            .render_frame((320, 240), 1.0, 0.0)
            .expect("initial logarithmic stream frame should render");

        stream.replace(std::iter::empty());
        prepared
            .render_frame((320, 240), 1.0, 0.0)
            .expect("empty logarithmic replacement should render");

        assert_eq!(stream.render_state(), StreamingRenderState::Unchanged);
    }

    #[test]
    fn test_prepared_plot_acknowledges_generic_streaming_buffers() {
        let x = crate::data::StreamingBuffer::new(16);
        let y = crate::data::StreamingBuffer::new(16);
        x.push_many(vec![0.0, 1.0]);
        y.push_many(vec![0.0, 1.0]);
        let plot: Plot = Plot::new().line_source(x.clone(), y.clone()).into();
        let prepared = plot.prepare();

        prepared
            .render_frame((320, 240), 1.0, 0.0)
            .expect("prepared generic streaming frame should render");

        assert_eq!(x.appended_since_mark(), 0);
        assert_eq!(y.appended_since_mark(), 0);
    }

    #[test]
    fn test_prepared_plot_cache_does_not_retain_static_series_vectors() {
        let plot: Plot = Plot::new().line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0]).into();
        let prepared = plot.prepare();

        prepared
            .render_frame((320, 240), 1.0, 0.0)
            .expect("prepared static frame should render");

        let cache = prepared
            .resolved_cache
            .lock()
            .expect("PreparedPlot resolved cache lock poisoned");
        let cached = cache
            .as_ref()
            .expect("prepared plot shell should be cached");
        let super::super::SeriesType::Line { x_data, y_data } =
            &cached.plot.series_mgr.series[0].series_type
        else {
            panic!("cached prepared series should remain a line");
        };
        assert!(x_data.as_static().is_some_and(Vec::is_empty));
        assert!(y_data.as_static().is_some_and(Vec::is_empty));
    }

    #[test]
    fn test_prepared_style_shell_uses_pre_sample_version_key() {
        let color = Observable::new(Color::RED);
        let plot: Plot = Plot::new()
            .line(&[0.0, 1.0], &[0.0, 1.0])
            .color_source(color.clone())
            .into();
        let prepared = plot.prepare();
        let size = (320, 240);
        let key_before = prepared.frame_key(size, 1.0, 0.0);
        let frame = prepared
            .plot
            .resolve_frame(0.0)
            .expect("frame should resolve");

        color.set(Color::BLUE);
        let key_after = prepared.frame_key(size, 1.0, 0.0);
        assert_ne!(key_before, key_after);

        prepared
            .prepared_render_plot(&key_before, size, 1.0, &frame.style)
            .expect("prepared style shell should build");
        let cache = prepared
            .resolved_cache
            .lock()
            .expect("PreparedPlot resolved cache lock poisoned");
        let cached = cache.as_ref().expect("style shell should be cached");
        assert_eq!(cached.key, key_before);
        assert_ne!(cached.key, key_after);
        assert_eq!(cached.plot.series_mgr.series[0].color, Some(Color::RED));
    }

    #[test]
    fn test_prepared_plot_observes_legacy_lane_writes_without_pair_duplicates() {
        let stream = StreamingXY::new(32);
        stream.push(1.0, 10.0);
        let plot: Plot = Plot::new()
            .line_streaming(&stream)
            .xlim(0.0, 3.0)
            .ylim(0.0, 30.0)
            .into();
        let prepared = plot.prepare();
        prepared
            .render_frame((320, 240), 1.0, 0.0)
            .expect("initial streaming frame should render");
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_for_callback = Arc::clone(&hits);
        let _subscription = prepared.subscribe_reactive(move || {
            hits_for_callback.fetch_add(1, Ordering::Relaxed);
        });

        stream.x().push(2.0);
        stream.y().push(20.0);

        assert_eq!(hits.load(Ordering::Relaxed), 2);
        assert!(prepared.is_dirty((320, 240), 1.0, 0.0));
        prepared
            .render_frame((320, 240), 1.0, 0.0)
            .expect("legacy lane writes should render as an aligned pair");
        assert!(!prepared.is_dirty((320, 240), 1.0, 0.0));
    }

    #[test]
    fn test_prepared_plot_streaming_callback_can_render_immediately() {
        let stream = StreamingXY::new(32);
        let plot: Plot = Plot::new()
            .line_streaming(&stream)
            .xlim(0.0, 4.0)
            .ylim(0.0, 4.0)
            .into();
        let prepared = plot.prepare();
        let prepared_for_callback = prepared.clone();
        let errors = Arc::new(Mutex::new(Vec::new()));
        let errors_for_callback = Arc::clone(&errors);

        let _subscription = prepared.subscribe_reactive(move || {
            if let Err(err) = prepared_for_callback.render_frame((320, 240), 1.0, 0.0) {
                errors_for_callback
                    .lock()
                    .expect("error lock poisoned")
                    .push(err.to_string());
            }
        });

        stream.push(1.0, 1.0);

        assert!(
            errors.lock().expect("error lock poisoned").is_empty(),
            "streaming rerender in callback should not observe mismatched x/y data"
        );
    }

    #[test]
    fn test_prepared_plot_normalizes_invalid_scale_factor_in_cache_key() {
        let plot: Plot = Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]).into();
        let prepared = plot.prepare();

        prepared
            .render_frame((320, 240), 0.0, 0.0)
            .expect("prepared plot should render with sanitized scale factor");

        assert!(
            !prepared.is_dirty((320, 240), -1.0, 0.0),
            "equivalent sanitized scale factors should reuse the cached frame"
        );
    }
}
