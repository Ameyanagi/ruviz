//! Native [`iced`] integration for [`ruviz`].
//!
//! The adapter follows Iced's Elm architecture: applications own a
//! [`PlotState`] (or [`Plot3DState`]), route the opaque [`Message`] through
//! [`PlotState::update`], return the resulting [`Task`], and include
//! [`PlotState::subscription`] in their application subscription.
//!
//! Rendering is image-backed and never happens in a widget `view`, `layout`, or
//! `draw` callback. CPU rendering is the default. The `gpu` feature only
//! enables ruviz GPU rendering followed by image readback; it is not a
//! zero-copy Iced texture path.

mod state;
mod widget;

use std::sync::Arc;

use iced::Task;
use iced::widget::image;
use ruviz::core::{AlphaMode, HitResult, PlottingError, StampedInteractiveFrame};

#[cfg(feature = "3d")]
use ruviz::core::{PickHit3D, RenderedImage3D};

pub use iced;
pub use ruviz;
#[cfg(feature = "3d")]
pub use state::{Plot3DState, interactive_3d, static_view_3d};
pub use state::{PlotState, interactive, static_view};
#[cfg(feature = "3d")]
pub use widget::{Plot3DWidget, plot3d};
pub use widget::{PlotWidget, plot};

#[derive(Clone, Debug)]
struct StateIncarnation(Arc<()>);

impl StateIncarnation {
    fn new() -> Self {
        Self(Arc::new(()))
    }
}

impl PartialEq for StateIncarnation {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for StateIncarnation {}

/// Whether a widget accepts plot interaction.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Presentation {
    /// Resize and redraw, but ignore pointer and keyboard interaction.
    Static,
    /// Enable pan, zoom, hover, selection, reset, and 3D camera interaction.
    #[default]
    Interactive,
}

/// Iced layout policy for a plot widget.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sizing {
    /// Consume the available width and height.
    Fill,
    /// Request an exact logical size.
    Fixed { width: f32, height: f32 },
}

impl Default for Sizing {
    fn default() -> Self {
        Self::Fixed {
            width: 640.0,
            height: 480.0,
        }
    }
}

impl Sizing {
    fn logical_fallback(self) -> (f64, f64) {
        match self {
            Self::Fill => (640.0, 480.0),
            Self::Fixed { width, height } => {
                (f64::from(width.max(1.0)), f64::from(height.max(1.0)))
            }
        }
    }
}

/// Observable adapter event produced while handling an Iced message.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Event {
    /// The latest render failed. Superseded renders are intentionally omitted.
    Error(PlottingError),
    /// A 2D click changed the selected hit.
    SelectionChanged(HitResult),
    /// A 2D click completed inside the fitted plot image.
    Clicked2D(HitResult),
    /// The current 2D hover hit, or `None` after the pointer leaves.
    Hovered2D(Option<HitResult>),
    /// A plot was reset to its initial 2D viewport.
    ViewReset,
    /// Pan, wheel zoom, or rectangular zoom changed the 2D viewport.
    ViewChanged,
    /// A pointer capture or drag was cancelled.
    DragCancelled,
    /// A 3D click picked a scene primitive.
    #[cfg(feature = "3d")]
    Picked3D(PickHit3D),
    /// Orbit, pan, or zoom changed the authoritative 3D camera.
    #[cfg(feature = "3d")]
    CameraChanged(ruviz::core::CameraSnapshot3D),
    /// A 3D view was reset to its initial camera.
    #[cfg(feature = "3d")]
    CameraReset,
}

/// Result of updating adapter-owned state.
///
/// Return [`into_task`](Self::into_task) (usually after mapping it into the
/// application's message enum) and inspect [`event`](Self::event) when the host
/// wants selection, pick, reset, or error callbacks.
pub struct Update {
    task: Task<Message>,
    events: Vec<Event>,
}

impl Update {
    fn none() -> Self {
        Self {
            task: Task::none(),
            events: Vec::new(),
        }
    }

    fn task(task: Task<Message>) -> Self {
        Self {
            task,
            events: Vec::new(),
        }
    }

    fn with_event(task: Task<Message>, event: Event) -> Self {
        Self {
            task,
            events: vec![event],
        }
    }

    /// First event produced by this update, if any.
    ///
    /// Use [`Self::events`] when an input can produce multiple events.
    pub fn event(&self) -> Option<&Event> {
        self.events.first()
    }

    /// All observable events produced by this update.
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// Consume the update and return its Iced task.
    pub fn into_task(self) -> Task<Message> {
        self.task
    }

    /// Consume the update and split it into task and event.
    pub fn into_parts(self) -> (Task<Message>, Vec<Event>) {
        (self.task, self.events)
    }
}

/// Opaque Iced message routed to [`PlotState::update`] or
/// [`Plot3DState::update`].
///
/// Keeping adapter internals opaque lets applications wrap one or many plot
/// messages without coupling themselves to render scheduling details.
#[derive(Clone, Debug)]
pub struct Message(MessageKind);

#[derive(Clone, Debug)]
enum MessageKind {
    Widget2D(WidgetEvent),
    Changed2D,
    Rendered2D {
        incarnation: StateIncarnation,
        request_id: ruviz::core::ScheduledRequestId,
        change_revision: ruviz::core::InteractiveChangeRevision,
        result: Result<StampedInteractiveFrame, PlottingError>,
    },
    Allocated2D {
        incarnation: StateIncarnation,
        request_id: ruviz::core::ScheduledRequestId,
        frame: Option<StampedInteractiveFrame>,
        source_alpha: AlphaMode,
        allocation: Result<image::Allocation, String>,
    },
    #[cfg(feature = "3d")]
    Widget3D(WidgetEvent),
    #[cfg(feature = "3d")]
    Rendered3D {
        incarnation: StateIncarnation,
        request_id: ruviz::core::ScheduledRequestId,
        result: Result<RenderedImage3D, PlottingError>,
    },
    #[cfg(feature = "3d")]
    Allocated3D {
        incarnation: StateIncarnation,
        request_id: ruviz::core::ScheduledRequestId,
        rendered: Option<RenderedImage3D>,
        source_alpha: AlphaMode,
        allocation: Result<image::Allocation, String>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PointerButton {
    Left,
    Middle,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum WidgetEvent {
    BoundsChanged {
        logical_size: (f64, f64),
    },
    ScaleFactorChanged(f32),
    PointerMoved(Option<(f64, f64)>),
    PointerPressed {
        position_px: (f64, f64),
        button: PointerButton,
    },
    PointerReleased {
        position_px: (f64, f64),
        button: PointerButton,
    },
    DoubleClick {
        position_px: (f64, f64),
    },
    Wheel {
        position_px: (f64, f64),
        delta_y: f32,
    },
    CancelDrag,
    Escape,
}

#[derive(Clone, Debug)]
struct PresentedImage {
    allocation: image::Allocation,
    size_px: (u32, u32),
    source_alpha: AlphaMode,
}

impl PresentedImage {
    fn handle(&self) -> &image::Handle {
        self.allocation.handle()
    }
}

fn iced_handle(image: &ruviz::core::Image) -> (image::Handle, AlphaMode) {
    let source_alpha = image.alpha_mode();
    let rgba = image.pixels_in_alpha_mode(AlphaMode::Straight).into_owned();
    (
        image::Handle::from_rgba(image.width, image.height, rgba),
        source_alpha,
    )
}
