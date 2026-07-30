use iced::advanced::image;
use iced::advanced::layout::{self, Layout};
use iced::advanced::mouse;
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::text;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::keyboard::{Key, key};
use iced::overlay::menu;
use iced::{Element, Event as IcedEvent, Length, Point, Rectangle, Size, Vector};

#[cfg(feature = "3d")]
use ruviz::core::CameraView3D;
use ruviz::core::{
    ImageFit, LogicalPoint, LogicalRect, PlotContextMenuAction, fitted_content_rect,
    logical_to_physical,
};

#[cfg(feature = "3d")]
use crate::state::Plot3DState;
use crate::state::PlotState;
use crate::{Message, MessageKind, PointerButton, PresentedImage, Sizing, WidgetEvent};

const SECONDARY_DRAG_THRESHOLD_PX: f64 = 3.0;
const MENU_WIDTH: f32 = 224.0;
const MENU_ITEM_HEIGHT: f32 = 30.0;
const MENU_SEPARATOR_HEIGHT: f32 = 9.0;
const MENU_PADDING_X: f32 = 10.0;
const SUBMENU_GAP: f32 = 2.0;
/// Logical pixels represented by one scrolled line, so line-based mice and
/// pixel-based trackpads reach the plot as the same unit.
const WHEEL_PIXELS_PER_LINE: f32 = 40.0;
/// Largest scroll distance honoured by a single wheel event.
///
/// Zooming is proportional (`exp`), so an unclamped trackpad flick of several
/// hundred pixels would multiply the view in one step. This bound matches the
/// core 3D wheel clamp, keeping 2D and 3D zoom equally paced.
const WHEEL_MAX_PIXELS: f32 = 120.0;

#[derive(Clone, Copy, Debug)]
struct PendingSecondary {
    anchor_px: (f64, f64),
    anchor_window: Point,
    forwarded: bool,
    crossed_threshold: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MenuTarget {
    Main(usize),
    Camera(usize),
}

#[derive(Debug, Default)]
struct ContextMenuState {
    open: bool,
    anchor: Point,
    hovered: Option<MenuTarget>,
    pressed: Option<MenuTarget>,
    camera_submenu_open: bool,
    main_scroll: f32,
    camera_scroll: f32,
}

#[derive(Debug, Default)]
struct WidgetState {
    last_size: Size,
    pressed: Option<PointerButton>,
    pending_secondary: Option<PendingSecondary>,
    previous_click: Option<mouse::Click>,
    hovered: bool,
    menu: ContextMenuState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PlotDimension {
    TwoD,
    #[cfg(feature = "3d")]
    ThreeD,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MenuEntryKind {
    Action(PlotContextMenuAction),
    Separator,
    #[cfg(feature = "3d")]
    CameraSubmenu,
}

#[derive(Clone, Debug)]
struct MenuEntry {
    label: String,
    kind: MenuEntryKind,
    enabled: bool,
}

/// Framework-native Iced widget presenting a retained 2D plot state.
pub struct PlotWidget {
    common: CommonWidget,
}

/// Build a 2D plot widget.
pub fn plot(state: &PlotState) -> PlotWidget {
    PlotWidget {
        common: CommonWidget::from_2d(state),
    }
}

impl PlotWidget {
    /// Build a widget from Elm-owned state.
    pub fn new(state: &PlotState) -> Self {
        plot(state)
    }
}

impl<Theme, Renderer> Widget<Message, Theme, Renderer> for PlotWidget
where
    Theme: menu::Catalog,
    Renderer: image::Renderer<Handle = image::Handle> + text::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<WidgetState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(WidgetState::default())
    }

    fn size(&self) -> Size<Length> {
        self.common.size()
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.common.layout(limits)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &IcedEvent,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        self.common
            .update(tree, event, layout, cursor, shell, MessageKind::Widget2D);
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.common.draw(renderer, layout.bounds(), viewport);
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if self.common.interaction_enabled
            && self.common.input_current
            && cursor.is_over(layout.bounds())
        {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        _layout: Layout<'a>,
        _renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, Renderer>> {
        self.common.overlay(tree, *viewport, translation)
    }
}

impl<'a, Theme, Renderer> From<PlotWidget> for Element<'a, Message, Theme, Renderer>
where
    Theme: menu::Catalog + 'a,
    Renderer: image::Renderer<Handle = image::Handle> + text::Renderer + 'a,
{
    fn from(widget: PlotWidget) -> Self {
        Element::new(widget)
    }
}

#[cfg(feature = "3d")]
/// Framework-native Iced widget presenting retained 3D state.
pub struct Plot3DWidget {
    common: CommonWidget,
}

#[cfg(feature = "3d")]
/// Build a 3D plot widget.
pub fn plot3d(state: &Plot3DState) -> Plot3DWidget {
    Plot3DWidget {
        common: CommonWidget::from_3d(state),
    }
}

#[cfg(feature = "3d")]
impl Plot3DWidget {
    /// Build a widget from Elm-owned state.
    pub fn new(state: &Plot3DState) -> Self {
        plot3d(state)
    }
}

#[cfg(feature = "3d")]
impl<Theme, Renderer> Widget<Message, Theme, Renderer> for Plot3DWidget
where
    Theme: menu::Catalog,
    Renderer: image::Renderer<Handle = image::Handle> + text::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<WidgetState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(WidgetState::default())
    }

    fn size(&self) -> Size<Length> {
        self.common.size()
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.common.layout(limits)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &IcedEvent,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        self.common
            .update(tree, event, layout, cursor, shell, MessageKind::Widget3D);
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.common.draw(renderer, layout.bounds(), viewport);
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if self.common.interaction_enabled
            && self.common.input_current
            && cursor.is_over(layout.bounds())
        {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        _layout: Layout<'a>,
        _renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, Renderer>> {
        self.common.overlay(tree, *viewport, translation)
    }
}

#[cfg(feature = "3d")]
impl<'a, Theme, Renderer> From<Plot3DWidget> for Element<'a, Message, Theme, Renderer>
where
    Theme: menu::Catalog + 'a,
    Renderer: image::Renderer<Handle = image::Handle> + text::Renderer + 'a,
{
    fn from(widget: Plot3DWidget) -> Self {
        Element::new(widget)
    }
}

struct CommonWidget {
    presented: Option<PresentedImage>,
    sizing: Sizing,
    fit: ImageFit,
    interaction_enabled: bool,
    dimension: PlotDimension,
    input_current: bool,
}

impl CommonWidget {
    fn from_2d(state: &PlotState) -> Self {
        Self {
            presented: state.presented.clone(),
            sizing: state.sizing,
            fit: state.fit,
            interaction_enabled: state.interaction_enabled,
            dimension: PlotDimension::TwoD,
            input_current: state
                .presented_stamp
                .is_some_and(|stamp| state.session().is_render_stamp_current(stamp)),
        }
    }

    #[cfg(feature = "3d")]
    fn from_3d(state: &Plot3DState) -> Self {
        Self {
            presented: state.presented.clone(),
            sizing: state.sizing,
            fit: state.fit,
            interaction_enabled: state.interaction_enabled,
            dimension: PlotDimension::ThreeD,
            input_current: state
                .presented_view
                .is_some_and(|stamp| state.session().is_view_current(stamp)),
        }
    }

    fn size(&self) -> Size<Length> {
        match self.sizing {
            Sizing::Fill => Size::new(Length::Fill, Length::Fill),
            Sizing::Fixed { width, height } => {
                Size::new(Length::Fixed(width), Length::Fixed(height))
            }
        }
    }

    fn layout(&self, limits: &layout::Limits) -> layout::Node {
        let requested = self.size();
        let fallback = self.sizing.logical_fallback();
        layout::Node::new(limits.resolve(
            requested.width,
            requested.height,
            Size::new(fallback.0 as f32, fallback.1 as f32),
        ))
    }

    fn update(
        &self,
        tree: &mut Tree,
        event: &IcedEvent,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        shell: &mut Shell<'_, Message>,
        wrap: fn(WidgetEvent) -> MessageKind,
    ) {
        let state = tree.state.downcast_mut::<WidgetState>();
        let bounds = layout.bounds();
        if state.last_size != bounds.size() {
            state.last_size = bounds.size();
            shell.publish(Message(wrap(WidgetEvent::BoundsChanged {
                logical_size: (f64::from(bounds.width), f64::from(bounds.height)),
            })));
        }

        if let IcedEvent::Window(iced::window::Event::Rescaled(scale)) = event {
            shell.publish(Message(wrap(WidgetEvent::ScaleFactorChanged(*scale))));
        }

        let mapped = self.map_cursor(bounds, cursor);
        if matches!(event, IcedEvent::Mouse(mouse::Event::CursorMoved { .. })) {
            state.hovered = mapped.is_some();
        }

        if let IcedEvent::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) = event {
            let (Some(position_px), Some(position)) = (mapped, cursor.position()) else {
                return;
            };
            if clear_capture_for_menu(state) {
                shell.publish(Message(wrap(WidgetEvent::CancelDrag)));
            }
            state.pressed = Some(PointerButton::Right);
            state.pending_secondary = Some(PendingSecondary {
                anchor_px: position_px,
                anchor_window: position,
                forwarded: false,
                crossed_threshold: false,
            });
            shell.capture_event();
            return;
        }

        if matches!(event, IcedEvent::Mouse(mouse::Event::CursorMoved { .. }))
            && let Some(mut pending) = state.pending_secondary
        {
            let Some(position_px) = mapped else {
                return;
            };
            let distance = secondary_distance(pending.anchor_px, position_px);
            pending.crossed_threshold |= distance >= SECONDARY_DRAG_THRESHOLD_PX;
            if pending.crossed_threshold
                && !pending.forwarded
                && self.interaction_enabled
                && self.input_current
            {
                shell.publish(Message(wrap(WidgetEvent::PointerPressed {
                    position_px: pending.anchor_px,
                    button: PointerButton::Right,
                })));
                pending.forwarded = true;
            }
            if pending.forwarded {
                shell.publish(Message(wrap(WidgetEvent::PointerMoved(Some(position_px)))));
            }
            state.pending_secondary = Some(pending);
            shell.capture_event();
            return;
        }

        if let IcedEvent::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)) = event {
            let Some(mut pending) = state.pending_secondary.take() else {
                return;
            };
            state.pressed = None;
            let release_px = mapped;
            if let Some(position_px) = release_px {
                pending.crossed_threshold |= secondary_distance(pending.anchor_px, position_px)
                    >= SECONDARY_DRAG_THRESHOLD_PX;
            }
            if pending.forwarded {
                match release_px {
                    Some(position_px) => {
                        shell.publish(Message(wrap(WidgetEvent::PointerReleased {
                            position_px,
                            button: PointerButton::Right,
                        })));
                    }
                    None => shell.publish(Message(wrap(WidgetEvent::CancelDrag))),
                }
            } else if pending.crossed_threshold
                && self.interaction_enabled
                && self.input_current
                && let Some(position_px) = release_px
            {
                shell.publish(Message(wrap(WidgetEvent::PointerPressed {
                    position_px: pending.anchor_px,
                    button: PointerButton::Right,
                })));
                shell.publish(Message(wrap(WidgetEvent::PointerMoved(Some(position_px)))));
                shell.publish(Message(wrap(WidgetEvent::PointerReleased {
                    position_px,
                    button: PointerButton::Right,
                })));
            } else if !pending.crossed_threshold && release_px.is_some() {
                state
                    .menu
                    .open_at(cursor.position().unwrap_or(pending.anchor_window));
                shell.invalidate_layout();
                shell.request_redraw();
            }
            shell.capture_event();
            return;
        }

        if let IcedEvent::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) = event
            && state.hovered
            && (matches!(key.as_ref(), Key::Named(key::Named::ContextMenu))
                || (matches!(key.as_ref(), Key::Named(key::Named::F10)) && modifiers.shift()))
        {
            if clear_capture_for_menu(state) {
                shell.publish(Message(wrap(WidgetEvent::CancelDrag)));
            }
            state.menu.open_at(
                cursor
                    .position()
                    .unwrap_or_else(|| Point::new(bounds.center_x(), bounds.center_y())),
            );
            shell.invalidate_layout();
            shell.request_redraw();
            shell.capture_event();
            return;
        }

        if !self.interaction_enabled {
            return;
        }

        if !self.input_current && !routes_while_frame_is_stale(state, event, mapped) {
            // Coalesce instead of dropping: a hover move that cannot be resolved
            // against stale geometry still records where the pointer is now, so
            // the next render uses the newest position rather than the one from
            // when the in-flight render started. No event is derived here.
            if matches!(event, IcedEvent::Mouse(mouse::Event::CursorMoved { .. }))
                && let Some(position_px) = mapped
            {
                shell.publish(Message(wrap(WidgetEvent::HoverCoalesced { position_px })));
            }
            return;
        }

        match event {
            IcedEvent::Mouse(mouse::Event::CursorMoved { .. }) => {
                shell.publish(Message(wrap(WidgetEvent::PointerMoved(mapped))));
                if state.pressed.is_some() {
                    shell.capture_event();
                }
            }
            IcedEvent::Mouse(mouse::Event::CursorLeft)
            | IcedEvent::Window(iced::window::Event::Unfocused) => {
                state.hovered = false;
                state.pending_secondary = None;
                if state.pressed.take().is_some() {
                    shell.publish(Message(wrap(WidgetEvent::CancelDrag)));
                }
                shell.publish(Message(wrap(WidgetEvent::PointerMoved(None))));
            }
            IcedEvent::Mouse(mouse::Event::ButtonPressed(button)) => {
                let Some(position_px) = mapped else {
                    return;
                };
                let Some(button) = pointer_button(*button) else {
                    return;
                };
                debug_assert_ne!(button, PointerButton::Right);
                state.pressed = Some(button);
                shell.publish(Message(wrap(WidgetEvent::PointerPressed {
                    position_px,
                    button,
                })));
                shell.capture_event();

                if button == PointerButton::Left
                    && let Some(position) = cursor.position()
                {
                    let click =
                        mouse::Click::new(position, mouse::Button::Left, state.previous_click);
                    if click.kind() == mouse::click::Kind::Double {
                        shell.publish(Message(wrap(WidgetEvent::DoubleClick { position_px })));
                    }
                    state.previous_click = Some(click);
                }
            }
            IcedEvent::Mouse(mouse::Event::ButtonReleased(button)) => {
                let Some(button) = pointer_button(*button) else {
                    return;
                };
                debug_assert_ne!(button, PointerButton::Right);
                if state.pressed != Some(button) {
                    return;
                }
                state.pressed = None;
                match mapped {
                    Some(position_px) => {
                        shell.publish(Message(wrap(WidgetEvent::PointerReleased {
                            position_px,
                            button,
                        })));
                    }
                    None => shell.publish(Message(wrap(WidgetEvent::CancelDrag))),
                }
                shell.capture_event();
            }
            IcedEvent::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let Some(position_px) = mapped else {
                    return;
                };
                let delta_y = wheel_delta_px(*delta);
                shell.publish(Message(wrap(WidgetEvent::Wheel {
                    position_px,
                    delta_y,
                })));
                shell.capture_event();
            }
            IcedEvent::Keyboard(iced::keyboard::Event::KeyPressed { key, .. })
                if matches!(key.as_ref(), Key::Named(key::Named::Escape)) && owns_escape(state) =>
            {
                state.pressed = None;
                state.pending_secondary = None;
                shell.publish(Message(wrap(WidgetEvent::Escape)));
                shell.capture_event();
            }
            _ => {}
        }
    }

    fn overlay<'a, Theme, Renderer>(
        &'a mut self,
        tree: &'a mut Tree,
        viewport: Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, Renderer>>
    where
        Theme: menu::Catalog,
        Renderer: image::Renderer<Handle = image::Handle> + text::Renderer,
    {
        let state = tree.state.downcast_mut::<WidgetState>();
        if !state.menu.open {
            return None;
        }
        Some(overlay::Element::new(Box::new(ContextMenuOverlay {
            state: &mut state.menu,
            main_entries: self.menu_entries(),
            camera_entries: camera_menu_entries(),
            viewport,
            dimension: self.dimension,
            translation,
        })))
    }

    fn menu_entries(&self) -> Vec<MenuEntry> {
        let entries = vec![
            MenuEntry::action("Reset view", PlotContextMenuAction::ResetView),
            MenuEntry::action("Fit to content", PlotContextMenuAction::FitToContent),
            MenuEntry::separator(),
            MenuEntry::action_enabled(
                "Save PNG…",
                PlotContextMenuAction::SaveImage,
                self.presented.is_some(),
            ),
            MenuEntry::action_enabled(
                "Copy image",
                PlotContextMenuAction::CopyImage,
                self.presented.is_some(),
            ),
            MenuEntry::separator(),
            MenuEntry::action(
                if self.interaction_enabled {
                    "Disable interaction"
                } else {
                    "Enable interaction"
                },
                PlotContextMenuAction::ToggleInteraction,
            ),
        ];
        #[cfg(feature = "3d")]
        {
            let mut entries = entries;
            if self.dimension == PlotDimension::ThreeD {
                entries.push(MenuEntry::separator());
                entries.push(MenuEntry {
                    label: "Camera view  ›".to_owned(),
                    kind: MenuEntryKind::CameraSubmenu,
                    enabled: true,
                });
            }
            entries
        }
        #[cfg(not(feature = "3d"))]
        entries
    }

    fn draw(
        &self,
        renderer: &mut impl image::Renderer<Handle = image::Handle>,
        outer: Rectangle,
        viewport: &Rectangle,
    ) {
        let Some(presented) = &self.presented else {
            return;
        };
        let content = fitted_rect(outer, presented.size_px, self.fit);
        let Some(clip) = outer.intersection(viewport) else {
            return;
        };
        renderer.draw_image(image::Image::new(presented.handle().clone()), content, clip);
        // The crosshair, tooltip, and selection layer is stacked here instead of
        // being composited into the base image on the CPU every frame.
        if let Some(overlay) = presented.overlay_handle() {
            renderer.draw_image(image::Image::new(overlay.clone()), content, clip);
        }
    }

    fn map_cursor(&self, outer: Rectangle, cursor: mouse::Cursor) -> Option<(f64, f64)> {
        let presented = self.presented.as_ref()?;
        let position = cursor.position()?;
        map_point(outer, presented.size_px, self.fit, position)
    }
}

impl ContextMenuState {
    fn open_at(&mut self, anchor: Point) {
        self.open = true;
        self.anchor = anchor;
        self.hovered = None;
        self.pressed = None;
        self.camera_submenu_open = false;
        self.main_scroll = 0.0;
        self.camera_scroll = 0.0;
    }

    fn close(&mut self) {
        self.open = false;
        self.hovered = None;
        self.pressed = None;
        self.camera_submenu_open = false;
        self.main_scroll = 0.0;
        self.camera_scroll = 0.0;
    }
}

impl MenuEntry {
    fn action(label: &str, action: PlotContextMenuAction) -> Self {
        Self::action_enabled(label, action, true)
    }

    fn action_enabled(label: &str, action: PlotContextMenuAction, enabled: bool) -> Self {
        Self {
            label: label.to_owned(),
            kind: MenuEntryKind::Action(action),
            enabled,
        }
    }

    fn separator() -> Self {
        Self {
            label: String::new(),
            kind: MenuEntryKind::Separator,
            enabled: false,
        }
    }
}

/// Normalize an Iced scroll delta into clamped logical pixels.
fn wheel_delta_px(delta: mouse::ScrollDelta) -> f32 {
    let pixels = match delta {
        mouse::ScrollDelta::Lines { y, .. } => y * WHEEL_PIXELS_PER_LINE,
        mouse::ScrollDelta::Pixels { y, .. } => y,
    };
    if pixels.is_finite() {
        pixels.clamp(-WHEEL_MAX_PIXELS, WHEEL_MAX_PIXELS)
    } else {
        0.0
    }
}

fn secondary_distance(anchor: (f64, f64), current: (f64, f64)) -> f64 {
    (current.0 - anchor.0).hypot(current.1 - anchor.1)
}

fn clear_capture_for_menu(state: &mut WidgetState) -> bool {
    let core_drag_active = state
        .pending_secondary
        .is_some_and(|pending| pending.forwarded)
        || state
            .pressed
            .is_some_and(|button| button != PointerButton::Right);
    state.pending_secondary = None;
    state.pressed = None;
    core_drag_active
}

struct ContextMenuOverlay<'a> {
    state: &'a mut ContextMenuState,
    main_entries: Vec<MenuEntry>,
    camera_entries: Vec<MenuEntry>,
    viewport: Rectangle,
    dimension: PlotDimension,
    translation: Vector,
}

#[derive(Debug)]
struct MenuGeometry {
    main_panel: Rectangle,
    main_rows: Vec<Rectangle>,
    camera_panel: Option<Rectangle>,
    camera_rows: Vec<Rectangle>,
}

impl ContextMenuOverlay<'_> {
    fn geometry(&self) -> MenuGeometry {
        menu_geometry(
            self.state.anchor + self.translation,
            self.viewport,
            &self.main_entries,
            &self.camera_entries,
            self.state.camera_submenu_open,
            self.state.main_scroll,
            self.state.camera_scroll,
        )
    }

    fn target_at(&self, position: Point) -> Option<MenuTarget> {
        let geometry = self.geometry();
        if geometry
            .camera_panel
            .is_some_and(|panel| panel.contains(position))
        {
            return geometry
                .camera_rows
                .iter()
                .position(|row| row.contains(position))
                .filter(|index| {
                    self.camera_entries[*index].enabled
                        && !matches!(self.camera_entries[*index].kind, MenuEntryKind::Separator)
                })
                .map(MenuTarget::Camera);
        }
        if geometry.main_panel.contains(position) {
            return geometry
                .main_rows
                .iter()
                .position(|row| row.contains(position))
                .filter(|index| {
                    self.main_entries[*index].enabled
                        && !matches!(self.main_entries[*index].kind, MenuEntryKind::Separator)
                })
                .map(MenuTarget::Main);
        }
        None
    }

    fn action_for_target(&self, target: MenuTarget) -> Option<PlotContextMenuAction> {
        let entry = match target {
            MenuTarget::Main(index) => self.main_entries.get(index),
            MenuTarget::Camera(index) => self.camera_entries.get(index),
        }?;
        if !entry.enabled {
            return None;
        }
        match entry.kind {
            MenuEntryKind::Action(action) => Some(action),
            MenuEntryKind::Separator => None,
            #[cfg(feature = "3d")]
            MenuEntryKind::CameraSubmenu => None,
        }
    }

    fn update_hover(&mut self, target: Option<MenuTarget>) -> bool {
        let before = (self.state.hovered, self.state.camera_submenu_open);
        self.state.hovered = target;
        match target {
            #[cfg(feature = "3d")]
            Some(MenuTarget::Main(index))
                if matches!(self.main_entries[index].kind, MenuEntryKind::CameraSubmenu) =>
            {
                self.state.camera_submenu_open = true;
            }
            Some(MenuTarget::Camera(_)) => self.state.camera_submenu_open = true,
            Some(MenuTarget::Main(_)) => self.state.camera_submenu_open = false,
            None => {}
        }
        before != (self.state.hovered, self.state.camera_submenu_open)
    }

    fn keyboard_targets(&self) -> Vec<MenuTarget> {
        if self.state.camera_submenu_open {
            self.camera_entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| {
                    entry.enabled && matches!(entry.kind, MenuEntryKind::Action(_))
                })
                .map(|(index, _)| MenuTarget::Camera(index))
                .collect()
        } else {
            self.main_entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| {
                    entry.enabled && !matches!(entry.kind, MenuEntryKind::Separator)
                })
                .map(|(index, _)| MenuTarget::Main(index))
                .collect()
        }
    }

    fn move_keyboard_hover(&mut self, forward: bool) {
        let targets = self.keyboard_targets();
        if targets.is_empty() {
            return;
        }
        let current = self
            .state
            .hovered
            .and_then(|hovered| targets.iter().position(|target| *target == hovered));
        let index = match (current, forward) {
            (Some(index), true) => (index + 1) % targets.len(),
            (Some(0), false) | (None, false) => targets.len() - 1,
            (Some(index), false) => index - 1,
            (None, true) => 0,
        };
        let target = targets[index];
        self.update_hover(Some(target));
        self.ensure_target_visible(target);
    }

    fn ensure_target_visible(&mut self, target: MenuTarget) {
        let geometry = self.geometry();
        let (panel, row, camera) = match target {
            MenuTarget::Main(index) => (
                Some(geometry.main_panel),
                geometry.main_rows.get(index).copied(),
                false,
            ),
            MenuTarget::Camera(index) => (
                geometry.camera_panel,
                geometry.camera_rows.get(index).copied(),
                true,
            ),
        };
        let (Some(panel), Some(row)) = (panel, row) else {
            return;
        };
        let delta = if row.y < panel.y {
            row.y - panel.y
        } else if row.y + row.height > panel.y + panel.height {
            row.y + row.height - panel.y - panel.height
        } else {
            0.0
        };
        if delta != 0.0 {
            self.scroll_menu(camera, delta);
        }
    }

    fn scroll_menu(&mut self, camera: bool, delta: f32) -> bool {
        let (entries, visible_height, scroll) = if camera {
            let Some(panel) = self.geometry().camera_panel else {
                return false;
            };
            (
                self.camera_entries.as_slice(),
                panel.height,
                &mut self.state.camera_scroll,
            )
        } else {
            let panel = self.geometry().main_panel;
            (
                self.main_entries.as_slice(),
                panel.height,
                &mut self.state.main_scroll,
            )
        };
        let before = *scroll;
        let maximum = (panel_height(entries) - visible_height).max(0.0);
        *scroll = (*scroll + delta).clamp(0.0, maximum);
        before != *scroll
    }

    fn close_and_refresh(&mut self, shell: &mut Shell<'_, Message>) {
        self.state.close();
        shell.invalidate_layout();
        shell.request_redraw();
    }

    fn request_redraw_if_changed(
        &self,
        before: (Option<MenuTarget>, bool),
        shell: &mut Shell<'_, Message>,
    ) {
        if before != (self.state.hovered, self.state.camera_submenu_open) {
            shell.request_redraw();
        }
    }
}

impl<Theme, Renderer> iced::advanced::Overlay<Message, Theme, Renderer> for ContextMenuOverlay<'_>
where
    Theme: menu::Catalog,
    Renderer: image::Renderer<Handle = image::Handle> + text::Renderer,
{
    fn layout(&mut self, _renderer: &Renderer, bounds: Size) -> layout::Node {
        layout::Node::new(bounds)
    }

    fn update(
        &mut self,
        event: &IcedEvent,
        _layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        match event {
            IcedEvent::Mouse(mouse::Event::CursorMoved { .. }) => {
                if self.update_hover(
                    cursor
                        .position()
                        .and_then(|position| self.target_at(position)),
                ) {
                    shell.request_redraw();
                }
                shell.capture_event();
            }
            IcedEvent::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let target = cursor
                    .position()
                    .and_then(|position| self.target_at(position));
                if target.is_none() {
                    self.close_and_refresh(shell);
                } else {
                    self.state.pressed = target;
                    if self.update_hover(target) {
                        shell.request_redraw();
                    }
                }
                shell.capture_event();
            }
            IcedEvent::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let target = cursor
                    .position()
                    .and_then(|position| self.target_at(position));
                let pressed = self.state.pressed.take();
                if target == pressed
                    && let Some(target) = target
                    && let Some(action) = self.action_for_target(target)
                {
                    self.close_and_refresh(shell);
                    shell.publish(Message(context_action_message(self.dimension, action)));
                }
                shell.capture_event();
            }
            IcedEvent::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                if let Some(position) = cursor.position() {
                    self.state.open_at(position - self.translation);
                    shell.invalidate_layout();
                    shell.request_redraw();
                } else {
                    self.close_and_refresh(shell);
                }
                shell.capture_event();
            }
            IcedEvent::Mouse(mouse::Event::CursorLeft) => {
                if self.update_hover(None) {
                    shell.request_redraw();
                }
                shell.capture_event();
            }
            IcedEvent::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let camera = cursor.position().is_some_and(|position| {
                    self.geometry()
                        .camera_panel
                        .is_some_and(|panel| panel.contains(position))
                });
                let amount = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => -*y * MENU_ITEM_HEIGHT,
                    mouse::ScrollDelta::Pixels { y, .. } => -*y,
                };
                if self.scroll_menu(camera, amount) {
                    self.state.pressed = None;
                    self.state.hovered = cursor
                        .position()
                        .and_then(|position| self.target_at(position));
                    shell.request_redraw();
                }
                shell.capture_event();
            }
            IcedEvent::Mouse(_) | IcedEvent::Touch(_) => {
                shell.capture_event();
            }
            IcedEvent::Keyboard(iced::keyboard::Event::KeyPressed { key, .. })
                if matches!(key.as_ref(), Key::Named(key::Named::Escape)) =>
            {
                self.close_and_refresh(shell);
                shell.capture_event();
            }
            IcedEvent::Keyboard(iced::keyboard::Event::KeyPressed { key, .. })
                if matches!(key.as_ref(), Key::Named(key::Named::ArrowDown)) =>
            {
                let before = (self.state.hovered, self.state.camera_submenu_open);
                self.move_keyboard_hover(true);
                self.request_redraw_if_changed(before, shell);
                shell.capture_event();
            }
            IcedEvent::Keyboard(iced::keyboard::Event::KeyPressed { key, .. })
                if matches!(key.as_ref(), Key::Named(key::Named::ArrowUp)) =>
            {
                let before = (self.state.hovered, self.state.camera_submenu_open);
                self.move_keyboard_hover(false);
                self.request_redraw_if_changed(before, shell);
                shell.capture_event();
            }
            IcedEvent::Keyboard(iced::keyboard::Event::KeyPressed { key, .. })
                if matches!(key.as_ref(), Key::Named(key::Named::ArrowRight)) =>
            {
                let before = (self.state.hovered, self.state.camera_submenu_open);
                #[cfg(feature = "3d")]
                if let Some(MenuTarget::Main(index)) = self.state.hovered
                    && matches!(self.main_entries[index].kind, MenuEntryKind::CameraSubmenu)
                {
                    self.state.camera_submenu_open = true;
                    self.state.hovered = self
                        .camera_entries
                        .iter()
                        .position(|entry| matches!(entry.kind, MenuEntryKind::Action(_)))
                        .map(MenuTarget::Camera);
                    if let Some(target) = self.state.hovered {
                        self.ensure_target_visible(target);
                    }
                }
                self.request_redraw_if_changed(before, shell);
                shell.capture_event();
            }
            IcedEvent::Keyboard(iced::keyboard::Event::KeyPressed { key, .. })
                if matches!(key.as_ref(), Key::Named(key::Named::ArrowLeft)) =>
            {
                let before = (self.state.hovered, self.state.camera_submenu_open);
                if self.state.camera_submenu_open {
                    self.state.camera_submenu_open = false;
                    #[cfg(feature = "3d")]
                    {
                        self.state.hovered = self
                            .main_entries
                            .iter()
                            .position(|entry| matches!(entry.kind, MenuEntryKind::CameraSubmenu))
                            .map(MenuTarget::Main);
                        if let Some(target) = self.state.hovered {
                            self.ensure_target_visible(target);
                        }
                    }
                }
                self.request_redraw_if_changed(before, shell);
                shell.capture_event();
            }
            IcedEvent::Keyboard(iced::keyboard::Event::KeyPressed { key, .. })
                if matches!(
                    key.as_ref(),
                    Key::Named(key::Named::Enter) | Key::Named(key::Named::Space)
                ) =>
            {
                if let Some(target) = self.state.hovered {
                    #[cfg(feature = "3d")]
                    if let MenuTarget::Main(index) = target
                        && matches!(self.main_entries[index].kind, MenuEntryKind::CameraSubmenu)
                    {
                        let before = (self.state.hovered, self.state.camera_submenu_open);
                        self.state.camera_submenu_open = true;
                        self.state.hovered = self
                            .camera_entries
                            .iter()
                            .position(|entry| matches!(entry.kind, MenuEntryKind::Action(_)))
                            .map(MenuTarget::Camera);
                        if let Some(target) = self.state.hovered {
                            self.ensure_target_visible(target);
                        }
                        self.request_redraw_if_changed(before, shell);
                        shell.capture_event();
                        return;
                    }
                    if let Some(action) = self.action_for_target(target) {
                        self.close_and_refresh(shell);
                        shell.publish(Message(context_action_message(self.dimension, action)));
                    }
                }
                shell.capture_event();
            }
            IcedEvent::Window(iced::window::Event::Unfocused) => {
                shell.publish(Message(context_cancel_message(self.dimension)));
                self.close_and_refresh(shell);
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        _layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        cursor
            .position()
            .map_or(mouse::Interaction::None, |position| {
                if self.target_at(position).is_some() {
                    mouse::Interaction::Pointer
                } else {
                    mouse::Interaction::None
                }
            })
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        _defaults: &renderer::Style,
        _layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let class = <Theme as menu::Catalog>::default();
        let style = <Theme as menu::Catalog>::style(theme, &class);
        let geometry = self.geometry();
        draw_menu_panel(
            renderer,
            &style,
            geometry.main_panel,
            &geometry.main_rows,
            &self.main_entries,
            self.state.hovered,
            false,
            self.viewport,
        );
        if let Some(panel) = geometry.camera_panel {
            draw_menu_panel(
                renderer,
                &style,
                panel,
                &geometry.camera_rows,
                &self.camera_entries,
                self.state.hovered,
                true,
                self.viewport,
            );
        }
        let _ = cursor;
    }
}

fn context_action_message(dimension: PlotDimension, action: PlotContextMenuAction) -> MessageKind {
    match dimension {
        PlotDimension::TwoD => MessageKind::Widget2D(WidgetEvent::ContextMenuAction(action)),
        #[cfg(feature = "3d")]
        PlotDimension::ThreeD => MessageKind::Widget3D(WidgetEvent::ContextMenuAction(action)),
    }
}

fn context_cancel_message(dimension: PlotDimension) -> MessageKind {
    match dimension {
        PlotDimension::TwoD => MessageKind::Widget2D(WidgetEvent::CancelDrag),
        #[cfg(feature = "3d")]
        PlotDimension::ThreeD => MessageKind::Widget3D(WidgetEvent::CancelDrag),
    }
}

fn camera_menu_entries() -> Vec<MenuEntry> {
    #[cfg(feature = "3d")]
    {
        vec![
            MenuEntry::action(
                "Isometric",
                PlotContextMenuAction::CameraView(CameraView3D::Isometric),
            ),
            MenuEntry::action(
                "Front",
                PlotContextMenuAction::CameraView(CameraView3D::Front),
            ),
            MenuEntry::action(
                "Back",
                PlotContextMenuAction::CameraView(CameraView3D::Back),
            ),
            MenuEntry::action(
                "Left",
                PlotContextMenuAction::CameraView(CameraView3D::Left),
            ),
            MenuEntry::action(
                "Right",
                PlotContextMenuAction::CameraView(CameraView3D::Right),
            ),
            MenuEntry::action("Top", PlotContextMenuAction::CameraView(CameraView3D::Top)),
            MenuEntry::action(
                "Bottom",
                PlotContextMenuAction::CameraView(CameraView3D::Bottom),
            ),
        ]
    }
    #[cfg(not(feature = "3d"))]
    {
        Vec::new()
    }
}

fn entry_height(entry: &MenuEntry) -> f32 {
    if matches!(entry.kind, MenuEntryKind::Separator) {
        MENU_SEPARATOR_HEIGHT
    } else {
        MENU_ITEM_HEIGHT
    }
}

fn panel_height(entries: &[MenuEntry]) -> f32 {
    entries.iter().map(entry_height).sum()
}

fn clamp_panel_origin(value: f32, start: f32, extent: f32, panel_extent: f32) -> f32 {
    let end = (start + extent - panel_extent).max(start);
    value.clamp(start, end)
}

fn row_rectangles(panel: Rectangle, entries: &[MenuEntry], scroll_offset: f32) -> Vec<Rectangle> {
    let mut y = panel.y - scroll_offset;
    entries
        .iter()
        .map(|entry| {
            let height = entry_height(entry);
            let row = Rectangle {
                x: panel.x,
                y,
                width: panel.width,
                height,
            };
            y += height;
            row
        })
        .collect()
}

fn menu_geometry(
    anchor: Point,
    viewport: Rectangle,
    main_entries: &[MenuEntry],
    camera_entries: &[MenuEntry],
    camera_open: bool,
    main_scroll: f32,
    camera_scroll: f32,
) -> MenuGeometry {
    let main_height = panel_height(main_entries);
    let main_panel = Rectangle {
        x: clamp_panel_origin(anchor.x, viewport.x, viewport.width, MENU_WIDTH),
        y: clamp_panel_origin(anchor.y, viewport.y, viewport.height, main_height),
        width: MENU_WIDTH.min(viewport.width),
        height: main_height.min(viewport.height),
    };
    let main_scroll = main_scroll.clamp(0.0, (main_height - main_panel.height).max(0.0));
    let main_rows = row_rectangles(main_panel, main_entries, main_scroll);
    let camera_row = main_entries
        .iter()
        .position(|entry| {
            #[cfg(feature = "3d")]
            {
                matches!(entry.kind, MenuEntryKind::CameraSubmenu)
            }
            #[cfg(not(feature = "3d"))]
            {
                let _ = entry;
                false
            }
        })
        .and_then(|index| main_rows.get(index).copied());
    let (camera_panel, camera_rows) = if camera_open
        && !camera_entries.is_empty()
        && let Some(camera_row) = camera_row
    {
        let height = panel_height(camera_entries);
        let right = main_panel.x + main_panel.width + SUBMENU_GAP;
        let left = main_panel.x - MENU_WIDTH - SUBMENU_GAP;
        let x = if right + MENU_WIDTH <= viewport.x + viewport.width {
            right
        } else {
            left.max(viewport.x)
        };
        let panel = Rectangle {
            x,
            y: clamp_panel_origin(camera_row.y, viewport.y, viewport.height, height),
            width: MENU_WIDTH.min(viewport.width),
            height: height.min(viewport.height),
        };
        let camera_scroll = camera_scroll.clamp(0.0, (height - panel.height).max(0.0));
        let rows = row_rectangles(panel, camera_entries, camera_scroll);
        (Some(panel), rows)
    } else {
        (None, Vec::new())
    };
    MenuGeometry {
        main_panel,
        main_rows,
        camera_panel,
        camera_rows,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_menu_panel<Renderer>(
    renderer: &mut Renderer,
    style: &menu::Style,
    panel: Rectangle,
    rows: &[Rectangle],
    entries: &[MenuEntry],
    hovered: Option<MenuTarget>,
    camera: bool,
    viewport: Rectangle,
) where
    Renderer: text::Renderer,
{
    renderer.fill_quad(
        renderer::Quad {
            bounds: panel,
            border: style.border,
            shadow: style.shadow,
            ..renderer::Quad::default()
        },
        style.background,
    );
    for (index, (entry, row)) in entries.iter().zip(rows).enumerate() {
        let Some(visible_row) = row.intersection(&panel) else {
            continue;
        };
        if matches!(entry.kind, MenuEntryKind::Separator) {
            let separator = Rectangle {
                x: row.x + MENU_PADDING_X,
                y: row.center_y(),
                width: (row.width - MENU_PADDING_X * 2.0).max(0.0),
                height: 1.0,
            };
            let Some(separator) = separator.intersection(&panel) else {
                continue;
            };
            renderer.fill_quad(
                renderer::Quad {
                    bounds: separator,
                    ..renderer::Quad::default()
                },
                style.border.color,
            );
            continue;
        }
        let target = if camera {
            MenuTarget::Camera(index)
        } else {
            MenuTarget::Main(index)
        };
        let selected = hovered == Some(target);
        if selected {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: visible_row.x + style.border.width,
                        y: visible_row.y,
                        width: (visible_row.width - style.border.width * 2.0).max(0.0),
                        height: visible_row.height,
                    },
                    border: iced::Border {
                        radius: style.border.radius,
                        ..iced::Border::default()
                    },
                    ..renderer::Quad::default()
                },
                style.selected_background,
            );
        }
        let size = iced::Pixels(14.0);
        let mut text_color = if selected {
            style.selected_text_color
        } else {
            style.text_color
        };
        if !entry.enabled {
            text_color.a *= 0.45;
        }
        renderer.fill_text(
            text::Text {
                content: entry.label.clone(),
                bounds: Size::new(row.width - MENU_PADDING_X * 2.0, row.height),
                size,
                line_height: text::LineHeight::default(),
                font: renderer.default_font(),
                align_x: text::Alignment::Default,
                align_y: iced::alignment::Vertical::Center,
                shaping: text::Shaping::Advanced,
                wrapping: text::Wrapping::None,
            },
            Point::new(row.x + MENU_PADDING_X, row.center_y()),
            text_color,
            panel.intersection(&viewport).unwrap_or_default(),
        );
    }
}

fn owns_escape(state: &WidgetState) -> bool {
    state.hovered || state.pressed.is_some()
}

fn routes_while_frame_is_stale(
    state: &WidgetState,
    event: &IcedEvent,
    mapped: Option<(f64, f64)>,
) -> bool {
    match event {
        IcedEvent::Mouse(mouse::Event::CursorMoved { .. }) => {
            state.pressed.is_some() || mapped.is_none()
        }
        IcedEvent::Mouse(mouse::Event::ButtonReleased(button)) => {
            pointer_button(*button).is_some_and(|button| state.pressed == Some(button))
        }
        // Wheel navigation only uses fitted pixel coordinates, not stale hit
        // geometry, and must remain responsive while its redraw catches up.
        IcedEvent::Mouse(mouse::Event::WheelScrolled { .. }) => true,
        IcedEvent::Mouse(mouse::Event::CursorLeft)
        | IcedEvent::Window(iced::window::Event::Unfocused) => true,
        IcedEvent::Keyboard(iced::keyboard::Event::KeyPressed { key, .. }) => {
            matches!(key.as_ref(), Key::Named(key::Named::Escape)) && owns_escape(state)
        }
        _ => false,
    }
}

fn map_point(
    outer: Rectangle,
    image_size_px: (u32, u32),
    fit: ImageFit,
    position: iced::Point,
) -> Option<(f64, f64)> {
    if fit == ImageFit::Cover && !outer.contains(position) {
        return None;
    }
    let content = fitted_rect(outer, image_size_px, fit);
    logical_to_physical(
        LogicalRect::new(
            f64::from(content.x),
            f64::from(content.y),
            f64::from(content.width),
            f64::from(content.height),
        ),
        LogicalPoint::new(f64::from(position.x), f64::from(position.y)),
        image_size_px,
    )
}

fn fitted_rect(outer: Rectangle, size_px: (u32, u32), fit: ImageFit) -> Rectangle {
    let fitted = fitted_content_rect(
        LogicalRect::new(
            f64::from(outer.x),
            f64::from(outer.y),
            f64::from(outer.width),
            f64::from(outer.height),
        ),
        size_px,
        fit,
    );
    Rectangle {
        x: fitted.x as f32,
        y: fitted.y as f32,
        width: fitted.width as f32,
        height: fitted.height as f32,
    }
}

fn pointer_button(button: mouse::Button) -> Option<PointerButton> {
    match button {
        mouse::Button::Left => Some(PointerButton::Left),
        mouse::Button::Middle => Some(PointerButton::Middle),
        mouse::Button::Right => Some(PointerButton::Right),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviz::core::physical_backing_size;

    /// Deterministic software renderer for widget tests.
    ///
    /// The dev-dependency enables `wgpu`, so `iced::Renderer` is the fallback
    /// renderer. Requesting the `tiny-skia` backend by name makes the wgpu
    /// branch decline before it touches a GPU adapter.
    fn test_renderer() -> iced::Renderer {
        use iced::advanced::renderer::Headless;

        futures::executor::block_on(iced::Renderer::new(
            iced::Font::default(),
            iced::Pixels(16.0),
            Some("tiny-skia"),
        ))
        .expect("tiny-skia test renderer")
    }

    fn test_common_widget() -> CommonWidget {
        let renderer = test_renderer();
        let handle = image::Handle::from_rgba(100, 100, vec![255; 100 * 100 * 4]);
        let allocation =
            image::Renderer::load_image(&renderer, &handle).expect("test image allocation");
        CommonWidget {
            presented: Some(PresentedImage {
                allocation,
                overlay: None,
                size_px: (100, 100),
                source_alpha: ruviz::core::AlphaMode::Straight,
            }),
            sizing: Sizing::Fixed {
                width: 100.0,
                height: 100.0,
            },
            fit: ImageFit::Fill,
            interaction_enabled: true,
            dimension: PlotDimension::TwoD,
            input_current: true,
        }
    }

    fn test_widget_tree() -> Tree {
        Tree {
            tag: tree::Tag::of::<WidgetState>(),
            state: tree::State::new(WidgetState {
                last_size: Size::new(100.0, 100.0),
                ..WidgetState::default()
            }),
            children: Vec::new(),
        }
    }

    #[test]
    fn fitted_mapping_uses_letterboxed_content_and_fractional_coordinates() {
        let outer = Rectangle {
            x: 10.25,
            y: 20.5,
            width: 300.5,
            height: 300.5,
        };
        let content = fitted_rect(outer, (600, 300), ImageFit::Contain);
        assert_eq!(content.width, 300.5);
        assert_eq!(content.height, 150.25);
        let mapped = logical_to_physical(
            LogicalRect::new(
                content.x.into(),
                content.y.into(),
                content.width.into(),
                content.height.into(),
            ),
            LogicalPoint::new(
                f64::from(content.x + content.width / 2.0),
                f64::from(content.y + content.height / 2.0),
            ),
            (600, 300),
        );
        let (x, y) = mapped.unwrap();
        assert!((x - 300.0).abs() < 1e-4);
        assert!((y - 150.0).abs() < 1e-4);
    }

    #[test]
    fn hidpi_backing_and_all_fit_modes_map_corners_and_centers() {
        assert_eq!(physical_backing_size(100.25, 50.1, 1.0), (101, 51));
        assert_eq!(physical_backing_size(100.25, 50.1, 1.25), (126, 63));
        assert_eq!(physical_backing_size(100.25, 50.1, 1.5), (151, 76));
        assert_eq!(physical_backing_size(100.25, 50.1, 2.0), (201, 101));

        let outer = Rectangle {
            x: 10.0,
            y: 20.0,
            width: 400.0,
            height: 300.0,
        };
        let size = (800, 400);

        assert_eq!(
            map_point(
                outer,
                size,
                ImageFit::Contain,
                iced::Point::new(210.0, 170.0)
            ),
            Some((400.0, 200.0))
        );
        assert_eq!(
            map_point(outer, size, ImageFit::Contain, iced::Point::new(10.0, 20.0)),
            None
        );
        assert_eq!(
            map_point(outer, size, ImageFit::Fill, iced::Point::new(10.0, 20.0)),
            Some((0.0, 0.0))
        );
        assert_eq!(
            map_point(outer, size, ImageFit::Fill, iced::Point::new(410.0, 320.0)),
            Some((800.0, 400.0))
        );
        assert_eq!(
            map_point(outer, size, ImageFit::Cover, iced::Point::new(210.0, 170.0)),
            Some((400.0, 200.0))
        );
        assert!(map_point(outer, size, ImageFit::Cover, iced::Point::new(9.99, 170.0)).is_none());
        let cover_corner =
            map_point(outer, size, ImageFit::Cover, iced::Point::new(10.0, 20.0)).unwrap();
        assert!((cover_corner.0 - 133.333_333).abs() < 1e-3);
        assert_eq!(cover_corner.1, 0.0);
    }

    #[test]
    fn escape_is_owned_only_by_hovered_or_dragging_widget() {
        let mut first = WidgetState::default();
        let second = WidgetState::default();
        assert!(!owns_escape(&first));
        assert!(!owns_escape(&second));

        first.hovered = true;
        assert!(owns_escape(&first));
        assert!(!owns_escape(&second));

        first.hovered = false;
        first.pressed = Some(PointerButton::Left);
        assert!(owns_escape(&first));
        assert!(!owns_escape(&second));
    }

    fn assert_stale_navigation_contract(button: PointerButton, iced_button: mouse::Button) {
        let mut state = WidgetState {
            pressed: Some(button),
            ..WidgetState::default()
        };
        let moved = IcedEvent::Mouse(mouse::Event::CursorMoved {
            position: iced::Point::new(40.0, 30.0),
        });
        let released = IcedEvent::Mouse(mouse::Event::ButtonReleased(iced_button));
        let mismatched_release =
            IcedEvent::Mouse(mouse::Event::ButtonReleased(mouse::Button::Other(9)));
        let wheel = IcedEvent::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 },
        });
        let new_press = IcedEvent::Mouse(mouse::Event::ButtonPressed(iced_button));
        let mapped = Some((40.0, 30.0));

        assert!(routes_while_frame_is_stale(&state, &moved, mapped));
        assert!(routes_while_frame_is_stale(&state, &released, mapped));
        assert!(!routes_while_frame_is_stale(
            &state,
            &mismatched_release,
            mapped,
        ));
        assert!(routes_while_frame_is_stale(&state, &wheel, mapped));
        assert!(!routes_while_frame_is_stale(&state, &new_press, mapped));

        state.pressed = None;
        assert!(!routes_while_frame_is_stale(&state, &moved, mapped));
        assert!(!routes_while_frame_is_stale(&state, &released, mapped));
        assert!(routes_while_frame_is_stale(&state, &wheel, mapped));
    }

    #[test]
    fn stale_frame_routes_leave_and_focus_loss_without_enabling_hover_hits() {
        let state = WidgetState::default();
        let moved = IcedEvent::Mouse(mouse::Event::CursorMoved {
            position: iced::Point::new(40.0, 30.0),
        });
        let left = IcedEvent::Mouse(mouse::Event::CursorLeft);
        let unfocused = IcedEvent::Window(iced::window::Event::Unfocused);
        let outer = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let mapped_content = map_point(
            outer,
            (100, 50),
            ImageFit::Contain,
            iced::Point::new(50.0, 50.0),
        );
        let mapped_padding = map_point(
            outer,
            (100, 50),
            ImageFit::Contain,
            iced::Point::new(50.0, 10.0),
        );
        assert!(mapped_content.is_some());
        assert!(mapped_padding.is_none());

        assert!(!routes_while_frame_is_stale(&state, &moved, mapped_content,));
        assert!(routes_while_frame_is_stale(&state, &moved, mapped_padding,));
        assert!(routes_while_frame_is_stale(&state, &left, mapped_padding,));
        assert!(routes_while_frame_is_stale(
            &state,
            &unfocused,
            mapped_padding,
        ));
    }

    #[test]
    fn stale_hover_move_is_coalesced_instead_of_dropped() {
        let mut common = test_common_widget();
        common.input_current = false;
        let mut tree = test_widget_tree();
        let node = layout::Node::new(Size::new(100.0, 100.0));
        let layout = Layout::new(&node);
        let position = Point::new(40.0, 45.0);
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);

        common.update(
            &mut tree,
            &IcedEvent::Mouse(mouse::Event::CursorMoved { position }),
            layout,
            mouse::Cursor::Available(position),
            &mut shell,
            MessageKind::Widget2D,
        );

        let [Message(MessageKind::Widget2D(event))] = messages.as_slice() else {
            panic!("a stale hover move must publish exactly one coalescing message");
        };
        assert_eq!(
            *event,
            WidgetEvent::HoverCoalesced {
                position_px: (40.0, 45.0)
            }
        );
    }

    #[test]
    fn wheel_delta_is_normalized_to_clamped_logical_pixels() {
        assert_eq!(
            wheel_delta_px(mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 }),
            WHEEL_PIXELS_PER_LINE
        );
        assert_eq!(
            wheel_delta_px(mouse::ScrollDelta::Lines { x: 0.0, y: -1.0 }),
            -WHEEL_PIXELS_PER_LINE
        );
        assert_eq!(
            wheel_delta_px(mouse::ScrollDelta::Pixels { x: 0.0, y: 30.0 }),
            30.0
        );
        // A trackpad flick must stay a single proportional step.
        assert_eq!(
            wheel_delta_px(mouse::ScrollDelta::Pixels { x: 0.0, y: 900.0 }),
            WHEEL_MAX_PIXELS
        );
        assert_eq!(
            wheel_delta_px(mouse::ScrollDelta::Lines { x: 0.0, y: -40.0 }),
            -WHEEL_MAX_PIXELS
        );
        assert_eq!(
            wheel_delta_px(mouse::ScrollDelta::Pixels {
                x: 0.0,
                y: f32::NAN
            }),
            0.0
        );
    }

    #[test]
    fn stale_two_d_frame_keeps_captured_pan_and_wheel_navigation_live() {
        assert_stale_navigation_contract(PointerButton::Left, mouse::Button::Left);
    }

    #[cfg(feature = "3d")]
    #[test]
    fn stale_three_d_frame_keeps_captured_orbit_and_wheel_navigation_live() {
        assert_stale_navigation_contract(PointerButton::Left, mouse::Button::Left);
    }

    #[test]
    fn secondary_threshold_keeps_click_and_drag_paths_distinct() {
        assert!(secondary_distance((10.0, 10.0), (12.9, 10.0)) < SECONDARY_DRAG_THRESHOLD_PX);
        assert_eq!(
            secondary_distance((10.0, 10.0), (13.0, 10.0)),
            SECONDARY_DRAG_THRESHOLD_PX
        );
        assert!(secondary_distance((10.0, 10.0), (14.0, 14.0)) > SECONDARY_DRAG_THRESHOLD_PX);
    }

    #[test]
    fn secondary_click_opens_menu_without_publishing_plot_input() {
        let common = test_common_widget();
        let mut tree = test_widget_tree();
        let node = layout::Node::new(Size::new(100.0, 100.0));
        let layout = Layout::new(&node);
        let cursor = mouse::Cursor::Available(Point::new(25.0, 30.0));
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);

        common.update(
            &mut tree,
            &IcedEvent::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
            layout,
            cursor,
            &mut shell,
            MessageKind::Widget2D,
        );
        assert!(messages.is_empty(), "right-down must remain pending");

        let mut shell = Shell::new(&mut messages);
        common.update(
            &mut tree,
            &IcedEvent::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)),
            layout,
            cursor,
            &mut shell,
            MessageKind::Widget2D,
        );
        let layout_invalid = shell.is_layout_invalid();
        let redraw = shell.redraw_request();
        drop(shell);
        assert!(
            messages.is_empty(),
            "a context click must not reach plot state"
        );
        assert!(layout_invalid);
        assert_eq!(redraw, iced::window::RedrawRequest::NextFrame);
        let state = tree.state.downcast_ref::<WidgetState>();
        assert!(state.menu.open);
        assert_eq!(state.menu.anchor, Point::new(25.0, 30.0));
    }

    #[test]
    fn secondary_drag_forwards_press_then_move_after_threshold() {
        let common = test_common_widget();
        let mut tree = test_widget_tree();
        let node = layout::Node::new(Size::new(100.0, 100.0));
        let layout = Layout::new(&node);
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        common.update(
            &mut tree,
            &IcedEvent::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
            layout,
            mouse::Cursor::Available(Point::new(10.0, 10.0)),
            &mut shell,
            MessageKind::Widget2D,
        );
        assert!(messages.is_empty());

        let mut shell = Shell::new(&mut messages);
        common.update(
            &mut tree,
            &IcedEvent::Mouse(mouse::Event::CursorMoved {
                position: Point::new(20.0, 10.0),
            }),
            layout,
            mouse::Cursor::Available(Point::new(20.0, 10.0)),
            &mut shell,
            MessageKind::Widget2D,
        );
        assert_eq!(messages.len(), 2);
        assert!(matches!(
            messages[0].0,
            MessageKind::Widget2D(WidgetEvent::PointerPressed {
                button: PointerButton::Right,
                ..
            })
        ));
        assert!(matches!(
            messages[1].0,
            MessageKind::Widget2D(WidgetEvent::PointerMoved(Some(_)))
        ));
        let state = tree.state.downcast_ref::<WidgetState>();
        assert!(!state.menu.open);
        assert!(
            state
                .pending_secondary
                .is_some_and(|pending| pending.forwarded)
        );
    }

    #[test]
    fn context_menu_key_opens_menu_when_interaction_is_disabled() {
        let mut common = test_common_widget();
        common.interaction_enabled = false;
        common.input_current = false;
        let mut tree = test_widget_tree();
        tree.state.downcast_mut::<WidgetState>().hovered = true;
        let node = layout::Node::new(Size::new(100.0, 100.0));
        let layout = Layout::new(&node);
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        let menu_key = Key::Named(key::Named::ContextMenu);
        common.update(
            &mut tree,
            &IcedEvent::Keyboard(iced::keyboard::Event::KeyPressed {
                key: menu_key.clone(),
                modified_key: menu_key,
                physical_key: key::Physical::Code(key::Code::ContextMenu),
                location: iced::keyboard::Location::Standard,
                modifiers: iced::keyboard::Modifiers::default(),
                text: None,
                repeat: false,
            }),
            layout,
            mouse::Cursor::Available(Point::new(40.0, 45.0)),
            &mut shell,
            MessageKind::Widget2D,
        );
        assert!(shell.is_layout_invalid());
        drop(shell);
        assert!(messages.is_empty());
        assert!(tree.state.downcast_ref::<WidgetState>().menu.open);
    }

    #[test]
    fn context_menu_key_cancels_an_active_plot_drag_before_opening() {
        let common = test_common_widget();
        let mut tree = test_widget_tree();
        {
            let state = tree.state.downcast_mut::<WidgetState>();
            state.hovered = true;
            state.pressed = Some(PointerButton::Left);
        }
        let node = layout::Node::new(Size::new(100.0, 100.0));
        let layout = Layout::new(&node);
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        let menu_key = Key::Named(key::Named::ContextMenu);
        common.update(
            &mut tree,
            &IcedEvent::Keyboard(iced::keyboard::Event::KeyPressed {
                key: menu_key.clone(),
                modified_key: menu_key,
                physical_key: key::Physical::Code(key::Code::ContextMenu),
                location: iced::keyboard::Location::Standard,
                modifiers: iced::keyboard::Modifiers::default(),
                text: None,
                repeat: false,
            }),
            layout,
            mouse::Cursor::Available(Point::new(40.0, 45.0)),
            &mut shell,
            MessageKind::Widget2D,
        );
        drop(shell);
        assert_eq!(messages.len(), 1);
        assert!(matches!(
            messages[0].0,
            MessageKind::Widget2D(WidgetEvent::CancelDrag)
        ));
        let state = tree.state.downcast_ref::<WidgetState>();
        assert!(state.menu.open);
        assert_eq!(state.pressed, None);
        assert!(state.pending_secondary.is_none());
    }

    #[test]
    fn context_menu_contains_every_common_action_when_interaction_is_disabled() {
        let common = CommonWidget {
            presented: None,
            sizing: Sizing::default(),
            fit: ImageFit::Contain,
            interaction_enabled: false,
            dimension: PlotDimension::TwoD,
            input_current: false,
        };
        let entries = common.menu_entries();
        let actions = entries
            .iter()
            .filter_map(|entry| match entry.kind {
                MenuEntryKind::Action(action) => Some(action),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(actions.contains(&PlotContextMenuAction::ResetView));
        assert!(actions.contains(&PlotContextMenuAction::FitToContent));
        assert!(actions.contains(&PlotContextMenuAction::SaveImage));
        assert!(actions.contains(&PlotContextMenuAction::CopyImage));
        assert!(actions.contains(&PlotContextMenuAction::ToggleInteraction));
        for action in [
            PlotContextMenuAction::SaveImage,
            PlotContextMenuAction::CopyImage,
        ] {
            assert!(
                entries
                    .iter()
                    .any(|entry| { entry.kind == MenuEntryKind::Action(action) && !entry.enabled }),
                "image actions must be disabled before a frame is presented"
            );
        }
        assert!(
            entries
                .iter()
                .any(|entry| entry.label == "Enable interaction")
        );

        let presented_entries = test_common_widget().menu_entries();
        assert!(presented_entries.iter().all(|entry| {
            !matches!(
                entry.kind,
                MenuEntryKind::Action(
                    PlotContextMenuAction::SaveImage | PlotContextMenuAction::CopyImage
                )
            ) || entry.enabled
        }));
    }

    #[test]
    fn menu_geometry_clamps_main_panel_to_every_viewport_edge() {
        let entries = vec![
            MenuEntry::action("Reset", PlotContextMenuAction::ResetView),
            MenuEntry::separator(),
            MenuEntry::action("Copy", PlotContextMenuAction::CopyImage),
        ];
        let viewport = Rectangle {
            x: 20.0,
            y: 30.0,
            width: 640.0,
            height: 480.0,
        };
        for anchor in [
            Point::new(-100.0, -100.0),
            Point::new(10_000.0, -100.0),
            Point::new(-100.0, 10_000.0),
            Point::new(10_000.0, 10_000.0),
        ] {
            let geometry = menu_geometry(anchor, viewport, &entries, &[], false, 0.0, 0.0);
            assert!(geometry.main_panel.x >= viewport.x);
            assert!(geometry.main_panel.y >= viewport.y);
            assert!(
                geometry.main_panel.x + geometry.main_panel.width <= viewport.x + viewport.width
            );
            assert!(
                geometry.main_panel.y + geometry.main_panel.height <= viewport.y + viewport.height
            );
        }
    }

    #[test]
    fn every_menu_action_remains_reachable_in_a_tiny_viewport() {
        let mut state = ContextMenuState::default();
        state.open_at(Point::new(0.0, 0.0));
        let entries = (0..8)
            .map(|index| {
                MenuEntry::action(&format!("Action {index}"), PlotContextMenuAction::ResetView)
            })
            .collect::<Vec<_>>();
        let mut overlay = ContextMenuOverlay {
            state: &mut state,
            main_entries: entries,
            camera_entries: Vec::new(),
            viewport: Rectangle {
                x: 0.0,
                y: 0.0,
                width: 80.0,
                height: 18.0,
            },
            dimension: PlotDimension::TwoD,
            translation: Vector::ZERO,
        };

        for index in 0..overlay.main_entries.len() {
            let target = MenuTarget::Main(index);
            overlay.ensure_target_visible(target);
            let geometry = overlay.geometry();
            let visible = geometry.main_rows[index]
                .intersection(&geometry.main_panel)
                .expect("selected action must have a visible hit area");
            assert_eq!(overlay.target_at(visible.center()), Some(target));
        }
    }

    #[test]
    fn overlay_translation_moves_geometry_and_hit_target_together() {
        let mut state = ContextMenuState::default();
        state.open_at(Point::new(10.0, 20.0));
        let overlay = ContextMenuOverlay {
            state: &mut state,
            main_entries: vec![MenuEntry::action("Reset", PlotContextMenuAction::ResetView)],
            camera_entries: Vec::new(),
            viewport: Rectangle {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 600.0,
            },
            dimension: PlotDimension::TwoD,
            translation: Vector::new(125.0, 75.0),
        };
        let geometry = overlay.geometry();
        assert_eq!(geometry.main_panel.x, 135.0);
        assert_eq!(geometry.main_panel.y, 95.0);
        assert_eq!(
            overlay.target_at(Point::new(145.0, 105.0)),
            Some(MenuTarget::Main(0))
        );
        assert_eq!(overlay.target_at(Point::new(20.0, 30.0)), None);
    }

    #[test]
    fn open_overlay_captures_pointer_input_and_refreshes_visual_changes() {
        let mut state = ContextMenuState::default();
        state.open_at(Point::new(10.0, 10.0));
        let mut overlay = ContextMenuOverlay {
            state: &mut state,
            main_entries: vec![MenuEntry::action("Reset", PlotContextMenuAction::ResetView)],
            camera_entries: Vec::new(),
            viewport: Rectangle {
                x: 0.0,
                y: 0.0,
                width: 400.0,
                height: 300.0,
            },
            dimension: PlotDimension::TwoD,
            translation: Vector::ZERO,
        };
        let renderer = test_renderer();
        let node = layout::Node::new(Size::new(400.0, 300.0));
        let layout = Layout::new(&node);
        let mut clipboard = iced::advanced::clipboard::Null;
        let mut messages = Vec::new();

        let mut shell = Shell::new(&mut messages);
        iced::advanced::Overlay::<Message, iced::Theme, iced::Renderer>::update(
            &mut overlay,
            &IcedEvent::Mouse(mouse::Event::CursorMoved {
                position: Point::new(20.0, 20.0),
            }),
            layout,
            mouse::Cursor::Available(Point::new(20.0, 20.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
        );
        assert!(shell.is_event_captured());
        assert_eq!(
            shell.redraw_request(),
            iced::window::RedrawRequest::NextFrame
        );
        drop(shell);
        assert_eq!(overlay.state.hovered, Some(MenuTarget::Main(0)));

        let mut shell = Shell::new(&mut messages);
        iced::advanced::Overlay::<Message, iced::Theme, iced::Renderer>::update(
            &mut overlay,
            &IcedEvent::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 },
            }),
            layout,
            mouse::Cursor::Available(Point::new(20.0, 20.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
        );
        assert!(shell.is_event_captured(), "wheel must not reach the plot");
        drop(shell);
        assert!(messages.is_empty());

        let mut shell = Shell::new(&mut messages);
        iced::advanced::Overlay::<Message, iced::Theme, iced::Renderer>::update(
            &mut overlay,
            &IcedEvent::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
            layout,
            mouse::Cursor::Available(Point::new(350.0, 250.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
        );
        assert!(shell.is_event_captured());
        assert!(shell.is_layout_invalid());
        assert_eq!(
            shell.redraw_request(),
            iced::window::RedrawRequest::NextFrame
        );
        assert!(overlay.state.open);
        assert_eq!(overlay.state.anchor, Point::new(350.0, 250.0));
    }

    #[cfg(feature = "3d")]
    #[test]
    fn three_d_menu_exposes_all_named_views_and_flips_submenu_at_right_edge() {
        let common = CommonWidget {
            presented: None,
            sizing: Sizing::default(),
            fit: ImageFit::Contain,
            interaction_enabled: true,
            dimension: PlotDimension::ThreeD,
            input_current: false,
        };
        let main = common.menu_entries();
        let camera = camera_menu_entries();
        assert_eq!(camera.len(), 7);
        for view in [
            CameraView3D::Isometric,
            CameraView3D::Front,
            CameraView3D::Back,
            CameraView3D::Left,
            CameraView3D::Right,
            CameraView3D::Top,
            CameraView3D::Bottom,
        ] {
            assert!(camera.iter().any(|entry| {
                entry.kind == MenuEntryKind::Action(PlotContextMenuAction::CameraView(view))
            }));
        }
        let viewport = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        };
        let geometry = menu_geometry(
            Point::new(795.0, 100.0),
            viewport,
            &main,
            &camera,
            true,
            0.0,
            0.0,
        );
        let camera_panel = geometry.camera_panel.expect("camera submenu");
        assert!(camera_panel.x < geometry.main_panel.x);
        assert!(camera_panel.x >= viewport.x);
    }
}
