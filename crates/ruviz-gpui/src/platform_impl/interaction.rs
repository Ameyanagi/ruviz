use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BuiltinContextMenuAction {
    ResetView,
    SetCurrentViewAsHome,
    GoToHomeView,
    SavePng,
    CopyImage,
    CopyCursorCoordinates,
    CopyVisibleBounds,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ContextMenuEntryKind {
    Builtin(BuiltinContextMenuAction),
    Custom { id: String },
    Separator,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ContextMenuEntry {
    kind: ContextMenuEntryKind,
    label: String,
    enabled: bool,
    bounds: Option<Bounds<Pixels>>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ContextMenuState {
    panel_bounds: Bounds<Pixels>,
    entries: Vec<ContextMenuEntry>,
    hovered_index: Option<usize>,
    trigger_position: Point<Pixels>,
    trigger_position_px: ViewportPoint,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) enum ActiveDrag {
    #[default]
    None,
    LeftPan {
        anchor_px: ViewportPoint,
        last_px: ViewportPoint,
        crossed_threshold: bool,
    },
    RightZoom {
        anchor_px: ViewportPoint,
        current_px: ViewportPoint,
        crossed_threshold: bool,
        zoom_enabled: bool,
    },
    Brush {
        anchor_px: ViewportPoint,
        current_px: ViewportPoint,
        crossed_threshold: bool,
    },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct InteractionState {
    pub(super) last_pointer_px: Option<ViewportPoint>,
    pub(super) active_drag: ActiveDrag,
    pub(super) context_menu: Option<ContextMenuState>,
    pub(super) home_view_bounds: Option<ViewportRect>,
}

impl InteractionState {
    pub(super) fn reset(&mut self) {
        self.reset_pointer_state();
        self.context_menu = None;
        self.home_view_bounds = None;
    }

    fn reset_pointer_state(&mut self) {
        self.last_pointer_px = None;
        self.active_drag = ActiveDrag::None;
    }
}

pub(super) fn log_interaction_error(action: &str, err: &PlottingError) {
    eprintln!("ruviz-gpui {action} failed: {err}");
}

pub(super) fn distance_px(a: ViewportPoint, b: ViewportPoint) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

pub(super) fn cursor_data_position(
    visible_bounds: ViewportRect,
    plot_area: ViewportRect,
    cursor_position_px: ViewportPoint,
) -> Option<ViewportPoint> {
    let plot_width = plot_area.width();
    let plot_height = plot_area.height();
    if plot_width <= f64::EPSILON || plot_height <= f64::EPSILON {
        return None;
    }
    if !plot_area.contains(cursor_position_px) {
        return None;
    }

    let normalized_x = (cursor_position_px.x - plot_area.min.x) / plot_width;
    let normalized_y = (cursor_position_px.y - plot_area.min.y) / plot_height;
    Some(ViewportPoint::new(
        visible_bounds.min.x + visible_bounds.width() * normalized_x,
        visible_bounds.max.y - visible_bounds.height() * normalized_y,
    ))
}

impl RuvizPlot {
    pub(super) fn reset_pointer_state(&mut self) {
        self.interaction_state.reset_pointer_state();
    }

    pub(super) fn current_cursor_position_px(&self) -> Option<ViewportPoint> {
        self.interaction_state.last_pointer_px
    }

    fn close_context_menu(&mut self, cx: &mut Context<Self>) {
        if self.interaction_state.context_menu.take().is_some() {
            cx.notify();
        }
    }

    fn update_context_menu_hover(&mut self, hovered_index: Option<usize>, cx: &mut Context<Self>) {
        if let Some(menu) = self.interaction_state.context_menu.as_mut() {
            if menu.hovered_index != hovered_index {
                menu.hovered_index = hovered_index;
                cx.notify();
            }
        }
    }

    fn context_menu_entry_index_at(&self, position: Point<Pixels>) -> Option<usize> {
        let menu = self.interaction_state.context_menu.as_ref()?;
        menu.entries.iter().position(|entry| {
            !matches!(entry.kind, ContextMenuEntryKind::Separator)
                && entry
                    .bounds
                    .is_some_and(|bounds| bounds.contains(&position))
        })
    }

    fn build_context_menu_entries(
        &self,
        cursor_position_px: ViewportPoint,
    ) -> Result<Vec<ContextMenuEntry>> {
        let snapshot = self.session.viewport_snapshot()?;
        let cursor_available = cursor_data_position(
            snapshot.visible_bounds,
            snapshot.plot_area,
            cursor_position_px,
        )
        .is_some();
        let mut entries = Vec::new();

        let push_entry = |entries: &mut Vec<ContextMenuEntry>,
                          kind: ContextMenuEntryKind,
                          label: &str,
                          enabled: bool| {
            entries.push(ContextMenuEntry {
                kind,
                label: label.to_string(),
                enabled,
                bounds: None,
            });
        };
        let push_separator = |entries: &mut Vec<ContextMenuEntry>| {
            if !entries.is_empty()
                && !matches!(
                    entries.last().map(|entry| &entry.kind),
                    Some(ContextMenuEntryKind::Separator)
                )
            {
                entries.push(ContextMenuEntry {
                    kind: ContextMenuEntryKind::Separator,
                    label: String::new(),
                    enabled: false,
                    bounds: None,
                });
            }
        };

        if self.options.context_menu.show_reset_view {
            push_entry(
                &mut entries,
                ContextMenuEntryKind::Builtin(BuiltinContextMenuAction::ResetView),
                "Reset View",
                true,
            );
        }
        if self.options.context_menu.show_set_home_view {
            push_entry(
                &mut entries,
                ContextMenuEntryKind::Builtin(BuiltinContextMenuAction::SetCurrentViewAsHome),
                "Set Current View As Home",
                true,
            );
        }
        if self.options.context_menu.show_go_to_home_view {
            push_entry(
                &mut entries,
                ContextMenuEntryKind::Builtin(BuiltinContextMenuAction::GoToHomeView),
                "Go To Home View",
                self.interaction_state.home_view_bounds.is_some(),
            );
        }

        let export_group_enabled = self.options.context_menu.show_save_png
            || self.options.context_menu.show_copy_image
            || self.options.context_menu.show_copy_cursor_coordinates
            || self.options.context_menu.show_copy_visible_bounds;
        if !entries.is_empty() && export_group_enabled {
            push_separator(&mut entries);
        }

        if self.options.context_menu.show_save_png {
            push_entry(
                &mut entries,
                ContextMenuEntryKind::Builtin(BuiltinContextMenuAction::SavePng),
                "Save PNG...",
                true,
            );
        }
        if self.options.context_menu.show_copy_image {
            push_entry(
                &mut entries,
                ContextMenuEntryKind::Builtin(BuiltinContextMenuAction::CopyImage),
                "Copy Image",
                true,
            );
        }
        if self.options.context_menu.show_copy_cursor_coordinates {
            push_entry(
                &mut entries,
                ContextMenuEntryKind::Builtin(BuiltinContextMenuAction::CopyCursorCoordinates),
                "Copy Cursor Coordinates",
                cursor_available,
            );
        }
        if self.options.context_menu.show_copy_visible_bounds {
            push_entry(
                &mut entries,
                ContextMenuEntryKind::Builtin(BuiltinContextMenuAction::CopyVisibleBounds),
                "Copy Visible Bounds",
                true,
            );
        }

        if !self.options.context_menu.custom_items.is_empty() {
            push_separator(&mut entries);
            for item in &self.options.context_menu.custom_items {
                entries.push(ContextMenuEntry {
                    kind: ContextMenuEntryKind::Custom {
                        id: item.id.clone(),
                    },
                    label: item.label.clone(),
                    enabled: item.enabled,
                    bounds: None,
                });
            }
        }

        while matches!(
            entries.last().map(|entry| &entry.kind),
            Some(ContextMenuEntryKind::Separator)
        ) {
            entries.pop();
        }

        Ok(entries)
    }

    fn build_context_menu(
        &self,
        position: Point<Pixels>,
        cursor_position_px: ViewportPoint,
    ) -> Result<Option<ContextMenuState>> {
        let layout = match self.last_layout.as_ref() {
            Some(layout) => layout,
            None => return Ok(None),
        };
        let mut entries = self.build_context_menu_entries(cursor_position_px)?;
        if entries.is_empty() {
            return Ok(None);
        }

        let max_chars = entries
            .iter()
            .filter(|entry| !matches!(entry.kind, ContextMenuEntryKind::Separator))
            .map(|entry| entry.label.chars().count())
            .max()
            .unwrap_or(0);
        let panel_width = MENU_MIN_WIDTH_PX.max(max_chars as f32 * 7.2 + MENU_PADDING_X_PX * 2.0);
        let content_height = entries.iter().fold(0.0, |height, entry| {
            height
                + if matches!(entry.kind, ContextMenuEntryKind::Separator) {
                    MENU_SEPARATOR_HEIGHT_PX
                } else {
                    MENU_ITEM_HEIGHT_PX
                }
        });
        let panel_height = content_height + MENU_PADDING_Y_PX * 2.0;

        let min_left = layout.component_bounds.origin.x + px(MENU_EDGE_MARGIN_PX);
        let min_top = layout.component_bounds.origin.y + px(MENU_EDGE_MARGIN_PX);
        let max_left = (layout.component_bounds.origin.x + layout.component_bounds.size.width)
            - px(panel_width + MENU_EDGE_MARGIN_PX);
        let max_top = (layout.component_bounds.origin.y + layout.component_bounds.size.height)
            - px(panel_height + MENU_EDGE_MARGIN_PX);
        let left = position.x.max(min_left).min(max_left.max(min_left));
        let top = position.y.max(min_top).min(max_top.max(min_top));
        let panel_bounds = Bounds::new(point(left, top), size(px(panel_width), px(panel_height)));

        let mut cursor_y = top + px(MENU_PADDING_Y_PX);
        for entry in &mut entries {
            let entry_height = if matches!(entry.kind, ContextMenuEntryKind::Separator) {
                MENU_SEPARATOR_HEIGHT_PX
            } else {
                MENU_ITEM_HEIGHT_PX
            };
            entry.bounds = Some(Bounds::new(
                point(left, cursor_y),
                size(px(panel_width), px(entry_height)),
            ));
            cursor_y += px(entry_height);
        }

        let hovered_index = entries.iter().position(|entry| {
            !matches!(entry.kind, ContextMenuEntryKind::Separator)
                && entry
                    .bounds
                    .is_some_and(|bounds| bounds.contains(&position))
        });
        Ok(Some(ContextMenuState {
            panel_bounds,
            entries,
            hovered_index,
            trigger_position: position,
            trigger_position_px: cursor_position_px,
        }))
    }

    fn open_context_menu(
        &mut self,
        position: Point<Pixels>,
        cursor_position_px: ViewportPoint,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        if !self.options.context_menu.enabled {
            return Ok(());
        }

        self.session.apply_input(PlotInputEvent::ClearHover);
        self.session.apply_input(PlotInputEvent::HideTooltip);
        self.interaction_state.context_menu =
            self.build_context_menu(position, cursor_position_px)?;
        cx.notify();
        Ok(())
    }

    fn handle_context_menu_left_click(
        &mut self,
        position: Point<Pixels>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let Some(index) = self.context_menu_entry_index_at(position) else {
            self.close_context_menu(cx);
            return Ok(());
        };

        let Some(menu) = self.interaction_state.context_menu.as_ref() else {
            return Ok(());
        };
        if !menu.entries.get(index).is_some_and(|entry| entry.enabled) {
            self.close_context_menu(cx);
            return Ok(());
        }

        self.activate_context_menu_entry(index, window, cx)
    }

    fn activate_context_menu_entry(
        &mut self,
        index: usize,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let Some(menu) = self.interaction_state.context_menu.as_ref() else {
            return Ok(());
        };
        let Some(entry) = menu.entries.get(index).cloned() else {
            return Ok(());
        };
        let trigger_position_px = menu.trigger_position_px;
        self.close_context_menu(cx);

        match entry.kind {
            ContextMenuEntryKind::Builtin(action) => self.execute_builtin_context_menu_action(
                action,
                Some(trigger_position_px),
                window,
                cx,
            ),
            ContextMenuEntryKind::Custom { id } => {
                let Some(handler) = self.context_menu_action_handler.clone() else {
                    return Ok(());
                };
                let Some(context) = self.build_action_context(id, window, trigger_position_px)?
                else {
                    return Ok(());
                };
                handler(context)
            }
            ContextMenuEntryKind::Separator => Ok(()),
        }
    }

    fn execute_builtin_context_menu_action(
        &mut self,
        action: BuiltinContextMenuAction,
        trigger_position_px: Option<ViewportPoint>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        match action {
            BuiltinContextMenuAction::ResetView => {
                self.reset_view(cx);
                Ok(())
            }
            BuiltinContextMenuAction::SetCurrentViewAsHome => self.set_current_view_as_home(cx),
            BuiltinContextMenuAction::GoToHomeView => self.go_to_home_view(cx),
            BuiltinContextMenuAction::SavePng => self.save_png(window, cx),
            BuiltinContextMenuAction::CopyImage => self.copy_image(window, cx),
            BuiltinContextMenuAction::CopyCursorCoordinates => trigger_position_px.map_or_else(
                || self.copy_cursor_coordinates(),
                |px| self.copy_cursor_coordinates_at(px),
            ),
            BuiltinContextMenuAction::CopyVisibleBounds => self.copy_visible_bounds(),
        }
    }

    pub(super) fn builtin_shortcut_action_for_keystroke(
        &self,
        event: &KeyDownEvent,
    ) -> Option<BuiltinContextMenuAction> {
        if !event.keystroke.modifiers.secondary() || event.keystroke.modifiers.alt {
            return None;
        }

        match event.keystroke.key.as_str() {
            "s" if self.options.context_menu.show_save_png => {
                Some(BuiltinContextMenuAction::SavePng)
            }
            "c" if self.options.context_menu.show_copy_image => {
                Some(BuiltinContextMenuAction::CopyImage)
            }
            _ => None,
        }
    }

    pub(super) fn zoom_overlay_bounds(&self) -> Option<Bounds<Pixels>> {
        let ActiveDrag::RightZoom {
            anchor_px,
            current_px,
            crossed_threshold,
            zoom_enabled,
        } = self.interaction_state.active_drag
        else {
            return None;
        };
        if !crossed_threshold || !zoom_enabled {
            return None;
        }

        let start = self.viewport_point_to_window_position(anchor_px)?;
        let end = self.viewport_point_to_window_position(current_px)?;
        let left = start.x.min(end.x);
        let top = start.y.min(end.y);
        let width = (start.x.max(end.x) - left).max(px(1.0));
        let height = (start.y.max(end.y) - top).max(px(1.0));
        Some(Bounds::new(point(left, top), size(width, height)))
    }

    pub(super) fn handle_left_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        cx.focus_self(window);

        if self.interaction_state.context_menu.is_some() {
            return self.handle_context_menu_left_click(event.position, window, cx);
        }

        let Some(position_px) = self.local_viewport_point(event.position) else {
            return Ok(());
        };

        self.interaction_state.last_pointer_px = Some(position_px);
        if event.click_count >= 2 {
            self.reset_view(cx);
            return Ok(());
        }

        if event.modifiers.shift && self.options.interaction.selection {
            self.interaction_state.active_drag = ActiveDrag::Brush {
                anchor_px: position_px,
                current_px: position_px,
                crossed_threshold: false,
            };
            self.session
                .apply_input(PlotInputEvent::BrushStart { position_px });
            cx.notify();
            return Ok(());
        }

        if self.options.interaction.pan {
            self.interaction_state.active_drag = ActiveDrag::LeftPan {
                anchor_px: position_px,
                last_px: position_px,
                crossed_threshold: false,
            };
        } else {
            self.interaction_state.active_drag = ActiveDrag::None;
        }
        cx.notify();
        Ok(())
    }

    pub(super) fn handle_right_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        cx.focus_self(window);
        if self.interaction_state.context_menu.is_some() {
            self.close_context_menu(cx);
        }

        let Some(position_px) = self.local_viewport_point(event.position) else {
            return Ok(());
        };
        self.interaction_state.last_pointer_px = Some(position_px);
        self.interaction_state.active_drag = ActiveDrag::RightZoom {
            anchor_px: position_px,
            current_px: position_px,
            crossed_threshold: false,
            zoom_enabled: self.options.interaction.zoom,
        };
        cx.notify();
        Ok(())
    }

    pub(super) fn handle_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        if self.interaction_state.context_menu.is_some() {
            let hovered_index = self.context_menu_entry_index_at(event.position);
            self.update_context_menu_hover(hovered_index, cx);
            return Ok(());
        }

        let position_px = self.local_viewport_point(event.position);
        if let Some(position_px) = position_px {
            self.interaction_state.last_pointer_px = Some(position_px);
        }

        match (self.interaction_state.active_drag, event.pressed_button) {
            (
                ActiveDrag::LeftPan {
                    anchor_px,
                    last_px,
                    crossed_threshold,
                },
                Some(MouseButton::Left),
            ) if self.options.interaction.pan => {
                let Some(position_px) = position_px else {
                    return Ok(());
                };
                let crossed_threshold_now =
                    crossed_threshold || distance_px(anchor_px, position_px) > DRAG_THRESHOLD_PX;
                let delta_px =
                    ViewportPoint::new(position_px.x - last_px.x, position_px.y - last_px.y);
                if delta_px.x.abs() > f64::EPSILON || delta_px.y.abs() > f64::EPSILON {
                    self.session.apply_input(PlotInputEvent::Pan { delta_px });
                    cx.notify();
                }
                self.interaction_state.active_drag = ActiveDrag::LeftPan {
                    anchor_px,
                    last_px: position_px,
                    crossed_threshold: crossed_threshold_now,
                };
                return Ok(());
            }
            (
                ActiveDrag::Brush {
                    anchor_px,
                    current_px: _,
                    crossed_threshold,
                },
                Some(MouseButton::Left),
            ) if self.options.interaction.selection => {
                let Some(position_px) = position_px else {
                    return Ok(());
                };
                let crossed_threshold_now =
                    crossed_threshold || distance_px(anchor_px, position_px) > DRAG_THRESHOLD_PX;
                self.session
                    .apply_input(PlotInputEvent::BrushMove { position_px });
                self.interaction_state.active_drag = ActiveDrag::Brush {
                    anchor_px,
                    current_px: position_px,
                    crossed_threshold: crossed_threshold_now,
                };
                cx.notify();
                return Ok(());
            }
            (
                ActiveDrag::RightZoom {
                    anchor_px,
                    current_px: _,
                    crossed_threshold,
                    zoom_enabled,
                },
                Some(MouseButton::Right),
            ) => {
                let position_px = if zoom_enabled {
                    self.clamped_viewport_point(event.position)
                        .unwrap_or(anchor_px)
                } else {
                    position_px.unwrap_or(anchor_px)
                };
                let crossed_threshold_now =
                    crossed_threshold || distance_px(anchor_px, position_px) > DRAG_THRESHOLD_PX;
                if crossed_threshold_now {
                    self.session.apply_input(PlotInputEvent::ClearHover);
                    self.session.apply_input(PlotInputEvent::HideTooltip);
                }
                self.interaction_state.active_drag = ActiveDrag::RightZoom {
                    anchor_px,
                    current_px: position_px,
                    crossed_threshold: crossed_threshold_now,
                    zoom_enabled,
                };
                cx.notify();
                return Ok(());
            }
            _ => {}
        }

        match position_px {
            Some(position_px) if self.options.interaction.hover => {
                self.session
                    .apply_input(PlotInputEvent::Hover { position_px });
                if !self.options.interaction.tooltips {
                    self.session.apply_input(PlotInputEvent::HideTooltip);
                }
                cx.notify();
            }
            None if self.options.interaction.hover => {
                self.session.apply_input(PlotInputEvent::ClearHover);
                self.session.apply_input(PlotInputEvent::HideTooltip);
                cx.notify();
            }
            None | Some(_) => {}
        }

        Ok(())
    }

    pub(super) fn handle_left_mouse_up(
        &mut self,
        event: &MouseUpEvent,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let position_px = self
            .local_viewport_point(event.position)
            .or(self.interaction_state.last_pointer_px);

        match (self.interaction_state.active_drag, position_px) {
            (
                ActiveDrag::Brush {
                    anchor_px: _,
                    current_px: _,
                    crossed_threshold: _,
                },
                Some(position_px),
            ) if self.options.interaction.selection => {
                self.session
                    .apply_input(PlotInputEvent::BrushEnd { position_px });
                cx.notify();
            }
            (
                ActiveDrag::LeftPan {
                    anchor_px,
                    last_px: _,
                    crossed_threshold,
                },
                Some(position_px),
            ) if self.options.interaction.selection && !crossed_threshold => {
                if distance_px(anchor_px, position_px) <= DRAG_THRESHOLD_PX {
                    self.session
                        .apply_input(PlotInputEvent::SelectAt { position_px });
                    cx.notify();
                }
            }
            _ => {}
        }

        self.reset_pointer_state();
        Ok(())
    }

    pub(super) fn handle_right_mouse_up(
        &mut self,
        event: &MouseUpEvent,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let position_px = self
            .local_viewport_point(event.position)
            .or_else(|| self.clamped_viewport_point(event.position))
            .or(self.interaction_state.last_pointer_px);

        match (self.interaction_state.active_drag, position_px) {
            (
                ActiveDrag::RightZoom {
                    anchor_px,
                    current_px: _,
                    crossed_threshold,
                    zoom_enabled,
                },
                Some(position_px),
            ) => {
                let crossed_threshold_now =
                    crossed_threshold || distance_px(anchor_px, position_px) > DRAG_THRESHOLD_PX;
                if crossed_threshold_now {
                    if zoom_enabled {
                        let region_px = ViewportRect::from_points(anchor_px, position_px);
                        if region_px.width() > DRAG_THRESHOLD_PX
                            && region_px.height() > DRAG_THRESHOLD_PX
                        {
                            self.session
                                .apply_input(PlotInputEvent::ZoomRect { region_px });
                            self.invalidate_frame_state();
                            cx.notify();
                        }
                    }
                } else {
                    self.open_context_menu(event.position, position_px, cx)?;
                }
            }
            (ActiveDrag::RightZoom { .. }, None) => {
                self.close_context_menu(cx);
            }
            _ => {}
        }

        self.reset_pointer_state();
        Ok(())
    }

    pub(super) fn handle_hover_change(&mut self, hovered: bool, cx: &mut Context<Self>) {
        if hovered {
            return;
        }

        if self.interaction_state.context_menu.is_some() {
            self.update_context_menu_hover(None, cx);
            return;
        }

        self.session.apply_input(PlotInputEvent::ClearHover);
        self.session.apply_input(PlotInputEvent::HideTooltip);
        if !matches!(
            self.interaction_state.active_drag,
            ActiveDrag::RightZoom { .. } | ActiveDrag::LeftPan { .. } | ActiveDrag::Brush { .. }
        ) {
            self.reset_pointer_state();
        }
        cx.notify();
    }

    pub(super) fn normalize_scroll_delta(delta: ScrollDelta) -> f64 {
        match delta {
            ScrollDelta::Pixels(point) => point.y.into(),
            ScrollDelta::Lines(point) => -f64::from(point.y) * f64::from(LINE_SCROLL_DELTA_PX),
        }
    }

    pub(super) fn handle_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        if !self.options.interaction.zoom || self.interaction_state.context_menu.is_some() {
            return Ok(());
        }

        let Some(center_px) = self.local_viewport_point(event.position) else {
            return Ok(());
        };
        let factor = (1.0025f64)
            .powf(-Self::normalize_scroll_delta(event.delta))
            .clamp(0.25, 4.0);
        self.session
            .apply_input(PlotInputEvent::Zoom { factor, center_px });
        cx.notify();
        Ok(())
    }

    pub(super) fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Result<bool> {
        if let Some(action) = self.builtin_shortcut_action_for_keystroke(event) {
            self.close_context_menu(cx);
            self.execute_builtin_context_menu_action(action, None, window, cx)?;
            return Ok(true);
        }

        match event.keystroke.key.as_str() {
            "escape" => {
                if self.interaction_state.context_menu.is_some() {
                    self.close_context_menu(cx);
                } else {
                    self.reset_view(cx);
                }
                Ok(true)
            }
            "delete" | "backspace" if self.options.interaction.selection => {
                self.session.apply_input(PlotInputEvent::ClearSelection);
                cx.notify();
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub(super) fn render_zoom_overlay(&self, bounds: Bounds<Pixels>) -> AnyElement {
        let Some(layout) = self.last_layout.as_ref() else {
            return div().into_any_element();
        };
        let local_left = bounds.origin.x - layout.component_bounds.origin.x;
        let local_top = bounds.origin.y - layout.component_bounds.origin.y;
        div()
            .absolute()
            .left(local_left)
            .top(local_top)
            .w(bounds.size.width)
            .h(bounds.size.height)
            .bg(rgba(0x5aa9e633))
            .border_1()
            .border_color(rgba(0x7fc8f8ff))
            .rounded_sm()
            .into_any_element()
    }

    pub(super) fn render_context_menu_overlay(&self) -> Option<AnyElement> {
        let layout = self.last_layout.as_ref()?;
        let menu = self.interaction_state.context_menu.as_ref()?;
        let panel_left = menu.panel_bounds.origin.x - layout.component_bounds.origin.x;
        let panel_top = menu.panel_bounds.origin.y - layout.component_bounds.origin.y;
        let mut panel = div()
            .absolute()
            .left(panel_left)
            .top(panel_top)
            .w(menu.panel_bounds.size.width)
            .h(menu.panel_bounds.size.height)
            .bg(rgb(0x1b1e24))
            .border_1()
            .border_color(rgba(0x46515eff))
            .rounded_md()
            .shadow_lg()
            .overflow_hidden()
            .flex()
            .flex_col()
            .py(px(MENU_PADDING_Y_PX));

        let mut children = Vec::new();
        for (index, entry) in menu.entries.iter().enumerate() {
            if matches!(entry.kind, ContextMenuEntryKind::Separator) {
                children.push(
                    div()
                        .h(px(1.0))
                        .mx(px(MENU_PADDING_X_PX))
                        .my(px((MENU_SEPARATOR_HEIGHT_PX - 1.0) * 0.5))
                        .bg(rgba(0x46515eff))
                        .into_any_element(),
                );
                continue;
            }

            let is_hovered = menu.hovered_index == Some(index) && entry.enabled;
            let text_color = if entry.enabled {
                rgb(0xf5f7fb)
            } else {
                rgb(0x707781)
            };

            let entry_bg = if is_hovered {
                rgba(0x3b4d6333)
            } else {
                rgba(0x00000000)
            };
            children.push(
                div()
                    .h(px(MENU_ITEM_HEIGHT_PX))
                    .px(px(MENU_PADDING_X_PX))
                    .bg(entry_bg)
                    .text_color(text_color)
                    .text_sm()
                    .flex()
                    .items_center()
                    .child(entry.label.clone())
                    .into_any_element(),
            );
        }
        panel = panel.children(children);
        Some(panel.into_any_element())
    }
}
