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
use ruviz::core::{
    AlphaMode, HitResult, Image, PlotContextMenuAction, PlottingError, StampedInteractiveLayers,
    source_over_straight_rgba,
};

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
    /// Resize and redraw with plot gestures initially disabled.
    ///
    /// The context menu remains available for save, copy, and enabling
    /// interaction.
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
    /// Plot interaction was enabled or disabled from the context menu.
    InteractionToggled(bool),
    /// The currently presented image was copied to the native clipboard.
    ImageCopied,
    /// The currently presented image was saved as a PNG.
    ImageSaved,
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
        result: Result<PreparedLayers, PlottingError>,
    },
    Allocated2D {
        incarnation: StateIncarnation,
        request_id: ruviz::core::ScheduledRequestId,
        layers: Option<StampedInteractiveLayers>,
        allocations: Vec<(LayerRole, Result<image::Allocation, String>)>,
    },
    #[cfg(feature = "3d")]
    Widget3D(WidgetEvent),
    #[cfg(feature = "3d")]
    Rendered3D {
        incarnation: StateIncarnation,
        request_id: ruviz::core::ScheduledRequestId,
        result: Result<Rendered3DFrame, PlottingError>,
    },
    #[cfg(feature = "3d")]
    Allocated3D {
        incarnation: StateIncarnation,
        request_id: ruviz::core::ScheduledRequestId,
        rendered: Option<RenderedImage3D>,
        source_alpha: AlphaMode,
        allocation: Result<image::Allocation, String>,
    },
    ContextActionCompleted(Result<ContextActionCompletion, PlottingError>),
}

#[derive(Clone, Copy, Debug)]
enum ContextActionCompletion {
    ImageCopied,
    ImageSaved,
    SaveCancelled,
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
    /// Newest hover position observed while the presented frame was stale.
    ///
    /// Carries no geometry-derived data, so it is safe to route while a render
    /// is in flight; the state replays it once the next frame is current.
    HoverCoalesced {
        position_px: (f64, f64),
    },
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
    ContextMenuAction(PlotContextMenuAction),
    CancelDrag,
    Escape,
}

/// Which stacked layer an allocation belongs to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LayerRole {
    Base,
    Overlay,
}

/// One layer's upload plan, decided on the render worker thread.
///
/// `Reuse` is the fast path: the freshly rendered layer is `Arc`-identical to
/// the presented one, so the existing Iced allocation stays valid and nothing
/// is uploaded again.
#[derive(Clone, Debug)]
enum LayerUpload {
    Reuse(image::Allocation),
    New(image::Handle),
}

/// Layered render output plus the Iced handles built for it off the UI thread.
#[derive(Clone, Debug)]
struct PreparedLayers {
    layers: StampedInteractiveLayers,
    base: LayerUpload,
    overlay: Option<LayerUpload>,
}

/// Base and overlay images already presented, used to detect reusable layers.
#[derive(Clone, Debug, Default)]
struct PresentedLayerSources {
    base: Option<(Arc<Image>, image::Allocation)>,
    overlay: Option<(Arc<Image>, image::Allocation)>,
}

impl PresentedLayerSources {
    fn upload(
        previous: Option<&(Arc<Image>, image::Allocation)>,
        current: &Arc<Image>,
    ) -> LayerUpload {
        match previous {
            Some((image, allocation)) if Arc::ptr_eq(image, current) => {
                LayerUpload::Reuse(allocation.clone())
            }
            _ => LayerUpload::New(iced_handle(current)),
        }
    }

    fn plan(&self, layers: StampedInteractiveLayers) -> PreparedLayers {
        let base = Self::upload(self.base.as_ref(), layers.base.image());
        let overlay = layers
            .overlay
            .as_ref()
            .map(|overlay| Self::upload(self.overlay.as_ref(), overlay.image()));
        PreparedLayers {
            layers,
            base,
            overlay,
        }
    }
}

#[cfg(feature = "3d")]
#[derive(Clone, Debug)]
struct Rendered3DFrame {
    rendered: RenderedImage3D,
    handle: image::Handle,
}

#[derive(Clone, Debug)]
struct PresentedImage {
    allocation: image::Allocation,
    /// Overlay drawn over `allocation` in the same fitted rectangle. `None`
    /// means no crosshair, tooltip, selection, or dynamic annotation is active.
    overlay: Option<image::Allocation>,
    size_px: (u32, u32),
    source_alpha: AlphaMode,
}

impl PresentedImage {
    fn handle(&self) -> &image::Handle {
        self.allocation.handle()
    }

    fn overlay_handle(&self) -> Option<&image::Handle> {
        self.overlay.as_ref().map(image::Allocation::handle)
    }

    /// Capture the presented layers for export. Handle clones are cheap; the
    /// per-pixel composite happens later, off the UI thread.
    fn export_source(&self) -> Result<ExportSource, PlottingError> {
        Ok(ExportSource {
            base: straight_rgba_handle(self.handle())?,
            overlay: self
                .overlay_handle()
                .map(straight_rgba_handle)
                .transpose()?,
        })
    }
}

fn straight_rgba_handle(handle: &image::Handle) -> Result<image::Handle, PlottingError> {
    match handle {
        image::Handle::Rgba { .. } => Ok(handle.clone()),
        _ => Err(PlottingError::RenderError(
            "ruviz-iced presented an unexpected non-RGBA image handle".to_owned(),
        )),
    }
}

/// Presented layers retained for "Save PNG" and "Copy image".
#[derive(Clone, Debug)]
struct ExportSource {
    base: image::Handle,
    overlay: Option<image::Handle>,
}

impl ExportSource {
    /// Flatten the presented layers into one straight-alpha RGBA buffer.
    ///
    /// Export is not a hot path, so the composite that the layered
    /// presentation avoids per frame is paid here instead, once per action.
    fn compose(self) -> Result<(u32, u32, Vec<u8>), PlottingError> {
        let (width, height, base) = rgba_parts(self.base)?;
        let mut pixels = base.to_vec();
        if let Some(overlay) = self.overlay {
            let (overlay_width, overlay_height, overlay) = rgba_parts(overlay)?;
            if (overlay_width, overlay_height) != (width, height) {
                return Err(PlottingError::RenderError(
                    "ruviz-iced overlay layer does not match the base image size".to_owned(),
                ));
            }
            for (destination, source) in pixels.chunks_exact_mut(4).zip(overlay.chunks_exact(4)) {
                let composed = source_over_straight_rgba(
                    [
                        destination[0],
                        destination[1],
                        destination[2],
                        destination[3],
                    ],
                    [source[0], source[1], source[2], source[3]],
                );
                destination.copy_from_slice(&composed);
            }
        }
        Ok((width, height, pixels))
    }
}

fn rgba_parts(handle: image::Handle) -> Result<(u32, u32, bytes::Bytes), PlottingError> {
    match handle {
        image::Handle::Rgba {
            width,
            height,
            pixels,
            ..
        } => Ok((width, height, pixels)),
        _ => Err(PlottingError::RenderError(
            "ruviz-iced export received an unexpected non-RGBA image handle".to_owned(),
        )),
    }
}

/// Straight-alpha RGBA view of a retained ruviz image.
///
/// [`ruviz::core::Image`] is always straight alpha, so Iced can borrow its
/// pixels directly instead of copying a full frame per redraw.
struct ImagePixels(Arc<Image>);

impl AsRef<[u8]> for ImagePixels {
    fn as_ref(&self) -> &[u8] {
        &self.0.pixels
    }
}

fn iced_handle(image: &Arc<Image>) -> image::Handle {
    debug_assert_eq!(image.alpha_mode(), AlphaMode::Straight);
    let (width, height) = (image.width, image.height);
    image::Handle::from_rgba(
        width,
        height,
        bytes::Bytes::from_owner(ImagePixels(Arc::clone(image))),
    )
}

/// Owned-pixel variant for producers that do not retain an [`Arc<Image>`].
#[cfg(feature = "3d")]
fn iced_handle_owned(image: &Image) -> (image::Handle, AlphaMode) {
    let source_alpha = image.alpha_mode();
    let rgba = image.pixels_in_alpha_mode(AlphaMode::Straight).into_owned();
    (
        image::Handle::from_rgba(image.width, image.height, rgba),
        source_alpha,
    )
}
