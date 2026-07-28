//! Framework-neutral building blocks for native GUI adapter crates.
//!
//! This module deliberately contains no windowing, event-loop, clipboard, or
//! GUI-framework dependency. Adapters retain ownership of those concerns while
//! sharing plot conversion, sizing, fitted-content mapping, and latest-request
//! scheduling semantics.

use super::{InteractivePlotSession, IntoPlot, Plot, PreparedPlot};
use std::{
    num::NonZeroU64,
    sync::atomic::{AtomicU64, Ordering},
};

/// Converts a 2D plot value into the retained session used by GUI adapters.
///
/// This is implemented for [`Plot`], every builder implementing [`IntoPlot`],
/// [`PreparedPlot`], and [`InteractivePlotSession`].
pub trait IntoPlotSession {
    /// Consume the value and return a retained interactive session.
    fn into_plot_session(self) -> InteractivePlotSession;
}

impl<T> IntoPlotSession for T
where
    T: IntoPlot,
{
    fn into_plot_session(self) -> InteractivePlotSession {
        self.into_plot().prepare_interactive()
    }
}

impl IntoPlotSession for PreparedPlot {
    fn into_plot_session(self) -> InteractivePlotSession {
        self.into_interactive()
    }
}

impl IntoPlotSession for InteractivePlotSession {
    fn into_plot_session(self) -> InteractivePlotSession {
        self
    }
}

/// Converts a 3D builder or retained session into a retained 3D session.
#[cfg(feature = "3d")]
pub trait TryIntoPlot3DSession {
    /// Consume the value and return a retained 3D session.
    fn try_into_plot3d_session(self) -> super::Result<super::InteractivePlot3DSession>;
}

#[cfg(feature = "3d")]
impl TryIntoPlot3DSession for super::InteractivePlot3DSession {
    fn try_into_plot3d_session(self) -> super::Result<super::InteractivePlot3DSession> {
        Ok(self)
    }
}

#[cfg(feature = "3d")]
macro_rules! impl_try_into_plot3d_session {
    ($($builder:ty),+ $(,)?) => {
        $(
            impl TryIntoPlot3DSession for $builder {
                fn try_into_plot3d_session(
                    self,
                ) -> super::Result<super::InteractivePlot3DSession> {
                    self.interactive_session()
                }
            }
        )+
    };
}

#[cfg(feature = "3d")]
impl_try_into_plot3d_session!(
    super::Scatter3DBuilder,
    super::Line3DBuilder,
    super::Surface3DBuilder,
    super::Wireframe3DBuilder,
);

/// Image fitting policy shared by image-backed adapter presentations.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ImageFit {
    /// Preserve aspect ratio and show the complete image.
    #[default]
    Contain,
    /// Preserve aspect ratio and cover the complete outer rectangle.
    Cover,
    /// Stretch the image to the complete outer rectangle.
    Fill,
}

/// Framework-neutral logical rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LogicalRect {
    /// Horizontal coordinate of the rectangle's minimum edge.
    pub x: f64,
    /// Vertical coordinate of the rectangle's minimum edge.
    pub y: f64,
    /// Logical width of the rectangle.
    pub width: f64,
    /// Logical height of the rectangle.
    pub height: f64,
}

impl LogicalRect {
    /// Construct a logical rectangle from its minimum corner and dimensions.
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Whether a logical point lies inside this rectangle.
    pub fn contains(self, point: LogicalPoint) -> bool {
        point.x >= self.x
            && point.y >= self.y
            && point.x <= self.x + self.width
            && point.y <= self.y + self.height
    }
}

/// Framework-neutral logical point.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LogicalPoint {
    /// Horizontal logical coordinate.
    pub x: f64,
    /// Vertical logical coordinate.
    pub y: f64,
}

impl LogicalPoint {
    /// Construct a logical point.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Return a finite positive device scale, falling back to `1.0`.
pub fn sanitize_scale_factor(scale_factor: f32) -> f32 {
    if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    }
}

/// Convert logical widget dimensions into a physical backing size.
///
/// Dimensions use `ceil` so a fractional logical edge never loses its final
/// physical pixel. Non-finite and non-positive dimensions produce one pixel.
pub fn physical_backing_size(
    logical_width: f64,
    logical_height: f64,
    scale_factor: f32,
) -> (u32, u32) {
    let scale = f64::from(sanitize_scale_factor(scale_factor));
    (
        physical_dimension(logical_width, scale),
        physical_dimension(logical_height, scale),
    )
}

fn physical_dimension(logical: f64, scale: f64) -> u32 {
    if !logical.is_finite() || logical <= 0.0 {
        return 1;
    }
    (logical * scale).ceil().clamp(1.0, f64::from(u32::MAX)) as u32
}

/// Fit an image into a logical outer rectangle.
pub fn fitted_content_rect(
    outer: LogicalRect,
    image_size_px: (u32, u32),
    fit: ImageFit,
) -> LogicalRect {
    if !outer.width.is_finite()
        || !outer.height.is_finite()
        || outer.width <= 0.0
        || outer.height <= 0.0
        || image_size_px.0 == 0
        || image_size_px.1 == 0
        || matches!(fit, ImageFit::Fill)
    {
        return outer;
    }

    let image_aspect = f64::from(image_size_px.0) / f64::from(image_size_px.1);
    let outer_aspect = outer.width / outer.height;
    let (width, height) = match fit {
        ImageFit::Contain if image_aspect > outer_aspect => {
            (outer.width, outer.width / image_aspect)
        }
        ImageFit::Contain => (outer.height * image_aspect, outer.height),
        ImageFit::Cover if image_aspect < outer_aspect => (outer.width, outer.width / image_aspect),
        ImageFit::Cover => (outer.height * image_aspect, outer.height),
        ImageFit::Fill => unreachable!("fill was returned above"),
    };
    LogicalRect {
        x: outer.x + (outer.width - width) * 0.5,
        y: outer.y + (outer.height - height) * 0.5,
        width,
        height,
    }
}

/// Map a logical point inside fitted content to physical image pixels.
pub fn logical_to_physical(
    content: LogicalRect,
    point: LogicalPoint,
    image_size_px: (u32, u32),
) -> Option<(f64, f64)> {
    if image_size_px.0 == 0
        || image_size_px.1 == 0
        || !content.width.is_finite()
        || !content.height.is_finite()
        || content.width <= 0.0
        || content.height <= 0.0
        || !content.contains(point)
    {
        return None;
    }
    Some((
        (point.x - content.x) / content.width * f64::from(image_size_px.0),
        (point.y - content.y) / content.height * f64::from(image_size_px.1),
    ))
}

static NEXT_SCHEDULER_INCARNATION: AtomicU64 = AtomicU64::new(1);

fn next_scheduler_incarnation() -> NonZeroU64 {
    next_scheduler_incarnation_from(&NEXT_SCHEDULER_INCARNATION)
}

fn next_scheduler_incarnation_from(counter: &AtomicU64) -> NonZeroU64 {
    let incarnation = counter
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
            next.checked_add(1)
        })
        .unwrap_or_else(|_| panic!("adapter scheduler incarnation space exhausted"));
    NonZeroU64::new(incarnation).expect("scheduler incarnation counter never produces zero")
}

/// Opaque identity of one accepted adapter render request.
///
/// The identity combines a process-unique scheduler incarnation with that
/// scheduler's monotonic generation. Pass the complete value to
/// [`LatestRequestScheduler::complete`]; a generation alone cannot distinguish
/// work produced by a dropped or reset scheduler.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ScheduledRequestId {
    scheduler_incarnation: NonZeroU64,
    generation: u64,
}

impl ScheduledRequestId {
    /// Per-scheduler generation, exposed for diagnostics only.
    ///
    /// Generations can repeat across scheduler incarnations and must not be
    /// used as completion identities.
    pub const fn generation(self) -> u64 {
        self.generation
    }
}

/// One request accepted by [`LatestRequestScheduler`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduledRequest<T> {
    id: ScheduledRequestId,
    request: T,
}

impl<T> ScheduledRequest<T> {
    /// Globally incarnation-safe identity used to complete this request.
    pub const fn id(&self) -> ScheduledRequestId {
        self.id
    }

    /// Per-scheduler generation, exposed for diagnostics only.
    ///
    /// Use [`Self::id`] when reporting completion to the scheduler.
    pub const fn generation(&self) -> u64 {
        self.id.generation()
    }

    /// Borrow the adapter request payload.
    pub const fn request(&self) -> &T {
        &self.request
    }

    /// Consume the scheduled request and return its payload.
    pub fn into_request(self) -> T {
        self.request
    }
}

/// Result of completing one scheduled request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestCompletion<T> {
    /// Whether the completed frame is still the latest requested frame.
    pub install: bool,
    /// The coalesced newest request to start next, if any.
    pub next: Option<ScheduledRequest<T>>,
}

/// Pure latest-request scheduler for background image rendering.
///
/// At most one request is in flight. While it runs, repeated requests are
/// coalesced to the newest one. Completion of an older request is explicitly
/// marked non-installable when a newer request exists.
#[derive(Debug)]
pub struct LatestRequestScheduler<T> {
    scheduler_incarnation: NonZeroU64,
    latest_generation: u64,
    in_flight: Option<ScheduledRequestId>,
    queued: Option<ScheduledRequest<T>>,
    dropped_requests: u64,
}

impl<T> Default for LatestRequestScheduler<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> LatestRequestScheduler<T> {
    /// Construct an idle scheduler with a process-unique incarnation.
    pub fn new() -> Self {
        Self {
            scheduler_incarnation: next_scheduler_incarnation(),
            latest_generation: 0,
            in_flight: None,
            queued: None,
            dropped_requests: 0,
        }
    }

    /// Latest per-incarnation generation, exposed for diagnostics only.
    pub const fn latest_generation(&self) -> u64 {
        self.latest_generation
    }

    /// Number of queued requests replaced by a newer request.
    pub const fn dropped_requests(&self) -> u64 {
        self.dropped_requests
    }

    /// Whether no request is currently running.
    pub fn is_idle(&self) -> bool {
        self.in_flight.is_none()
    }

    /// Cancel pending work and begin a new scheduler incarnation.
    ///
    /// A worker cancelled by this reset may still complete later. Rotating the
    /// incarnation guarantees its identity cannot match any replacement work,
    /// even when the replacement starts again at generation one.
    pub fn reset(&mut self) {
        self.scheduler_incarnation = next_scheduler_incarnation();
        self.latest_generation = 0;
        self.in_flight = None;
        self.queued = None;
        self.dropped_requests = 0;
    }
}

impl<T> LatestRequestScheduler<T> {
    /// Queue a request and return it when a worker should be started now.
    pub fn request(&mut self, request: T) -> Option<ScheduledRequest<T>> {
        self.latest_generation = self
            .latest_generation
            .checked_add(1)
            .expect("adapter render request generation exhausted");
        let scheduled = ScheduledRequest {
            id: ScheduledRequestId {
                scheduler_incarnation: self.scheduler_incarnation,
                generation: self.latest_generation,
            },
            request,
        };
        if self.in_flight.is_some() {
            if self.queued.replace(scheduled).is_some() {
                self.dropped_requests = self.dropped_requests.saturating_add(1);
            }
            None
        } else {
            self.in_flight = Some(scheduled.id());
            Some(scheduled)
        }
    }

    /// Complete an in-flight request.
    ///
    /// Returns `None` for a stale or duplicate completion. The returned `next`
    /// request has already become the in-flight request.
    pub fn complete(&mut self, id: ScheduledRequestId) -> Option<RequestCompletion<T>> {
        if self.in_flight != Some(id) {
            return None;
        }
        self.in_flight = None;
        let install = id.scheduler_incarnation == self.scheduler_incarnation
            && id.generation == self.latest_generation;
        let next = self.queued.take();
        if let Some(next) = &next {
            self.in_flight = Some(next.id());
        }
        Some(RequestCompletion { install, next })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plot_values_convert_to_retained_sessions() {
        let session = Plot::new()
            .line(&[0.0, 1.0], &[1.0, 2.0])
            .into_plot_session();
        assert_eq!(session.displayed_frame_generation(), None);
    }

    #[test]
    fn fractional_hidpi_dimensions_round_up() {
        assert_eq!(physical_backing_size(100.25, 50.1, 1.25), (126, 63));
        assert_eq!(physical_backing_size(100.25, 50.1, 1.5), (151, 76));
        assert_eq!(physical_backing_size(100.25, 50.1, 2.0), (201, 101));
    }

    #[test]
    fn contain_fit_and_mapping_use_actual_content_bounds() {
        let outer = LogicalRect::new(10.0, 20.0, 400.0, 400.0);
        let content = fitted_content_rect(outer, (400, 200), ImageFit::Contain);
        assert_eq!(content, LogicalRect::new(10.0, 120.0, 400.0, 200.0));
        assert_eq!(
            logical_to_physical(content, LogicalPoint::new(210.0, 220.0), (800, 400)),
            Some((400.0, 200.0))
        );
        assert_eq!(
            logical_to_physical(content, LogicalPoint::new(210.0, 100.0), (800, 400)),
            None
        );
    }

    #[test]
    fn newest_request_is_the_only_installable_completion() {
        let mut scheduler = LatestRequestScheduler::default();
        let first = scheduler.request("a").expect("first request starts");
        assert!(scheduler.request("b").is_none());
        assert!(scheduler.request("c").is_none());
        assert_eq!(scheduler.dropped_requests(), 1);

        let completion = scheduler
            .complete(first.id())
            .expect("first request was in flight");
        assert!(!completion.install);
        let newest = completion.next.expect("newest request starts next");
        assert_eq!(newest.request(), &"c");

        let completion = scheduler
            .complete(newest.id())
            .expect("newest request was in flight");
        assert!(completion.install);
        assert!(completion.next.is_none());
        assert!(scheduler.is_idle());
    }

    #[test]
    fn stale_completion_cannot_change_scheduler_state() {
        let mut scheduler = LatestRequestScheduler::default();
        let request = scheduler.request(1).unwrap();
        let mut stale_id = request.id();
        stale_id.generation += 1;
        assert!(scheduler.complete(stale_id).is_none());
        assert!(!scheduler.is_idle());
        assert!(scheduler.complete(request.id()).is_some());
    }

    #[test]
    fn scheduler_accepts_non_clone_requests_without_copying_jobs() {
        #[derive(Debug, Eq, PartialEq)]
        struct NonCloneRequest(&'static str);

        let mut scheduler = LatestRequestScheduler::new();
        let first = scheduler
            .request(NonCloneRequest("first"))
            .expect("first starts");
        assert!(scheduler.request(NonCloneRequest("queued")).is_none());

        let completion = scheduler.complete(first.id()).expect("first completion");
        assert!(!completion.install);
        let queued = completion.next.expect("queued request starts");
        assert_eq!(queued.request(), &NonCloneRequest("queued"));
        assert!(
            scheduler
                .complete(queued.id())
                .expect("queued completion")
                .install
        );
    }

    #[test]
    fn completion_from_a_dropped_scheduler_cannot_consume_new_work() {
        let stale = {
            let mut old_scheduler = LatestRequestScheduler::default();
            old_scheduler.request("old scheduler").unwrap()
        };
        let mut new_scheduler = LatestRequestScheduler::default();
        let current = new_scheduler.request("new scheduler").unwrap();

        assert_eq!(stale.generation(), current.generation());
        assert_ne!(stale.id(), current.id());
        assert!(new_scheduler.complete(stale.id()).is_none());
        assert!(!new_scheduler.is_idle());
        assert!(
            new_scheduler
                .complete(current.id())
                .expect("current completion")
                .install
        );
    }

    #[test]
    fn reset_rotates_incarnation_before_reusing_generation() {
        let mut scheduler = LatestRequestScheduler::default();
        let cancelled = scheduler.request("old session").unwrap();
        scheduler.reset();
        let replacement = scheduler.request("new session").unwrap();

        assert_eq!(cancelled.generation(), replacement.generation());
        assert_ne!(cancelled.id(), replacement.id());
        assert!(scheduler.complete(cancelled.id()).is_none());
        assert!(!scheduler.is_idle());
        assert!(
            scheduler
                .complete(replacement.id())
                .expect("replacement completion")
                .install
        );
    }

    #[test]
    #[should_panic(expected = "adapter scheduler incarnation space exhausted")]
    fn scheduler_incarnation_exhaustion_panics_before_reuse() {
        let exhausted = AtomicU64::new(u64::MAX);
        let _ = next_scheduler_incarnation_from(&exhausted);
    }
}
