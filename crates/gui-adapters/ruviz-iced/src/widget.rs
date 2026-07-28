use iced::advanced::image;
use iced::advanced::layout::{self, Layout};
use iced::advanced::mouse;
use iced::advanced::renderer;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::keyboard::{Key, key};
use iced::{Element, Event as IcedEvent, Length, Rectangle, Size};

use ruviz::core::{ImageFit, LogicalPoint, LogicalRect, fitted_content_rect, logical_to_physical};

#[cfg(feature = "3d")]
use crate::state::Plot3DState;
use crate::state::PlotState;
use crate::{
    Message, MessageKind, PointerButton, Presentation, PresentedImage, Sizing, WidgetEvent,
};

#[derive(Default)]
struct WidgetState {
    last_size: Size,
    pressed: Option<PointerButton>,
    previous_click: Option<mouse::Click>,
    hovered: bool,
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
    Renderer: image::Renderer<Handle = image::Handle>,
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
        if self.common.presentation == Presentation::Interactive
            && self.common.input_current
            && cursor.is_over(layout.bounds())
        {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

impl<'a, Theme, Renderer> From<PlotWidget> for Element<'a, Message, Theme, Renderer>
where
    Theme: 'a,
    Renderer: image::Renderer<Handle = image::Handle> + 'a,
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
    Renderer: image::Renderer<Handle = image::Handle>,
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
        if self.common.presentation == Presentation::Interactive
            && self.common.input_current
            && cursor.is_over(layout.bounds())
        {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}

#[cfg(feature = "3d")]
impl<'a, Theme, Renderer> From<Plot3DWidget> for Element<'a, Message, Theme, Renderer>
where
    Theme: 'a,
    Renderer: image::Renderer<Handle = image::Handle> + 'a,
{
    fn from(widget: Plot3DWidget) -> Self {
        Element::new(widget)
    }
}

struct CommonWidget {
    presented: Option<PresentedImage>,
    sizing: Sizing,
    fit: ImageFit,
    presentation: Presentation,
    input_current: bool,
}

impl CommonWidget {
    fn from_2d(state: &PlotState) -> Self {
        Self {
            presented: state.presented.clone(),
            sizing: state.sizing,
            fit: state.fit,
            presentation: state.presentation,
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
            presentation: state.presentation,
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

        if self.presentation == Presentation::Static {
            return;
        }

        let mapped = self.map_cursor(bounds, cursor);
        if matches!(event, IcedEvent::Mouse(mouse::Event::CursorMoved { .. })) {
            state.hovered = mapped.is_some();
        }

        if !self.input_current && !routes_while_frame_is_stale(state, event, mapped) {
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
                let delta_y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y * 40.0,
                    mouse::ScrollDelta::Pixels { y, .. } => *y,
                };
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
                shell.publish(Message(wrap(WidgetEvent::Escape)));
                shell.capture_event();
            }
            _ => {}
        }
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
    }

    fn map_cursor(&self, outer: Rectangle, cursor: mouse::Cursor) -> Option<(f64, f64)> {
        let presented = self.presented.as_ref()?;
        let position = cursor.position()?;
        map_point(outer, presented.size_px, self.fit, position)
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
    fn stale_two_d_frame_keeps_captured_pan_and_wheel_navigation_live() {
        assert_stale_navigation_contract(PointerButton::Left, mouse::Button::Left);
    }

    #[cfg(feature = "3d")]
    #[test]
    fn stale_three_d_frame_keeps_captured_orbit_and_wheel_navigation_live() {
        assert_stale_navigation_contract(PointerButton::Left, mouse::Button::Left);
    }
}
