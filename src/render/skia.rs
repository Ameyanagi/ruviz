use crate::{
    core::{
        ComputedMargins, CoordinateTransform, LayoutRect, Legend, LegendItem, LegendItemType,
        LegendPosition, LegendSpacingPixels, LegendStyle, PlottingError, RenderScale, Result,
        SpacingConfig, TextPosition, TickFormatter, find_best_position,
        plot::{Image, RenderDiagnostics, TextEngineMode, TickDirection, TickSides},
        pt_to_px,
    },
    render::{
        Color, FontConfig, FontFamily, LineStyle, MarkerStyle, TextRenderer, Theme,
        typst_text::{self, TypstBackendKind, TypstTextAnchor},
    },
};
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use tiny_skia::*;

mod annotations;
mod primitives;
mod utils;
pub use self::utils::{
    ColorbarTicks, calculate_plot_area, calculate_plot_area_config, calculate_plot_area_dpi,
    compute_colorbar_ticks, format_log_tick_label, format_tick_label, format_tick_labels,
    format_tick_labels_for_scale, generate_minor_ticks, generate_ticks, map_data_to_pixels,
    map_data_to_pixels_scaled,
};
pub(crate) use self::utils::{
    colorbar_major_label_anchor_center_from_top, colorbar_major_label_top,
    compute_colorbar_layout_metrics,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct ClipMaskKey {
    x_bits: u32,
    y_bits: u32,
    width_bits: u32,
    height_bits: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct MarkerPathKey {
    style: MarkerStyle,
    size_bits: u32,
}

impl MarkerPathKey {
    fn new(style: MarkerStyle, size: f32) -> Self {
        Self {
            style,
            size_bits: size.to_bits(),
        }
    }
}

impl ClipMaskKey {
    fn new((x, y, width, height): (f32, f32, f32, f32)) -> Self {
        Self {
            x_bits: x.to_bits(),
            y_bits: y.to_bits(),
            width_bits: width.to_bits(),
            height_bits: height.to_bits(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct MarkerSpriteKey {
    style: MarkerStyle,
    size_bits: u32,
    rgba_bits: u32,
    phase_x: u8,
    phase_y: u8,
}

const GLOBAL_MARKER_SPRITE_CACHE_LIMIT: usize = 4096;
static GLOBAL_MARKER_SPRITE_CACHE: OnceLock<Mutex<HashMap<MarkerSpriteKey, Arc<MarkerSprite>>>> =
    OnceLock::new();

fn global_marker_sprite_cache() -> &'static Mutex<HashMap<MarkerSpriteKey, Arc<MarkerSprite>>> {
    GLOBAL_MARKER_SPRITE_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

impl MarkerSpriteKey {
    fn new(style: MarkerStyle, size: f32, color: Color, phase_x: u8, phase_y: u8) -> Self {
        Self {
            style,
            size_bits: size.to_bits(),
            rgba_bits: u32::from_be_bytes([color.r, color.g, color.b, color.a]),
            phase_x,
            phase_y,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct MarkerSpriteScanline {
    pub start_x: u16,
    pub end_x: u16,
    pub opaque_start_x: u16,
    pub opaque_end_x: u16,
}

#[derive(Clone, Debug)]
pub(crate) struct MarkerSprite {
    pub width: u32,
    pub height: u32,
    pub origin_x: i32,
    pub origin_y: i32,
    pub pixels: Vec<u8>,
    pub scanlines: Option<Arc<[MarkerSpriteScanline]>>,
}

/// Tiny-skia based renderer with cosmic-text for professional typography
pub struct SkiaRenderer {
    width: u32,
    height: u32,
    pixmap: Pixmap,
    paint: Paint<'static>,
    theme: Theme,
    text_renderer: TextRenderer,
    font_config: FontConfig,
    /// Shared render scale for unit conversion.
    render_scale: RenderScale,
    /// Active text rendering engine.
    text_engine_mode: TextEngineMode,
    clip_mask_cache: HashMap<ClipMaskKey, Arc<Mask>>,
    marker_path_cache: HashMap<MarkerPathKey, Arc<tiny_skia::Path>>,
    marker_sprite_cache: HashMap<MarkerSpriteKey, Arc<MarkerSprite>>,
    render_diagnostics: RenderDiagnostics,
}

impl SkiaRenderer {
    /// Create a new renderer with the given dimensions
    pub fn new(width: u32, height: u32, theme: Theme) -> Result<Self> {
        Self::with_font_family(width, height, theme, FontFamily::SansSerif)
    }

    /// Create a new renderer with specified font family
    pub fn with_font_family(
        width: u32,
        height: u32,
        theme: Theme,
        font_family: FontFamily,
    ) -> Result<Self> {
        let mut pixmap = Pixmap::new(width, height).ok_or(PlottingError::OutOfMemory)?;

        // Fill background
        let bg_color = theme.background.to_tiny_skia_color();
        pixmap.fill(bg_color);

        let paint = Paint::default();

        // Create text renderer with default font configuration
        let text_renderer = TextRenderer::new();
        let font_config = FontConfig::new(font_family, 12.0);

        Ok(Self {
            width,
            height,
            pixmap,
            paint,
            theme,
            text_renderer,
            font_config,
            render_scale: RenderScale::from_canvas_size(width, height, crate::core::REFERENCE_DPI),
            text_engine_mode: TextEngineMode::Plain,
            clip_mask_cache: HashMap::new(),
            marker_path_cache: HashMap::new(),
            marker_sprite_cache: HashMap::new(),
            render_diagnostics: RenderDiagnostics::default(),
        })
    }

    /// Set the render scale context used for unit conversion.
    pub fn set_render_scale(&mut self, render_scale: RenderScale) {
        self.render_scale = render_scale;
    }

    /// Get the render scale context used for unit conversion.
    pub fn render_scale(&self) -> RenderScale {
        self.render_scale
    }

    /// Legacy compatibility shim for callers that still pass `dpi / 100.0`.
    pub fn set_dpi_scale(&mut self, dpi_scale: f32) {
        self.set_render_scale(RenderScale::from_reference_scale(dpi_scale));
    }

    /// Legacy compatibility shim for callers that still expect `dpi / 100.0`.
    pub fn dpi_scale(&self) -> f32 {
        self.render_scale.reference_scale()
    }

    fn points_to_pixels(&self, points: f32) -> f32 {
        self.render_scale.points_to_pixels(points)
    }

    fn logical_pixels_to_pixels(&self, logical_pixels: f32) -> f32 {
        self.render_scale.logical_pixels_to_pixels(logical_pixels)
    }

    /// Convert line style to a DPI-scaled dash pattern.
    ///
    /// Dash definitions are authored in logical pixels at the reference DPI and
    /// converted through the shared render scale so physical dash spacing
    /// remains consistent across output resolutions.
    fn scaled_dash_pattern(&self, style: &LineStyle) -> Option<Vec<f32>> {
        style.to_dash_array().map(|pattern| {
            pattern
                .into_iter()
                .map(|segment| self.logical_pixels_to_pixels(segment))
                .collect()
        })
    }

    /// Set text rendering backend mode.
    pub fn set_text_engine_mode(&mut self, mode: TextEngineMode) {
        self.text_engine_mode = mode;
    }

    /// Get text rendering backend mode.
    pub fn text_engine_mode(&self) -> TextEngineMode {
        self.text_engine_mode
    }

    pub(crate) fn set_render_mode_diagnostics(&mut self, mode: &'static str) {
        self.render_diagnostics.render_mode = mode;
    }

    pub(crate) fn note_parallel_render(&mut self) {
        self.render_diagnostics.used_parallel = true;
    }

    pub(crate) fn note_auto_datashader(&mut self) {
        self.render_diagnostics.used_auto_datashader = true;
    }

    pub(crate) fn note_exact_line_canonicalization(&mut self) {
        self.render_diagnostics.used_exact_line_canonicalization = true;
    }

    pub(crate) fn note_raster_line_reduction(&mut self) {
        self.render_diagnostics.used_raster_line_reduction = true;
    }

    pub(crate) fn note_marker_path_cache(&mut self) {
        self.render_diagnostics.used_marker_path_cache = true;
    }

    pub(crate) fn note_marker_sprite_cache(&mut self) {
        self.render_diagnostics.used_marker_sprite_cache = true;
    }

    pub(crate) fn note_marker_sprite_compositor(&mut self) {
        self.render_diagnostics.used_marker_sprite_compositor = true;
    }

    pub(crate) fn note_marker_sprite_fallback(&mut self) {
        self.render_diagnostics.used_marker_sprite_fallback = true;
    }

    pub(crate) fn note_marker_scanline_blit(&mut self) {
        self.render_diagnostics.used_marker_scanline_blit = true;
    }

    pub(crate) fn note_direct_rect_fill(&mut self) {
        self.render_diagnostics.used_direct_rect_fill = true;
    }

    pub(crate) fn note_pixel_aligned_rect_fill(&mut self) {
        self.render_diagnostics.used_pixel_aligned_rect_fill = true;
    }

    pub(crate) fn note_prepared_geometry_cache(&mut self) {
        self.render_diagnostics.used_prepared_geometry_cache = true;
    }

    pub(crate) fn note_rebuilt_prepared_geometry_cache(&mut self) {
        self.render_diagnostics.rebuilt_prepared_geometry_cache = true;
    }

    pub(crate) fn render_diagnostics(&self) -> &RenderDiagnostics {
        &self.render_diagnostics
    }

    pub(crate) fn marker_path(
        &mut self,
        style: MarkerStyle,
        size: f32,
    ) -> Result<Option<Arc<tiny_skia::Path>>> {
        let key = MarkerPathKey::new(style, size);
        if let Some(path) = self.marker_path_cache.get(&key) {
            return Ok(Some(Arc::clone(path)));
        }

        let path = match style {
            MarkerStyle::Circle | MarkerStyle::CircleOpen => {
                let mut builder = PathBuilder::new();
                builder.push_circle(0.0, 0.0, size * 0.5);
                builder.finish()
            }
            MarkerStyle::Triangle | MarkerStyle::TriangleOpen | MarkerStyle::TriangleDown => {
                let radius = size * 0.5;
                let mut builder = PathBuilder::new();
                if style == MarkerStyle::TriangleDown {
                    builder.move_to(0.0, radius);
                    builder.line_to(-radius * 0.866, -radius * 0.5);
                    builder.line_to(radius * 0.866, -radius * 0.5);
                } else {
                    builder.move_to(0.0, -radius);
                    builder.line_to(-radius * 0.866, radius * 0.5);
                    builder.line_to(radius * 0.866, radius * 0.5);
                }
                builder.close();
                builder.finish()
            }
            MarkerStyle::Diamond | MarkerStyle::DiamondOpen => {
                let radius = size * 0.5;
                let mut builder = PathBuilder::new();
                builder.move_to(0.0, -radius);
                builder.line_to(radius, 0.0);
                builder.line_to(0.0, radius);
                builder.line_to(-radius, 0.0);
                builder.close();
                builder.finish()
            }
            _ => None,
        };

        let Some(path) = path else {
            return Ok(None);
        };

        let path = Arc::new(path);
        self.marker_path_cache.insert(key, Arc::clone(&path));
        Ok(Some(path))
    }

    pub(crate) fn marker_sprite(
        &mut self,
        style: MarkerStyle,
        size: f32,
        color: Color,
        phase_x: u8,
        phase_y: u8,
    ) -> Result<Arc<MarkerSprite>> {
        let key = MarkerSpriteKey::new(style, size, color, phase_x, phase_y);
        if let Some(sprite) = self.marker_sprite_cache.get(&key) {
            let sprite = Arc::clone(sprite);
            self.note_marker_sprite_cache();
            return Ok(sprite);
        }

        if let Ok(global_cache) = global_marker_sprite_cache().lock() {
            if let Some(sprite) = global_cache.get(&key).cloned() {
                self.marker_sprite_cache.insert(key, Arc::clone(&sprite));
                self.note_marker_sprite_cache();
                return Ok(sprite);
            }
        }

        let sprite = Arc::new(self.create_marker_sprite(style, size, color, phase_x, phase_y)?);
        self.marker_sprite_cache.insert(key, Arc::clone(&sprite));
        if let Ok(mut global_cache) = global_marker_sprite_cache().lock() {
            if global_cache.len() >= GLOBAL_MARKER_SPRITE_CACHE_LIMIT
                && !global_cache.contains_key(&key)
            {
                global_cache.clear();
            }
            global_cache.insert(key, Arc::clone(&sprite));
        }
        self.note_marker_sprite_cache();
        Ok(sprite)
    }

    fn create_marker_sprite(
        &self,
        style: MarkerStyle,
        size: f32,
        color: Color,
        phase_x: u8,
        phase_y: u8,
    ) -> Result<MarkerSprite> {
        let (origin, side) = Self::marker_sprite_geometry(style, size);
        let mut sprite_renderer = SkiaRenderer::new(side, side, self.theme.clone())?;
        sprite_renderer.set_render_scale(self.render_scale);
        sprite_renderer.set_text_engine_mode(self.text_engine_mode);
        sprite_renderer.pixmap.fill(tiny_skia::Color::TRANSPARENT);

        let phase_step = 1.0 / Self::marker_subpixel_phases() as f32;
        let center_x = origin as f32 + phase_x as f32 * phase_step;
        let center_y = origin as f32 + phase_y as f32 * phase_step;

        sprite_renderer
            .draw_marker_with_mask_vector(center_x, center_y, size, style, color, None)?;

        Ok(MarkerSprite {
            width: side,
            height: side,
            origin_x: origin,
            origin_y: origin,
            pixels: sprite_renderer.pixmap.data().to_vec(),
            scanlines: Self::marker_scanlines(style, sprite_renderer.pixmap.data(), side, side),
        })
    }

    fn marker_scanlines(
        style: MarkerStyle,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> Option<Arc<[MarkerSpriteScanline]>> {
        if !matches!(
            style,
            MarkerStyle::Circle
                | MarkerStyle::Square
                | MarkerStyle::Triangle
                | MarkerStyle::TriangleDown
        ) {
            return None;
        }

        let width = width as usize;
        let height = height as usize;
        let mut scanlines = Vec::with_capacity(height);
        for row in 0..height {
            let row_start = row * width * 4;
            let mut start = None;
            let mut end = None;
            let mut opaque_start = None;
            let mut opaque_end = None;

            for col in 0..width {
                let alpha = pixels[row_start + col * 4 + 3];
                if alpha != 0 {
                    start.get_or_insert(col);
                    end = Some(col + 1);
                }
                if alpha == u8::MAX {
                    opaque_start.get_or_insert(col);
                    opaque_end = Some(col + 1);
                }
            }

            if let (Some(start), Some(end)) = (start, end) {
                scanlines.push(MarkerSpriteScanline {
                    start_x: start as u16,
                    end_x: end as u16,
                    opaque_start_x: opaque_start.unwrap_or(start) as u16,
                    opaque_end_x: opaque_end.unwrap_or(start) as u16,
                });
            } else {
                scanlines.push(MarkerSpriteScanline {
                    start_x: 0,
                    end_x: 0,
                    opaque_start_x: 0,
                    opaque_end_x: 0,
                });
            }
        }

        Some(scanlines.into())
    }

    pub(crate) const fn marker_subpixel_phases() -> u8 {
        32
    }

    pub(crate) fn marker_sprite_geometry(style: MarkerStyle, size: f32) -> (i32, u32) {
        let radius = size * 0.5;
        let stroke_half = match style {
            MarkerStyle::SquareOpen => (size * 0.15).max(1.0) * 0.5,
            MarkerStyle::TriangleOpen | MarkerStyle::DiamondOpen => (size * 0.15).max(1.0) * 0.5,
            MarkerStyle::Plus | MarkerStyle::Cross => (size * 0.25).max(1.0) * 0.5,
            MarkerStyle::Star => (size * 0.22).max(1.0) * 0.5,
            _ => 0.5,
        };
        let padding = (radius + stroke_half + 3.0).ceil() as i32;
        let origin = padding + 1;
        let side = (origin * 2 + 2).max(4) as u32;
        (origin, side)
    }

    fn vertical_tick_span(
        spine_y: f32,
        tick_size: f32,
        tick_direction: &TickDirection,
        top: bool,
    ) -> (f32, f32) {
        match tick_direction {
            TickDirection::Inside => {
                if top {
                    (spine_y, spine_y + tick_size)
                } else {
                    (spine_y, spine_y - tick_size)
                }
            }
            TickDirection::Outside => {
                if top {
                    (spine_y, spine_y - tick_size)
                } else {
                    (spine_y, spine_y + tick_size)
                }
            }
            TickDirection::InOut => (spine_y - tick_size / 2.0, spine_y + tick_size / 2.0),
        }
    }

    fn horizontal_tick_span(
        spine_x: f32,
        tick_size: f32,
        tick_direction: &TickDirection,
        right: bool,
    ) -> (f32, f32) {
        match tick_direction {
            TickDirection::Inside => {
                if right {
                    (spine_x, spine_x - tick_size)
                } else {
                    (spine_x, spine_x + tick_size)
                }
            }
            TickDirection::Outside => {
                if right {
                    (spine_x, spine_x + tick_size)
                } else {
                    (spine_x, spine_x - tick_size)
                }
            }
            TickDirection::InOut => (spine_x - tick_size / 2.0, spine_x + tick_size / 2.0),
        }
    }

    fn x_label_center(plot_area: &LayoutRect, x_value: f64, x_min: f64, x_max: f64) -> f32 {
        let x_range = x_max - x_min;
        if x_range.abs() < f64::EPSILON {
            plot_area.center_x()
        } else {
            plot_area.left + ((x_value - x_min) as f32 / x_range as f32) * plot_area.width()
        }
    }

    fn x_label_center_scaled(
        plot_area: &LayoutRect,
        x_value: f64,
        x_min: f64,
        x_max: f64,
        scale: &crate::axes::AxisScale,
    ) -> f32 {
        if (x_max - x_min).abs() < f64::EPSILON {
            plot_area.center_x()
        } else {
            let normalized = scale.normalized_position(x_value, x_min, x_max);
            plot_area.left + normalized as f32 * plot_area.width()
        }
    }

    fn y_label_center(plot_area: &LayoutRect, y_value: f64, y_min: f64, y_max: f64) -> f32 {
        let y_range = y_max - y_min;
        if y_range.abs() < f64::EPSILON {
            plot_area.center_y()
        } else {
            plot_area.bottom - ((y_value - y_min) as f32 / y_range as f32) * plot_area.height()
        }
    }

    fn y_label_center_scaled(
        plot_area: &LayoutRect,
        y_value: f64,
        y_min: f64,
        y_max: f64,
        scale: &crate::axes::AxisScale,
    ) -> f32 {
        if (y_max - y_min).abs() < f64::EPSILON {
            plot_area.center_y()
        } else {
            let normalized = scale.normalized_position(y_value, y_min, y_max);
            plot_area.bottom - normalized as f32 * plot_area.height()
        }
    }

    /// Draw axis lines and ticks
    pub fn draw_axes(
        &mut self,
        plot_area: Rect,
        x_ticks: &[f32],
        y_ticks: &[f32],
        tick_direction: &TickDirection,
        tick_sides: &TickSides,
        color: Color,
    ) -> Result<()> {
        // Axis metrics are authored in logical pixels and resolved via RenderScale.
        let axis_width = self.logical_pixels_to_pixels(1.5);
        let tick_size = self.logical_pixels_to_pixels(5.0);
        let tick_width = self.logical_pixels_to_pixels(1.0);

        // Draw the full plot frame. Tick side selection only controls tick marks.
        self.draw_line(
            plot_area.left(),
            plot_area.bottom(),
            plot_area.right(),
            plot_area.bottom(),
            color,
            axis_width,
            LineStyle::Solid,
        )?;

        self.draw_line(
            plot_area.left(),
            plot_area.top(),
            plot_area.left(),
            plot_area.bottom(),
            color,
            axis_width,
            LineStyle::Solid,
        )?;

        self.draw_line(
            plot_area.left(),
            plot_area.top(),
            plot_area.right(),
            plot_area.top(),
            color,
            axis_width,
            LineStyle::Solid,
        )?;

        self.draw_line(
            plot_area.right(),
            plot_area.top(),
            plot_area.right(),
            plot_area.bottom(),
            color,
            axis_width,
            LineStyle::Solid,
        )?;

        // Draw tick marks
        for &x in x_ticks {
            if x >= plot_area.left() && x <= plot_area.right() {
                if tick_sides.bottom {
                    let (tick_start, tick_end) = Self::vertical_tick_span(
                        plot_area.bottom(),
                        tick_size,
                        tick_direction,
                        false,
                    );
                    self.draw_line(
                        x,
                        tick_start,
                        x,
                        tick_end,
                        color,
                        tick_width,
                        LineStyle::Solid,
                    )?;
                }
                if tick_sides.top {
                    let (tick_start, tick_end) =
                        Self::vertical_tick_span(plot_area.top(), tick_size, tick_direction, true);
                    self.draw_line(
                        x,
                        tick_start,
                        x,
                        tick_end,
                        color,
                        tick_width,
                        LineStyle::Solid,
                    )?;
                }
            }
        }

        for &y in y_ticks {
            if y >= plot_area.top() && y <= plot_area.bottom() {
                if tick_sides.left {
                    let (tick_start, tick_end) = Self::horizontal_tick_span(
                        plot_area.left(),
                        tick_size,
                        tick_direction,
                        false,
                    );
                    self.draw_line(
                        tick_start,
                        y,
                        tick_end,
                        y,
                        color,
                        tick_width,
                        LineStyle::Solid,
                    )?;
                }
                if tick_sides.right {
                    let (tick_start, tick_end) = Self::horizontal_tick_span(
                        plot_area.right(),
                        tick_size,
                        tick_direction,
                        true,
                    );
                    self.draw_line(
                        tick_start,
                        y,
                        tick_end,
                        y,
                        color,
                        tick_width,
                        LineStyle::Solid,
                    )?;
                }
            }
        }

        Ok(())
    }

    /// Draw axis lines with major and minor tick marks.
    pub fn draw_axes_with_minor_ticks(
        &mut self,
        plot_area: Rect,
        x_major_ticks: &[f32],
        y_major_ticks: &[f32],
        x_minor_ticks: &[f32],
        y_minor_ticks: &[f32],
        tick_direction: &TickDirection,
        tick_sides: &TickSides,
        color: Color,
    ) -> Result<()> {
        let axis_width = self.logical_pixels_to_pixels(1.5);
        let major_tick_size = self.logical_pixels_to_pixels(5.0);
        let minor_tick_size = self.logical_pixels_to_pixels(3.0);
        let major_tick_width = self.logical_pixels_to_pixels(1.0);
        let minor_tick_width = self.logical_pixels_to_pixels(0.8);

        self.draw_line(
            plot_area.left(),
            plot_area.bottom(),
            plot_area.right(),
            plot_area.bottom(),
            color,
            axis_width,
            LineStyle::Solid,
        )?;

        self.draw_line(
            plot_area.left(),
            plot_area.top(),
            plot_area.left(),
            plot_area.bottom(),
            color,
            axis_width,
            LineStyle::Solid,
        )?;

        self.draw_line(
            plot_area.left(),
            plot_area.top(),
            plot_area.right(),
            plot_area.top(),
            color,
            axis_width,
            LineStyle::Solid,
        )?;

        self.draw_line(
            plot_area.right(),
            plot_area.top(),
            plot_area.right(),
            plot_area.bottom(),
            color,
            axis_width,
            LineStyle::Solid,
        )?;

        for (tick_size, tick_width, ticks) in [
            (major_tick_size, major_tick_width, x_major_ticks),
            (minor_tick_size, minor_tick_width, x_minor_ticks),
        ] {
            for &x in ticks {
                if x >= plot_area.left() && x <= plot_area.right() {
                    if tick_sides.bottom {
                        let (tick_start, tick_end) = Self::vertical_tick_span(
                            plot_area.bottom(),
                            tick_size,
                            tick_direction,
                            false,
                        );
                        self.draw_line(
                            x,
                            tick_start,
                            x,
                            tick_end,
                            color,
                            tick_width,
                            LineStyle::Solid,
                        )?;
                    }
                    if tick_sides.top {
                        let (tick_start, tick_end) = Self::vertical_tick_span(
                            plot_area.top(),
                            tick_size,
                            tick_direction,
                            true,
                        );
                        self.draw_line(
                            x,
                            tick_start,
                            x,
                            tick_end,
                            color,
                            tick_width,
                            LineStyle::Solid,
                        )?;
                    }
                }
            }
        }

        for (tick_size, tick_width, ticks) in [
            (major_tick_size, major_tick_width, y_major_ticks),
            (minor_tick_size, minor_tick_width, y_minor_ticks),
        ] {
            for &y in ticks {
                if y >= plot_area.top() && y <= plot_area.bottom() {
                    if tick_sides.left {
                        let (tick_start, tick_end) = Self::horizontal_tick_span(
                            plot_area.left(),
                            tick_size,
                            tick_direction,
                            false,
                        );
                        self.draw_line(
                            tick_start,
                            y,
                            tick_end,
                            y,
                            color,
                            tick_width,
                            LineStyle::Solid,
                        )?;
                    }
                    if tick_sides.right {
                        let (tick_start, tick_end) = Self::horizontal_tick_span(
                            plot_area.right(),
                            tick_size,
                            tick_direction,
                            true,
                        );
                        self.draw_line(
                            tick_start,
                            y,
                            tick_end,
                            y,
                            color,
                            tick_width,
                            LineStyle::Solid,
                        )?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Draw axis lines and ticks with advanced configuration
    pub fn draw_axes_with_config(
        &mut self,
        plot_area: Rect,
        x_major_ticks: &[f32],
        y_major_ticks: &[f32],
        x_minor_ticks: &[f32],
        y_minor_ticks: &[f32],
        tick_direction: &TickDirection,
        tick_sides: &TickSides,
        color: Color,
        dpi_scale: f32,
    ) -> Result<()> {
        let render_scale = RenderScale::from_reference_scale(dpi_scale);
        let axis_width = render_scale.logical_pixels_to_pixels(1.5);
        let major_tick_size = render_scale.logical_pixels_to_pixels(8.0);
        let minor_tick_size = render_scale.logical_pixels_to_pixels(4.0);
        let major_tick_width = render_scale.logical_pixels_to_pixels(1.5);
        let minor_tick_width = render_scale.logical_pixels_to_pixels(1.0);

        // Draw the full plot frame. Tick side selection only controls tick marks.
        self.draw_line(
            plot_area.left(),
            plot_area.bottom(),
            plot_area.right(),
            plot_area.bottom(),
            color,
            axis_width,
            LineStyle::Solid,
        )?;

        self.draw_line(
            plot_area.left(),
            plot_area.top(),
            plot_area.left(),
            plot_area.bottom(),
            color,
            axis_width,
            LineStyle::Solid,
        )?;

        self.draw_line(
            plot_area.left(),
            plot_area.top(),
            plot_area.right(),
            plot_area.top(),
            color,
            axis_width,
            LineStyle::Solid,
        )?;

        self.draw_line(
            plot_area.right(),
            plot_area.top(),
            plot_area.right(),
            plot_area.bottom(),
            color,
            axis_width,
            LineStyle::Solid,
        )?;

        for &x in x_major_ticks {
            if x >= plot_area.left() && x <= plot_area.right() {
                if tick_sides.bottom {
                    let (tick_start, tick_end) = Self::vertical_tick_span(
                        plot_area.bottom(),
                        major_tick_size,
                        tick_direction,
                        false,
                    );
                    self.draw_line(
                        x,
                        tick_start,
                        x,
                        tick_end,
                        color,
                        major_tick_width,
                        LineStyle::Solid,
                    )?;
                }
                if tick_sides.top {
                    let (tick_start, tick_end) = Self::vertical_tick_span(
                        plot_area.top(),
                        major_tick_size,
                        tick_direction,
                        true,
                    );
                    self.draw_line(
                        x,
                        tick_start,
                        x,
                        tick_end,
                        color,
                        major_tick_width,
                        LineStyle::Solid,
                    )?;
                }
            }
        }

        for &x in x_minor_ticks {
            if x >= plot_area.left() && x <= plot_area.right() {
                if tick_sides.bottom {
                    let (tick_start, tick_end) = Self::vertical_tick_span(
                        plot_area.bottom(),
                        minor_tick_size,
                        tick_direction,
                        false,
                    );
                    self.draw_line(
                        x,
                        tick_start,
                        x,
                        tick_end,
                        color,
                        minor_tick_width,
                        LineStyle::Solid,
                    )?;
                }
                if tick_sides.top {
                    let (tick_start, tick_end) = Self::vertical_tick_span(
                        plot_area.top(),
                        minor_tick_size,
                        tick_direction,
                        true,
                    );
                    self.draw_line(
                        x,
                        tick_start,
                        x,
                        tick_end,
                        color,
                        minor_tick_width,
                        LineStyle::Solid,
                    )?;
                }
            }
        }

        for &y in y_major_ticks {
            if y >= plot_area.top() && y <= plot_area.bottom() {
                if tick_sides.left {
                    let (tick_start, tick_end) = Self::horizontal_tick_span(
                        plot_area.left(),
                        major_tick_size,
                        tick_direction,
                        false,
                    );
                    self.draw_line(
                        tick_start,
                        y,
                        tick_end,
                        y,
                        color,
                        major_tick_width,
                        LineStyle::Solid,
                    )?;
                }
                if tick_sides.right {
                    let (tick_start, tick_end) = Self::horizontal_tick_span(
                        plot_area.right(),
                        major_tick_size,
                        tick_direction,
                        true,
                    );
                    self.draw_line(
                        tick_start,
                        y,
                        tick_end,
                        y,
                        color,
                        major_tick_width,
                        LineStyle::Solid,
                    )?;
                }
            }
        }

        for &y in y_minor_ticks {
            if y >= plot_area.top() && y <= plot_area.bottom() {
                if tick_sides.left {
                    let (tick_start, tick_end) = Self::horizontal_tick_span(
                        plot_area.left(),
                        minor_tick_size,
                        tick_direction,
                        false,
                    );
                    self.draw_line(
                        tick_start,
                        y,
                        tick_end,
                        y,
                        color,
                        minor_tick_width,
                        LineStyle::Solid,
                    )?;
                }
                if tick_sides.right {
                    let (tick_start, tick_end) = Self::horizontal_tick_span(
                        plot_area.right(),
                        minor_tick_size,
                        tick_direction,
                        true,
                    );
                    self.draw_line(
                        tick_start,
                        y,
                        tick_end,
                        y,
                        color,
                        minor_tick_width,
                        LineStyle::Solid,
                    )?;
                }
            }
        }

        Ok(())
    }

    /// Draw a DataShader aggregated image
    pub fn draw_datashader_image(
        &mut self,
        image: &crate::data::DataShaderImage,
        plot_area: Rect,
    ) -> Result<()> {
        // Create a pixmap from the DataShader image data
        let mut datashader_pixmap = Pixmap::new(image.width as u32, image.height as u32)
            .ok_or(PlottingError::OutOfMemory)?;

        // Copy the RGBA data from DataShader
        if image.pixels.len() != (image.width * image.height * 4) {
            return Err(PlottingError::RenderError(
                "Invalid DataShader image pixel data".to_string(),
            ));
        }

        let tint = self.theme.foreground;

        // Convert the density mask to tiny-skia's native tinted premultiplied format.
        let pixmap_data = datashader_pixmap.data_mut();
        for (i, chunk) in image.pixels.chunks_exact(4).enumerate() {
            let a = chunk[3];

            // tiny-skia uses premultiplied alpha BGRA format
            let alpha_f = a as f32 / 255.0;
            let premult_r = (tint.r as f32 * alpha_f).round() as u8;
            let premult_g = (tint.g as f32 * alpha_f).round() as u8;
            let premult_b = (tint.b as f32 * alpha_f).round() as u8;

            // BGRA order for tiny-skia
            pixmap_data[i * 4] = premult_b;
            pixmap_data[i * 4 + 1] = premult_g;
            pixmap_data[i * 4 + 2] = premult_r;
            pixmap_data[i * 4 + 3] = a;
        }

        // Scale and draw the DataShader image onto the plot area
        let transform = Transform::from_scale(
            plot_area.width() / image.width as f32,
            plot_area.height() / image.height as f32,
        )
        .post_translate(plot_area.x(), plot_area.y());

        self.pixmap.draw_pixmap(
            0,
            0,
            datashader_pixmap.as_ref(),
            &PixmapPaint::default(),
            transform,
            None,
        );

        Ok(())
    }

    /// Draw text at the specified position using cosmic-text (professional quality).
    /// `y` is interpreted as the top of the text rendering area.
    pub fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color) -> Result<()> {
        match self.text_engine_mode {
            TextEngineMode::Plain => {
                let config = FontConfig::new(self.font_config.family.clone(), size);
                self.text_renderer
                    .render_text(&mut self.pixmap, text, x, y, &config, color)
            }
            #[cfg(feature = "typst-math")]
            TextEngineMode::Typst => {
                let size_pt = self.typst_size_pt(size);
                let rendered =
                    typst_text::render_raster(text, size_pt, color, 0.0, "Skia text rendering")?;
                let (draw_x, draw_y) = typst_text::anchored_top_left(
                    x,
                    y,
                    rendered.width,
                    rendered.height,
                    TypstTextAnchor::TopLeft,
                );
                self.draw_typst_raster(&rendered, draw_x, draw_y);
                Ok(())
            }
        }
    }

    /// Draw text rotated 90 degrees counterclockwise using cosmic-text
    pub fn draw_text_rotated(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
    ) -> Result<()> {
        match self.text_engine_mode {
            TextEngineMode::Plain => {
                let config = FontConfig::new(self.font_config.family.clone(), size);
                self.text_renderer
                    .render_text_rotated(&mut self.pixmap, text, x, y, &config, color)
            }
            #[cfg(feature = "typst-math")]
            TextEngineMode::Typst => {
                let size_pt = self.typst_size_pt(size);
                let rendered = typst_text::render_raster(
                    text,
                    size_pt,
                    color,
                    -90.0,
                    "Skia rotated text rendering",
                )?;
                let (draw_x, draw_y) = typst_text::anchored_top_left(
                    x,
                    y,
                    rendered.width,
                    rendered.height,
                    TypstTextAnchor::Center,
                );
                self.draw_typst_raster(&rendered, draw_x, draw_y);
                Ok(())
            }
        }
    }

    /// Draw text centered horizontally at the given position.
    /// `y` is interpreted as the top of the text rendering area.
    pub fn draw_text_centered(
        &mut self,
        text: &str,
        center_x: f32,
        y: f32,
        size: f32,
        color: Color,
    ) -> Result<()> {
        match self.text_engine_mode {
            TextEngineMode::Plain => {
                let config = FontConfig::new(self.font_config.family.clone(), size);
                self.text_renderer.render_text_centered(
                    &mut self.pixmap,
                    text,
                    center_x,
                    y,
                    &config,
                    color,
                )
            }
            #[cfg(feature = "typst-math")]
            TextEngineMode::Typst => {
                let size_pt = self.typst_size_pt(size);
                let rendered = typst_text::render_raster(
                    text,
                    size_pt,
                    color,
                    0.0,
                    "Skia centered text rendering",
                )?;
                let (draw_x, draw_y) = typst_text::anchored_top_left(
                    center_x,
                    y,
                    rendered.width,
                    rendered.height,
                    TypstTextAnchor::TopCenter,
                );
                self.draw_typst_raster(&rendered, draw_x, draw_y);
                Ok(())
            }
        }
    }

    /// Measure text dimensions
    pub fn measure_text(&self, text: &str, size: f32) -> Result<(f32, f32)> {
        match self.text_engine_mode {
            TextEngineMode::Plain => {
                let config = FontConfig::new(self.font_config.family.clone(), size);
                self.text_renderer.measure_text(text, &config)
            }
            #[cfg(feature = "typst-math")]
            TextEngineMode::Typst => {
                let size_pt = self.typst_size_pt(size);
                typst_text::measure_text(
                    text,
                    size_pt,
                    self.theme.foreground,
                    0.0,
                    TypstBackendKind::Raster,
                    "Skia text measurement",
                )
            }
        }
    }

    pub(crate) fn measure_text_ink_center_from_top(&self, text: &str, size: f32) -> Result<f32> {
        match self.text_engine_mode {
            TextEngineMode::Plain => {
                let config = FontConfig::new(self.font_config.family.clone(), size);
                self.text_renderer
                    .measure_text_ink_center_from_top(text, &config)
            }
            #[cfg(feature = "typst-math")]
            TextEngineMode::Typst => Ok(self.measure_text(text, size)?.1 / 2.0),
        }
    }

    pub(crate) fn measure_label_text(&self, text: &str, size: f32) -> Result<(f32, f32)> {
        let label_snippet = self.generated_label(text);
        self.measure_text(&label_snippet, size)
    }

    fn generated_label<'a>(&self, text: &'a str) -> Cow<'a, str> {
        #[cfg(feature = "typst-math")]
        if self.text_engine_mode.uses_typst() {
            return Cow::Owned(typst_text::literal_text_snippet(text));
        }

        Cow::Borrowed(text)
    }

    /// Draw axis labels and tick values using spacing configuration
    ///
    /// Positions tick labels and axis labels using `spacing.tick_pad` and `spacing.label_pad`
    /// for consistent, DPI-independent spacing.
    pub fn draw_axis_labels(
        &mut self,
        plot_area: Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        x_label: &str,
        y_label: &str,
        color: Color,
        label_size: f32,
        dpi: f32,
        spacing: &SpacingConfig,
    ) -> Result<()> {
        let tick_size = label_size * 0.7; // Tick labels slightly smaller than axis labels
        let render_scale = RenderScale::new(dpi);

        // Convert spacing config values from points to pixels
        let tick_pad_px = pt_to_px(spacing.tick_pad, dpi);
        let label_pad_px = pt_to_px(spacing.label_pad, dpi);
        let char_width_estimate = render_scale.logical_pixels_to_pixels(4.0);

        // Generate ticks and format all labels with consistent precision
        let x_ticks = generate_ticks(x_min, x_max, 5);
        let y_ticks = generate_ticks(y_min, y_max, 5);
        let x_labels = format_tick_labels(&x_ticks);
        let y_labels = format_tick_labels(&y_ticks);

        // Draw X-axis tick labels
        for (tick_value, label_text) in x_ticks.iter().zip(x_labels.iter()) {
            let x_pixel = plot_area.left()
                + (*tick_value - x_min) as f32 / (x_max - x_min) as f32 * plot_area.width();

            let text_width_estimate = label_text.len() as f32 * char_width_estimate / 2.0;
            let label_x = (x_pixel - text_width_estimate)
                .max(0.0)
                .min(self.width() as f32 - text_width_estimate * 2.0);
            // Position tick labels with tick_pad below the axis
            let label_y = (plot_area.bottom() + tick_pad_px + tick_size)
                .min(self.height() as f32 - tick_size - 5.0);
            let label_snippet = self.generated_label(label_text);
            self.draw_text(&label_snippet, label_x, label_y, tick_size, color)?;
        }

        // Draw Y-axis tick labels
        for (tick_value, label_text) in y_ticks.iter().zip(y_labels.iter()) {
            let y_pixel = plot_area.bottom()
                - (*tick_value - y_min) as f32 / (y_max - y_min) as f32 * plot_area.height();

            let text_width_estimate = label_text.len() as f32 * char_width_estimate;
            // Position tick labels with tick_pad left of the axis
            let label_x = (plot_area.left() - text_width_estimate - tick_pad_px).max(5.0);
            let label_snippet = self.generated_label(label_text);
            self.draw_text(
                &label_snippet,
                label_x,
                y_pixel - tick_size / 3.0,
                tick_size,
                color,
            )?;
        }

        // Draw X-axis label: positioned label_pad below the tick labels
        let x_label_x =
            plot_area.left() + plot_area.width() / 2.0 - x_label.len() as f32 * char_width_estimate;
        // X-label goes below tick labels: bottom + tick_pad + tick_size + label_pad
        let x_label_y = plot_area.bottom() + tick_pad_px + tick_size + label_pad_px + label_size;
        self.draw_text(x_label, x_label_x, x_label_y, label_size, color)?;

        // Draw Y-axis label (rotated 90 degrees counterclockwise)
        // Position label_pad left of the tick labels
        // Estimate tick label width (assume ~4 characters average)
        let estimated_tick_width = 4.0 * char_width_estimate;
        let y_label_x = plot_area.left() - tick_pad_px - estimated_tick_width - label_pad_px;
        let y_label_y = plot_area.top() + plot_area.height() / 2.0;
        self.draw_text_rotated(y_label, y_label_x, y_label_y, label_size, color)?;

        // Draw border around plot area
        self.draw_plot_border(plot_area, color, render_scale.reference_scale())?;

        Ok(())
    }

    /// Draw axis labels with DPI scale (legacy compatibility)
    pub fn draw_axis_labels_legacy(
        &mut self,
        plot_area: Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        x_label: &str,
        y_label: &str,
        color: Color,
        label_size: f32,
        dpi_scale: f32,
    ) -> Result<()> {
        let tick_size = label_size * 0.7;
        let render_scale = RenderScale::from_reference_scale(dpi_scale);
        let tick_offset_y = render_scale.logical_pixels_to_pixels(20.0);
        let x_label_offset = render_scale.logical_pixels_to_pixels(50.0);
        let y_label_offset = render_scale.logical_pixels_to_pixels(25.0);
        let char_width_estimate = render_scale.logical_pixels_to_pixels(4.0);

        // Generate ticks and format all labels with consistent precision
        let x_ticks = generate_ticks(x_min, x_max, 5);
        let y_ticks = generate_ticks(y_min, y_max, 5);
        let x_labels = format_tick_labels(&x_ticks);
        let y_labels = format_tick_labels(&y_ticks);

        for (tick_value, label_text) in x_ticks.iter().zip(x_labels.iter()) {
            let x_pixel = plot_area.left()
                + (*tick_value - x_min) as f32 / (x_max - x_min) as f32 * plot_area.width();
            let text_width_estimate = label_text.len() as f32 * char_width_estimate / 2.0;
            let label_x = (x_pixel - text_width_estimate)
                .max(0.0)
                .min(self.width() as f32 - text_width_estimate * 2.0);
            let label_y =
                (plot_area.bottom() + tick_offset_y).min(self.height() as f32 - tick_size - 5.0);
            let label_snippet = self.generated_label(label_text);
            self.draw_text(&label_snippet, label_x, label_y, tick_size, color)?;
        }

        for (tick_value, label_text) in y_ticks.iter().zip(y_labels.iter()) {
            let y_pixel = plot_area.bottom()
                - (*tick_value - y_min) as f32 / (y_max - y_min) as f32 * plot_area.height();
            let text_width_estimate = label_text.len() as f32 * char_width_estimate;
            let label_x = (plot_area.left()
                - text_width_estimate
                - render_scale.logical_pixels_to_pixels(15.0))
            .max(5.0);
            let label_snippet = self.generated_label(label_text);
            self.draw_text(
                &label_snippet,
                label_x,
                y_pixel - tick_size / 3.0,
                tick_size,
                color,
            )?;
        }

        let x_label_x =
            plot_area.left() + plot_area.width() / 2.0 - x_label.len() as f32 * char_width_estimate;
        let x_label_y = plot_area.bottom() + x_label_offset;
        self.draw_text(x_label, x_label_x, x_label_y, label_size, color)?;

        let y_label_x = plot_area.left() - y_label_offset;
        let y_label_y = plot_area.top() + plot_area.height() / 2.0;
        self.draw_text_rotated(y_label, y_label_x, y_label_y, label_size, color)?;

        self.draw_plot_border(plot_area, color, dpi_scale)?;

        Ok(())
    }

    /// Draw axis labels and tick values with provided major ticks
    pub fn draw_axis_labels_with_ticks(
        &mut self,
        plot_area: Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        x_major_ticks: &[f64],
        y_major_ticks: &[f64],
        x_label: &str,
        y_label: &str,
        color: Color,
        label_size: f32,
        dpi_scale: f32,
    ) -> Result<()> {
        let tick_size = label_size * 0.7; // Tick labels slightly smaller than axis labels
        let render_scale = RenderScale::from_reference_scale(dpi_scale);

        // Spacing constants are authored in logical pixels and resolved via RenderScale.
        let tick_offset_y = render_scale.logical_pixels_to_pixels(25.0);
        let x_label_offset = render_scale.logical_pixels_to_pixels(55.0);
        let y_label_offset = render_scale.logical_pixels_to_pixels(50.0);
        let y_tick_offset = render_scale.logical_pixels_to_pixels(15.0);
        let char_width_estimate = render_scale.logical_pixels_to_pixels(4.0);

        // Format all tick labels with consistent precision
        let x_labels = format_tick_labels(x_major_ticks);
        let y_labels = format_tick_labels(y_major_ticks);

        // Draw X-axis tick labels using provided major ticks
        for (tick_value, label_text) in x_major_ticks.iter().zip(x_labels.iter()) {
            let x_pixel = plot_area.left()
                + (*tick_value - x_min) as f32 / (x_max - x_min) as f32 * plot_area.width();

            // Center X-axis tick labels horizontally under the tick mark, with proper offset
            // Ensure labels don't overflow canvas bounds
            let text_width_estimate = label_text.len() as f32 * char_width_estimate / 2.0;
            let label_x = (x_pixel - text_width_estimate)
                .max(0.0)
                .min(self.width() as f32 - text_width_estimate * 2.0);
            let label_y =
                (plot_area.bottom() + tick_offset_y).min(self.height() as f32 - tick_size - 5.0); // Ensure within canvas
            let label_snippet = self.generated_label(label_text);
            self.draw_text(&label_snippet, label_x, label_y, tick_size, color)?;
        }

        // Draw Y-axis tick labels using provided major ticks
        for (tick_value, label_text) in y_major_ticks.iter().zip(y_labels.iter()) {
            let y_pixel = plot_area.bottom()
                - (*tick_value - y_min) as f32 / (y_max - y_min) as f32 * plot_area.height();

            // Right-align Y-axis tick labels next to the tick mark with proper offset
            // Ensure labels fit within the left margin space
            let text_width_estimate = label_text.len() as f32 * char_width_estimate;
            let label_x = (plot_area.left() - text_width_estimate - y_tick_offset).max(5.0); // Ensure minimum 5px from canvas edge
            let label_snippet = self.generated_label(label_text);
            self.draw_text(
                &label_snippet,
                label_x,
                y_pixel + tick_size * 0.3,
                tick_size,
                color,
            )?;
        }

        // Draw X-axis label
        let x_label_x =
            plot_area.left() + plot_area.width() / 2.0 - x_label.len() as f32 * char_width_estimate;
        let x_label_y = plot_area.bottom() + x_label_offset;
        self.draw_text(x_label, x_label_x, x_label_y, label_size, color)?;

        // Draw Y-axis label (rotated 90 degrees counterclockwise)
        // Calculate required margin based on rotated text dimensions
        let estimated_text_width = y_label.len() as f32 * label_size * 0.8;
        let improved_y_label_offset = (estimated_text_width * 0.6).max(y_label_offset);
        let y_label_x = plot_area.left() - improved_y_label_offset;
        let y_label_y = plot_area.top() + plot_area.height() / 2.0;
        self.draw_text_rotated(y_label, y_label_x, y_label_y, label_size, color)?;

        // Draw border around plot area
        self.draw_plot_border(plot_area, color, dpi_scale)?;

        Ok(())
    }

    /// Draw axis labels with categorical x-axis labels for bar charts (legacy style)
    ///
    /// Similar to `draw_axis_labels_with_ticks` but uses category names on x-axis
    /// instead of numeric tick values.
    ///
    /// Uses the same data-to-pixel mapping as bar rendering to ensure precise alignment.
    pub fn draw_axis_labels_with_categories(
        &mut self,
        plot_area: Rect,
        categories: &[String],
        y_min: f64,
        y_max: f64,
        y_major_ticks: &[f64],
        x_label: &str,
        y_label: &str,
        color: Color,
        label_size: f32,
        dpi_scale: f32,
    ) -> Result<()> {
        let tick_size = label_size * 0.7;
        let render_scale = RenderScale::from_reference_scale(dpi_scale);
        let tick_offset_y = render_scale.logical_pixels_to_pixels(25.0);
        let x_label_offset = render_scale.logical_pixels_to_pixels(55.0);
        let y_label_offset = render_scale.logical_pixels_to_pixels(50.0);
        let y_tick_offset = render_scale.logical_pixels_to_pixels(15.0);
        let char_width_estimate = render_scale.logical_pixels_to_pixels(4.0);

        // Draw X-axis category labels using same data-to-pixel mapping as bars
        let n_categories = categories.len();
        if n_categories > 0 {
            // X-axis range with matplotlib-compatible padding: [-0.5, n-0.5]
            let x_min = -0.5_f64;
            let x_max = n_categories as f64 - 0.5;
            let x_range = x_max - x_min;

            for (i, category) in categories.iter().enumerate() {
                // Position label at category index (same as bar center in data space)
                let x_data = i as f64;
                let x_center =
                    plot_area.left() + ((x_data - x_min) / x_range) as f32 * plot_area.width();

                // Estimate text width for centering
                let text_width_estimate = category.len() as f32 * char_width_estimate / 2.0;
                let label_x = (x_center - text_width_estimate)
                    .max(0.0)
                    .min(self.width() as f32 - text_width_estimate * 2.0);
                let label_y = (plot_area.bottom() + tick_offset_y)
                    .min(self.height() as f32 - tick_size - 5.0);

                self.draw_text(category, label_x, label_y, tick_size, color)?;
            }
        }

        // Draw Y-axis tick labels with consistent precision
        let y_labels = format_tick_labels(y_major_ticks);
        for (tick_value, label_text) in y_major_ticks.iter().zip(y_labels.iter()) {
            let y_pixel = plot_area.bottom()
                - (*tick_value - y_min) as f32 / (y_max - y_min) as f32 * plot_area.height();

            let text_width_estimate = label_text.len() as f32 * char_width_estimate;
            let label_x = (plot_area.left() - text_width_estimate - y_tick_offset).max(5.0);
            let label_snippet = self.generated_label(label_text);
            self.draw_text(
                &label_snippet,
                label_x,
                y_pixel + tick_size * 0.3,
                tick_size,
                color,
            )?;
        }

        // Draw X-axis label
        let x_label_x =
            plot_area.left() + plot_area.width() / 2.0 - x_label.len() as f32 * char_width_estimate;
        let x_label_y = plot_area.bottom() + x_label_offset;
        self.draw_text(x_label, x_label_x, x_label_y, label_size, color)?;

        // Draw Y-axis label (rotated)
        let estimated_text_width = y_label.len() as f32 * label_size * 0.8;
        let improved_y_label_offset = (estimated_text_width * 0.6).max(y_label_offset);
        let y_label_x = plot_area.left() - improved_y_label_offset;
        let y_label_y = plot_area.top() + plot_area.height() / 2.0;
        self.draw_text_rotated(y_label, y_label_x, y_label_y, label_size, color)?;

        // Draw border around plot area
        self.draw_plot_border(plot_area, color, dpi_scale)?;

        Ok(())
    }

    /// Draw border around plot area
    pub fn draw_plot_border(
        &mut self,
        plot_area: Rect,
        color: Color,
        dpi_scale: f32,
    ) -> Result<()> {
        // Matches the full-frame axis width used by draw_axes/draw_axes_with_config.
        let border_width =
            RenderScale::from_reference_scale(dpi_scale).logical_pixels_to_pixels(1.5);

        // Create border paint
        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(color.r, color.g, color.b, color.a);
        paint.anti_alias = true;

        // Create stroke
        let stroke = tiny_skia::Stroke {
            width: border_width,
            ..tiny_skia::Stroke::default()
        };

        // Draw rectangle border around plot area
        let path = tiny_skia::PathBuilder::from_rect(plot_area);
        self.pixmap.stroke_path(
            &path,
            &paint,
            &stroke,
            tiny_skia::Transform::identity(),
            None,
        );

        Ok(())
    }

    /// Draw title using spacing configuration
    ///
    /// The title is positioned near the top of the canvas with minimal padding.
    pub fn draw_title(
        &mut self,
        title: &str,
        _plot_area: Rect,
        color: Color,
        title_size: f32,
        dpi: f32,
        _spacing: &SpacingConfig,
    ) -> Result<()> {
        // Center title horizontally over the entire canvas width
        let canvas_center_x = self.width() as f32 / 2.0;

        // Position title near top of canvas with small top padding
        // Text baseline is at title_y, so top of text is roughly at title_y - title_size * 0.8
        let top_padding = RenderScale::new(dpi).logical_pixels_to_pixels(8.0);
        let title_y = top_padding + title_size;

        self.draw_text_centered(title, canvas_center_x, title_y, title_size, color)
    }

    /// Draw title at a computed position from LayoutCalculator
    ///
    /// This is the preferred method for content-driven layout.
    pub fn draw_title_at(&mut self, pos: &TextPosition, text: &str, color: Color) -> Result<()> {
        self.draw_text_centered(text, pos.x, pos.y, pos.size, color)
    }

    /// Draw X-axis label at a computed position from LayoutCalculator
    ///
    /// This is the preferred method for content-driven layout.
    pub fn draw_xlabel_at(&mut self, pos: &TextPosition, text: &str, color: Color) -> Result<()> {
        self.draw_text_centered(text, pos.x, pos.y, pos.size, color)
    }

    /// Draw Y-axis label at a computed position from LayoutCalculator
    ///
    /// The text is rotated 90° counterclockwise for vertical display.
    pub fn draw_ylabel_at(&mut self, pos: &TextPosition, text: &str, color: Color) -> Result<()> {
        self.draw_text_rotated(text, pos.x, pos.y, pos.size, color)
    }

    /// Draw axis tick labels and border using layout positions
    ///
    /// Uses the computed positions from LayoutCalculator for precise placement.
    pub fn draw_axis_labels_at(
        &mut self,
        plot_area: &LayoutRect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        x_ticks: &[f64],
        y_ticks: &[f64],
        xtick_baseline_y: f32,
        ytick_right_x: f32,
        tick_size: f32,
        color: Color,
        dpi: f32,
        show_tick_labels: bool,
        draw_border: bool,
    ) -> Result<()> {
        let render_scale = RenderScale::new(dpi);

        // Convert LayoutRect to tiny_skia Rect for border drawing
        let skia_plot_area = Rect::from_ltrb(
            plot_area.left,
            plot_area.top,
            plot_area.right,
            plot_area.bottom,
        )
        .ok_or(PlottingError::InvalidData {
            message: "Invalid plot area dimensions".to_string(),
            position: None,
        })?;

        // Format all tick labels with consistent precision
        let x_labels = format_tick_labels(x_ticks);
        let y_labels = format_tick_labels(y_ticks);

        if show_tick_labels {
            // Draw X-axis tick labels using provided ticks
            for (tick_value, label_text) in x_ticks.iter().zip(x_labels.iter()) {
                let x_pixel = Self::x_label_center(plot_area, *tick_value, x_min, x_max);

                let label_snippet = self.generated_label(label_text);
                let (text_width, _) = self.measure_text(&label_snippet, tick_size)?;
                let label_x = (x_pixel - text_width / 2.0)
                    .max(0.0)
                    .min(self.width() as f32 - text_width);
                self.draw_text(&label_snippet, label_x, xtick_baseline_y, tick_size, color)?;
            }

            // Draw Y-axis tick labels using provided ticks
            for (tick_value, label_text) in y_ticks.iter().zip(y_labels.iter()) {
                let y_pixel = Self::y_label_center(plot_area, *tick_value, y_min, y_max);

                let label_snippet = self.generated_label(label_text);
                let (text_width, text_height) = self.measure_text(&label_snippet, tick_size)?;
                let label_x = (ytick_right_x - text_width).max(0.0);
                let centered_y = y_pixel - text_height / 2.0;
                self.draw_text(&label_snippet, label_x, centered_y, tick_size, color)?;
            }
        }

        if draw_border {
            self.draw_plot_border(skia_plot_area, color, render_scale.reference_scale())?;
        }

        Ok(())
    }

    /// Draw axis tick labels and border using scale-aware layout positions.
    pub(crate) fn draw_axis_labels_at_scaled(
        &mut self,
        plot_area: &LayoutRect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        x_ticks: &[f64],
        y_ticks: &[f64],
        xtick_baseline_y: f32,
        ytick_right_x: f32,
        tick_size: f32,
        color: Color,
        dpi: f32,
        show_tick_labels: bool,
        draw_border: bool,
        x_scale: &crate::axes::AxisScale,
        y_scale: &crate::axes::AxisScale,
    ) -> Result<()> {
        let render_scale = RenderScale::new(dpi);

        let skia_plot_area = Rect::from_ltrb(
            plot_area.left,
            plot_area.top,
            plot_area.right,
            plot_area.bottom,
        )
        .ok_or(PlottingError::InvalidData {
            message: "Invalid plot area dimensions".to_string(),
            position: None,
        })?;

        let x_labels = format_tick_labels_for_scale(x_ticks, x_scale);
        let y_labels = format_tick_labels_for_scale(y_ticks, y_scale);

        if show_tick_labels {
            for (tick_value, label_text) in x_ticks.iter().zip(x_labels.iter()) {
                let x_pixel =
                    Self::x_label_center_scaled(plot_area, *tick_value, x_min, x_max, x_scale);

                let label_snippet = self.generated_label(label_text);
                let (text_width, _) = self.measure_text(&label_snippet, tick_size)?;
                let label_x = (x_pixel - text_width / 2.0)
                    .max(0.0)
                    .min(self.width() as f32 - text_width);
                self.draw_text(&label_snippet, label_x, xtick_baseline_y, tick_size, color)?;
            }

            for (tick_value, label_text) in y_ticks.iter().zip(y_labels.iter()) {
                let y_pixel =
                    Self::y_label_center_scaled(plot_area, *tick_value, y_min, y_max, y_scale);

                let label_snippet = self.generated_label(label_text);
                let (text_width, text_height) = self.measure_text(&label_snippet, tick_size)?;
                let label_x = (ytick_right_x - text_width).max(0.0);
                let centered_y = y_pixel - text_height / 2.0;
                self.draw_text(&label_snippet, label_x, centered_y, tick_size, color)?;
            }
        }

        if draw_border {
            self.draw_plot_border(skia_plot_area, color, render_scale.reference_scale())?;
        }

        Ok(())
    }

    /// Draw axis tick labels with categorical x-axis labels for bar charts
    ///
    /// Similar to `draw_axis_labels_at` but uses category names instead of numeric ticks
    /// on the x-axis. Categories are positioned at the center of each bar.
    ///
    /// Uses the same data-to-pixel mapping as bar rendering to ensure precise alignment.
    /// With bar chart x-range [-0.5, n-0.5], category i maps to position i in data space.
    pub fn draw_axis_labels_at_categorical(
        &mut self,
        plot_area: &LayoutRect,
        categories: &[String],
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        y_ticks: &[f64],
        xtick_baseline_y: f32,
        ytick_right_x: f32,
        tick_size: f32,
        color: Color,
        dpi: f32,
        show_tick_labels: bool,
        draw_border: bool,
    ) -> Result<()> {
        let render_scale = RenderScale::new(dpi);

        // Convert LayoutRect to tiny_skia Rect for border drawing
        let skia_plot_area = Rect::from_ltrb(
            plot_area.left,
            plot_area.top,
            plot_area.right,
            plot_area.bottom,
        )
        .ok_or(PlottingError::InvalidData {
            message: "Invalid plot area dimensions".to_string(),
            position: None,
        })?;

        if show_tick_labels {
            let n_categories = categories.len();
            if n_categories > 0 {
                for (i, category) in categories.iter().enumerate() {
                    let x_center = Self::x_label_center(plot_area, i as f64, x_min, x_max);

                    let label_snippet = self.generated_label(category);
                    let (text_width, _) = self.measure_text(&label_snippet, tick_size)?;
                    let label_x = (x_center - text_width / 2.0)
                        .max(0.0)
                        .min(self.width() as f32 - text_width);

                    self.draw_text(&label_snippet, label_x, xtick_baseline_y, tick_size, color)?;
                }
            }

            let y_labels = format_tick_labels(y_ticks);
            for (tick_value, label_text) in y_ticks.iter().zip(y_labels.iter()) {
                let y_pixel = Self::y_label_center(plot_area, *tick_value, y_min, y_max);

                let label_snippet = self.generated_label(label_text);
                let (text_width, text_height) = self.measure_text(&label_snippet, tick_size)?;
                let label_x = (ytick_right_x - text_width).max(0.0);
                let centered_y = y_pixel - text_height / 2.0;
                self.draw_text(&label_snippet, label_x, centered_y, tick_size, color)?;
            }
        }

        if draw_border {
            self.draw_plot_border(skia_plot_area, color, render_scale.reference_scale())?;
        }

        Ok(())
    }

    /// Draw axis labels for violin/distribution plots with categorical x-axis
    ///
    /// Unlike bar charts which use integer positions (0, 1, 2, ...), violin plots
    /// use arbitrary x-positions (e.g., 0.5 for a single violin). This method
    /// draws category labels at the actual x-positions within the data range.
    ///
    /// # Arguments
    /// * `plot_area` - The computed plot area
    /// * `categories` - Category labels to draw
    /// * `x_positions` - X positions for each category in data space
    /// * `x_min` - Minimum x value (data space)
    /// * `x_max` - Maximum x value (data space)
    /// * `y_min`, `y_max` - Y data range
    /// * `y_ticks` - Y-axis tick values
    /// * Other arguments for positioning and styling
    #[allow(clippy::too_many_arguments)]
    pub fn draw_axis_labels_at_categorical_violin(
        &mut self,
        plot_area: &LayoutRect,
        categories: &[String],
        x_positions: &[f64],
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        y_ticks: &[f64],
        xtick_baseline_y: f32,
        ytick_right_x: f32,
        tick_size: f32,
        color: Color,
        dpi: f32,
        show_tick_labels: bool,
        draw_border: bool,
    ) -> Result<()> {
        let render_scale = RenderScale::new(dpi);

        // Convert LayoutRect to tiny_skia Rect for border drawing
        let skia_plot_area = Rect::from_ltrb(
            plot_area.left,
            plot_area.top,
            plot_area.right,
            plot_area.bottom,
        )
        .ok_or(PlottingError::InvalidData {
            message: "Invalid plot area dimensions".to_string(),
            position: None,
        })?;

        if show_tick_labels {
            for (category, &x_pos) in categories.iter().zip(x_positions.iter()) {
                let x_center = Self::x_label_center(plot_area, x_pos, x_min, x_max);

                let label_snippet = self.generated_label(category);
                let (text_width, _) = self.measure_text(&label_snippet, tick_size)?;
                let label_x = (x_center - text_width / 2.0)
                    .max(0.0)
                    .min(self.width() as f32 - text_width);

                self.draw_text(&label_snippet, label_x, xtick_baseline_y, tick_size, color)?;
            }

            let y_labels = format_tick_labels(y_ticks);
            for (tick_value, label_text) in y_ticks.iter().zip(y_labels.iter()) {
                let y_pixel = Self::y_label_center(plot_area, *tick_value, y_min, y_max);

                let label_snippet = self.generated_label(label_text);
                let (text_width, text_height) = self.measure_text(&label_snippet, tick_size)?;
                let label_x = (ytick_right_x - text_width).max(0.0);
                let centered_y = y_pixel - text_height / 2.0;
                self.draw_text(&label_snippet, label_x, centered_y, tick_size, color)?;
            }
        }

        if draw_border {
            self.draw_plot_border(skia_plot_area, color, render_scale.reference_scale())?;
        }

        Ok(())
    }

    /// Draw title with DPI scale (legacy compatibility)
    ///
    /// This method uses a hardcoded offset for backward compatibility.
    /// Prefer `draw_title` with `SpacingConfig` for new code.
    pub fn draw_title_legacy(
        &mut self,
        title: &str,
        plot_area: Rect,
        color: Color,
        title_size: f32,
        dpi_scale: f32,
    ) -> Result<()> {
        let title_offset =
            RenderScale::from_reference_scale(dpi_scale).logical_pixels_to_pixels(30.0);
        let canvas_center_x = self.width() as f32 / 2.0;
        let title_y = (plot_area.top() - title_offset).max(title_size + 5.0);
        self.draw_text_centered(title, canvas_center_x, title_y, title_size, color)
    }

    /// Draw legend
    pub fn draw_legend(&mut self, legend_items: &[(String, Color)], plot_area: Rect) -> Result<()> {
        if legend_items.is_empty() {
            return Ok(());
        }

        let legend_size = 12.0;
        let legend_spacing = 20.0;
        let legend_x = plot_area.right() - 150.0;
        let mut legend_y = plot_area.top() + 30.0;

        // Draw legend background (simple rectangle)
        let legend_bg = Rect::from_xywh(
            legend_x - 10.0,
            legend_y - 15.0,
            140.0,
            legend_items.len() as f32 * legend_spacing + 10.0,
        )
        .ok_or(PlottingError::InvalidData {
            message: "Invalid legend dimensions".to_string(),
            position: None,
        })?;

        self.draw_rectangle(
            legend_bg.left(),
            legend_bg.top(),
            legend_bg.width(),
            legend_bg.height(),
            Color::new_rgba(255, 255, 255, 200),
            true,
        )?;

        // Draw legend items
        for (label, color) in legend_items {
            // Draw color square
            let color_rect = Rect::from_xywh(legend_x, legend_y - 8.0, 12.0, 12.0).ok_or(
                PlottingError::InvalidData {
                    message: "Invalid legend item dimensions".to_string(),
                    position: None,
                },
            )?;
            self.draw_rectangle(
                color_rect.left(),
                color_rect.top(),
                color_rect.width(),
                color_rect.height(),
                *color,
                true,
            )?;

            // Draw label text
            self.draw_text(
                label,
                legend_x + 20.0,
                legend_y,
                legend_size,
                Color::new_rgba(0, 0, 0, 255),
            )?;

            legend_y += legend_spacing;
        }

        Ok(())
    }

    /// Draw legend with configurable position
    pub fn draw_legend_positioned(
        &mut self,
        legend_items: &[(String, Color)],
        plot_area: Rect,
        position: crate::core::Position,
    ) -> Result<()> {
        if legend_items.is_empty() {
            return Ok(());
        }

        let legend_size = 12.0;
        let legend_spacing = 20.0;
        let legend_width = 140.0;
        let legend_height = legend_items.len() as f32 * legend_spacing + 10.0;

        // Calculate legend position based on position enum
        let center_x = plot_area.left() + plot_area.width() / 2.0;
        let center_y = plot_area.top() + plot_area.height() / 2.0;

        let (legend_x, legend_y) = match position {
            // Best defaults to TopRight in legacy method; full best positioning in draw_legend_full
            crate::core::Position::Best | crate::core::Position::TopRight => (
                plot_area.right() - legend_width - 10.0,
                plot_area.top() + 10.0,
            ),
            crate::core::Position::TopLeft => (plot_area.left() + 10.0, plot_area.top() + 10.0),
            crate::core::Position::TopCenter => {
                (center_x - legend_width / 2.0, plot_area.top() + 10.0)
            }
            crate::core::Position::CenterLeft => {
                (plot_area.left() + 10.0, center_y - legend_height / 2.0)
            }
            crate::core::Position::Center => (
                center_x - legend_width / 2.0,
                center_y - legend_height / 2.0,
            ),
            crate::core::Position::CenterRight => (
                plot_area.right() - legend_width - 10.0,
                center_y - legend_height / 2.0,
            ),
            crate::core::Position::BottomLeft => (
                plot_area.left() + 10.0,
                plot_area.bottom() - legend_height - 10.0,
            ),
            crate::core::Position::BottomCenter => (
                center_x - legend_width / 2.0,
                plot_area.bottom() - legend_height - 10.0,
            ),
            crate::core::Position::BottomRight => (
                plot_area.right() - legend_width - 10.0,
                plot_area.bottom() - legend_height - 10.0,
            ),
            crate::core::Position::Custom { x, y } => (x, y),
        };

        // Draw legend background (simple rectangle)
        let legend_bg =
            Rect::from_xywh(legend_x - 10.0, legend_y - 5.0, legend_width, legend_height).ok_or(
                PlottingError::InvalidData {
                    message: "Invalid legend dimensions".to_string(),
                    position: None,
                },
            )?;

        self.draw_rectangle(
            legend_bg.left(),
            legend_bg.top(),
            legend_bg.width(),
            legend_bg.height(),
            Color::new_rgba(255, 255, 255, 200),
            true,
        )?;

        // Draw legend items
        let mut item_y = legend_y + 10.0;
        for (label, color) in legend_items {
            // Draw color square
            let color_rect = Rect::from_xywh(legend_x, item_y - 8.0, 12.0, 12.0).ok_or(
                PlottingError::InvalidData {
                    message: "Invalid legend item dimensions".to_string(),
                    position: None,
                },
            )?;
            self.draw_rectangle(
                color_rect.left(),
                color_rect.top(),
                color_rect.width(),
                color_rect.height(),
                *color,
                true,
            )?;

            // Draw label text
            self.draw_text(
                label,
                legend_x + 20.0,
                item_y,
                legend_size,
                Color::new_rgba(0, 0, 0, 255),
            )?;

            item_y += legend_spacing;
        }

        Ok(())
    }

    // =========================================================================
    // New Legend System with proper handle rendering
    // =========================================================================

    /// Draw a line handle in the legend (for line series)
    ///
    /// Draws a horizontal line segment with the specified style, color, and width.
    fn draw_legend_line_handle(
        &mut self,
        x: f32,
        y: f32,
        length: f32,
        color: Color,
        style: &LineStyle,
        width: f32,
    ) -> Result<()> {
        // Draw horizontal line at vertical center
        self.draw_line(x, y, x + length, y, color, width, style.clone())
    }

    /// Draw a scatter/marker handle in the legend
    ///
    /// Draws a single marker symbol centered in the handle area.
    fn draw_legend_scatter_handle(
        &mut self,
        x: f32,
        y: f32,
        length: f32,
        color: Color,
        marker: &MarkerStyle,
        size: f32,
    ) -> Result<()> {
        // Draw marker at center of handle area
        let center_x = x + length / 2.0;
        self.draw_marker(center_x, y, size, *marker, color)
    }

    /// Draw a bar handle in the legend
    ///
    /// Draws a filled rectangle to represent bar/histogram series.
    fn draw_legend_bar_handle(
        &mut self,
        x: f32,
        y: f32,
        length: f32,
        height: f32,
        color: Color,
    ) -> Result<()> {
        // Draw filled rectangle centered vertically
        let rect_y = y - height / 2.0;
        self.draw_rectangle(x, rect_y, length, height, color, true)
    }

    /// Draw a line+marker handle in the legend
    ///
    /// Draws a line segment with a marker symbol at the center.
    fn draw_legend_line_marker_handle(
        &mut self,
        x: f32,
        y: f32,
        length: f32,
        color: Color,
        line_style: &LineStyle,
        line_width: f32,
        marker: &MarkerStyle,
        marker_size: f32,
    ) -> Result<()> {
        // Draw line first
        self.draw_legend_line_handle(x, y, length, color, line_style, line_width)?;
        // Draw marker on top at center
        self.draw_legend_scatter_handle(x, y, length, color, marker, marker_size)
    }

    /// Draw a legend handle based on the item type
    fn draw_legend_handle(
        &mut self,
        item: &LegendItem,
        x: f32,
        y: f32,
        spacing: &LegendSpacingPixels,
    ) -> Result<()> {
        let handle_length = spacing.handle_length;
        let handle_height = spacing.handle_height;
        // First draw the base type
        match &item.item_type {
            LegendItemType::Line { style, width } => {
                let scaled_width = self.points_to_pixels(*width);
                self.draw_legend_line_handle(x, y, handle_length, item.color, style, scaled_width)?;
            }
            LegendItemType::Scatter { marker, size } => {
                let scaled_size = self.points_to_pixels(*size);
                self.draw_legend_scatter_handle(
                    x,
                    y,
                    handle_length,
                    item.color,
                    marker,
                    scaled_size,
                )?;
            }
            LegendItemType::LineMarker {
                line_style,
                line_width,
                marker,
                marker_size,
            } => {
                let scaled_line_width = self.points_to_pixels(*line_width);
                let scaled_marker_size = self.points_to_pixels(*marker_size);
                self.draw_legend_line_marker_handle(
                    x,
                    y,
                    handle_length,
                    item.color,
                    line_style,
                    scaled_line_width,
                    marker,
                    scaled_marker_size,
                )?;
            }
            LegendItemType::Bar | LegendItemType::Histogram => {
                self.draw_legend_bar_handle(x, y, handle_length, handle_height, item.color)?;
            }
            LegendItemType::Area { edge_color } => {
                // Draw filled rectangle with optional edge
                self.draw_legend_bar_handle(x, y, handle_length, handle_height, item.color)?;
                if let Some(edge) = edge_color {
                    // Draw edge around the rectangle
                    let rect_y = y - handle_height / 2.0;
                    let scaled_edge_width = self.logical_pixels_to_pixels(1.0);
                    self.draw_rectangle_outline(
                        x,
                        rect_y,
                        handle_length,
                        handle_height,
                        *edge,
                        scaled_edge_width,
                    )?;
                }
            }
            LegendItemType::ErrorBar => {
                // ErrorBar type: Draw vertical error bar with marker (matplotlib-style)
                let center_x = x + handle_length / 2.0;
                let error_height = handle_height * 0.8;
                let half_error = error_height / 2.0;
                let cap_width = handle_height * 0.5;
                let half_cap = cap_width / 2.0;
                let error_line_width = self.logical_pixels_to_pixels(1.5);

                // Vertical error bar line
                self.draw_line(
                    center_x,
                    y - half_error,
                    center_x,
                    y + half_error,
                    item.color,
                    error_line_width,
                    LineStyle::Solid,
                )?;
                // Top cap (horizontal)
                self.draw_line(
                    center_x - half_cap,
                    y - half_error,
                    center_x + half_cap,
                    y - half_error,
                    item.color,
                    error_line_width,
                    LineStyle::Solid,
                )?;
                // Bottom cap (horizontal)
                self.draw_line(
                    center_x - half_cap,
                    y + half_error,
                    center_x + half_cap,
                    y + half_error,
                    item.color,
                    error_line_width,
                    LineStyle::Solid,
                )?;
                // Draw marker in center (handle_height is already in pixels, scale marker proportionally)
                let marker_size = handle_height * 0.4;
                self.draw_marker(center_x, y, marker_size, MarkerStyle::Circle, item.color)?;
            }
        }

        // If the series has attached error bars (not ErrorBar type), overlay error bar indicator
        if item.has_error_bars && !matches!(item.item_type, LegendItemType::ErrorBar) {
            let center_x = x + handle_length / 2.0;
            let error_height = handle_height * 0.7; // Slightly smaller for overlay
            let half_error = error_height / 2.0;
            let cap_width = handle_height * 0.4;
            let half_cap = cap_width / 2.0;
            let overlay_line_width = self.logical_pixels_to_pixels(1.0);

            // Vertical error bar line
            self.draw_line(
                center_x,
                y - half_error,
                center_x,
                y + half_error,
                item.color,
                overlay_line_width,
                LineStyle::Solid,
            )?;
            // Top cap (horizontal)
            self.draw_line(
                center_x - half_cap,
                y - half_error,
                center_x + half_cap,
                y - half_error,
                item.color,
                overlay_line_width,
                LineStyle::Solid,
            )?;
            // Bottom cap (horizontal)
            self.draw_line(
                center_x - half_cap,
                y + half_error,
                center_x + half_cap,
                y + half_error,
                item.color,
                overlay_line_width,
                LineStyle::Solid,
            )?;
        }

        Ok(())
    }

    /// Draw rectangle outline (stroke only, no fill)
    fn draw_rectangle_outline(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
        line_width: f32,
    ) -> Result<()> {
        // Draw 4 lines forming a rectangle
        let x2 = x + width;
        let y2 = y + height;
        self.draw_line(x, y, x2, y, color, line_width, LineStyle::Solid)?;
        self.draw_line(x2, y, x2, y2, color, line_width, LineStyle::Solid)?;
        self.draw_line(x2, y2, x, y2, color, line_width, LineStyle::Solid)?;
        self.draw_line(x, y2, x, y, color, line_width, LineStyle::Solid)
    }

    /// Draw rounded rectangle outline (stroke only, no fill)
    fn draw_rounded_rectangle_outline(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        corner_radius: f32,
        color: Color,
        line_width: f32,
    ) -> Result<()> {
        // Clamp radius to half of the smaller dimension
        let max_radius = (width.min(height) / 2.0).max(0.0);
        let radius = corner_radius.min(max_radius);

        // If radius is effectively zero, use regular rectangle outline
        if radius < 0.1 {
            return self.draw_rectangle_outline(x, y, width, height, color, line_width);
        }

        // Build rounded rectangle path
        let mut pb = PathBuilder::new();

        pb.move_to(x + radius, y);
        pb.line_to(x + width - radius, y);
        pb.quad_to(x + width, y, x + width, y + radius);
        pb.line_to(x + width, y + height - radius);
        pb.quad_to(x + width, y + height, x + width - radius, y + height);
        pb.line_to(x + radius, y + height);
        pb.quad_to(x, y + height, x, y + height - radius);
        pb.line_to(x, y + radius);
        pb.quad_to(x, y, x + radius, y);
        pb.close();

        let path = pb.finish().ok_or(PlottingError::RenderError(
            "Failed to create rounded rectangle outline path".to_string(),
        ))?;

        let mut paint = Paint::default();
        paint.set_color(color.to_tiny_skia_color());
        paint.anti_alias = true;

        let stroke = Stroke {
            width: line_width,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        };

        self.pixmap
            .stroke_path(&path, &paint, &stroke, Transform::identity(), None);

        Ok(())
    }

    /// Draw legend frame with background and optional border
    fn draw_legend_frame(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        style: &LegendStyle,
    ) -> Result<()> {
        if !style.visible {
            return Ok(());
        }

        let radius = style.effective_corner_radius();

        // Draw shadow if enabled
        if style.shadow {
            let (shadow_dx, shadow_dy) = style.shadow_offset;
            if radius > 0.0 {
                self.draw_rounded_rectangle(
                    x + shadow_dx,
                    y + shadow_dy,
                    width,
                    height,
                    radius,
                    style.shadow_color,
                    true,
                )?;
            } else {
                self.draw_rectangle(
                    x + shadow_dx,
                    y + shadow_dy,
                    width,
                    height,
                    style.shadow_color,
                    true,
                )?;
            }
        }

        // Draw background with alpha applied
        let face_color = style.effective_face_color();
        if radius > 0.0 {
            self.draw_rounded_rectangle(x, y, width, height, radius, face_color, true)?;
        } else {
            self.draw_rectangle(x, y, width, height, face_color, true)?;
        }

        // Draw border if specified
        if let Some(edge_color) = style.edge_color {
            if radius > 0.0 {
                self.draw_rounded_rectangle_outline(
                    x,
                    y,
                    width,
                    height,
                    radius,
                    edge_color,
                    style.border_width,
                )?;
            } else {
                self.draw_rectangle_outline(x, y, width, height, edge_color, style.border_width)?;
            }
        }

        Ok(())
    }

    /// Calculate legend dimensions from items
    fn calculate_legend_dimensions(
        &self,
        items: &[LegendItem],
        legend: &Legend,
        char_width: f32,
    ) -> (f32, f32) {
        legend.calculate_size(items, char_width)
    }

    fn scaled_legend_for_render(&self, legend: &Legend) -> Legend {
        let mut scaled = legend.clone();
        scaled.font_size = self.points_to_pixels(legend.font_size);
        scaled.style.border_width = self.points_to_pixels(legend.style.border_width);
        scaled.style.corner_radius = self.points_to_pixels(legend.style.corner_radius);
        scaled.style.shadow_offset = (
            self.points_to_pixels(legend.style.shadow_offset.0),
            self.points_to_pixels(legend.style.shadow_offset.1),
        );
        scaled
    }

    /// Draw legend with full LegendItem support
    ///
    /// This is the new legend drawing method that properly renders different
    /// series types with their correct visual handles.
    pub fn draw_legend_full(
        &mut self,
        items: &[LegendItem],
        legend: &Legend,
        plot_area: Rect,
        data_bboxes: Option<&[(f32, f32, f32, f32)]>,
    ) -> Result<()> {
        if items.is_empty() || !legend.enabled {
            return Ok(());
        }

        let legend = self.scaled_legend_for_render(legend);
        let legend = &legend;
        let spacing = legend.spacing.to_pixels(legend.font_size);

        // Estimate character width for size calculation
        let char_width = legend.font_size * 0.6;

        // Calculate legend size
        let (legend_width, legend_height) =
            self.calculate_legend_dimensions(items, legend, char_width);

        // Determine position
        let plot_bounds = (
            plot_area.left(),
            plot_area.top(),
            plot_area.right(),
            plot_area.bottom(),
        );

        let position = if matches!(legend.position, LegendPosition::Best) {
            // Use best position algorithm
            let bboxes = data_bboxes.unwrap_or(&[]);
            if bboxes.iter().map(|b| 1).sum::<usize>() > 100000 {
                // Performance guard: skip for very large datasets
                LegendPosition::UpperRight
            } else {
                find_best_position(
                    (legend_width, legend_height),
                    plot_bounds,
                    bboxes,
                    &legend.spacing,
                    legend.font_size,
                )
            }
        } else {
            legend.position
        };

        // Create a temporary legend with the resolved position to calculate coordinates
        let resolved_legend = Legend {
            position,
            ..legend.clone()
        };

        let (legend_x, legend_y) =
            resolved_legend.calculate_position((legend_width, legend_height), plot_bounds);

        // Draw frame
        self.draw_legend_frame(
            legend_x,
            legend_y,
            legend_width,
            legend_height,
            &legend.style,
        )?;

        // Starting position for items (inside padding)
        let item_x = legend_x + spacing.border_pad;
        let mut item_y = legend_y + spacing.border_pad + legend.font_size / 2.0;

        // Draw title if present
        if let Some(ref title) = legend.title {
            let title_x = legend_x + legend_width / 2.0;
            self.draw_text_centered(title, title_x, item_y, legend.font_size, legend.text_color)?;
            item_y += legend.font_size + spacing.label_spacing;
        }

        // Calculate items per column
        let items_per_col = items.len().div_ceil(legend.columns);

        // Calculate column width
        let max_label_len = items.iter().map(|item| item.label.len()).max().unwrap_or(0);
        let label_width = max_label_len as f32 * char_width;
        let col_width = spacing.handle_length + spacing.handle_text_pad + label_width;

        // Draw items column by column
        for col in 0..legend.columns {
            let col_x = item_x + col as f32 * (col_width + spacing.column_spacing);
            let mut row_y = item_y;

            for row in 0..items_per_col {
                let idx = col * items_per_col + row;
                if idx >= items.len() {
                    break;
                }

                let item = &items[idx];

                // Draw handle
                self.draw_legend_handle(item, col_x, row_y, &spacing)?;

                // Draw label - vertically centered with handle
                let text_x = col_x + spacing.handle_length + spacing.handle_text_pad;
                // Center text vertically on handle
                let centered_y = row_y - legend.font_size * 0.65;
                self.draw_text(
                    &item.label,
                    text_x,
                    centered_y,
                    legend.font_size,
                    legend.text_color,
                )?;

                row_y += legend.font_size + spacing.label_spacing;
            }
        }

        Ok(())
    }

    /// Draw a colorbar for heatmaps
    ///
    /// Draws a vertical gradient bar showing the color mapping from vmin to vmax,
    /// with tick marks and optional label.
    ///
    /// # Arguments
    ///
    /// * `colormap` - The color map to sample from
    /// * `vmin` - Minimum value in the data range
    /// * `vmax` - Maximum value in the data range
    /// * `x` - X position of colorbar (left edge)
    /// * `y` - Y position of colorbar (top edge)
    /// * `width` - Width of the colorbar
    /// * `height` - Height of the colorbar
    /// * `value_scale` - Scale used to normalize values along the colorbar
    /// * `label` - Optional label to display (rotated 90°)
    /// * `foreground_color` - Color for ticks, text, and border
    /// * `tick_font_size` - Font size for tick labels (in points)
    /// * `label_font_size` - Font size for colorbar label (in points, optional)
    /// * `show_log_subticks` - Whether to draw unlabeled logarithmic subticks
    pub fn draw_colorbar(
        &mut self,
        colormap: &crate::render::ColorMap,
        vmin: f64,
        vmax: f64,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        value_scale: &crate::axes::AxisScale,
        label: Option<&str>,
        foreground_color: Color,
        tick_font_size: f32,
        label_font_size: Option<f32>,
        show_log_subticks: bool,
    ) -> Result<()> {
        // Use tick font size for label if not specified separately
        let label_font_size = label_font_size.unwrap_or(tick_font_size * 1.1);

        // Draw the colorbar gradient (vertical, from vmax at top to vmin at bottom)
        // Use one segment per pixel row to eliminate anti-aliasing artifacts
        let num_segments = (height as usize).max(50);
        let segment_height = height / num_segments as f32;

        for i in 0..num_segments {
            // Map segment to value (top = vmax, bottom = vmin)
            let normalized = 1.0 - (i as f64 / (num_segments - 1).max(1) as f64);
            let color = colormap.sample(normalized);
            let segment_y = y + i as f32 * segment_height;

            // Use solid rectangle with small overlap to ensure seamless gradient
            // draw_solid_rectangle has 100% opacity and no anti-aliasing
            self.draw_solid_rectangle(x, segment_y, width, segment_height + 0.5, color)?;
        }

        // Draw border around colorbar
        let stroke_width = 1.0;
        self.draw_rectangle(x, y, width, height, foreground_color, false)?;

        let ticks = compute_colorbar_ticks(vmin, vmax, value_scale, show_log_subticks);
        let mut measured_major_labels = Vec::with_capacity(ticks.major_labels.len());
        let mut max_label_width: f32 = 0.0;
        for label_text in &ticks.major_labels {
            let label_snippet = self.generated_label(label_text);
            let (text_width, _) = self.measure_text(&label_snippet, tick_font_size)?;
            let ink_center_from_top =
                self.measure_text_ink_center_from_top(&label_snippet, tick_font_size)?;
            max_label_width = max_label_width.max(text_width);
            measured_major_labels.push((label_snippet, ink_center_from_top));
        }

        let rotated_label_width = if let Some(label_text) = label {
            Some(self.measure_text(label_text, label_font_size)?.1)
        } else {
            None
        };
        let log_decade_base_center = matches!(value_scale, crate::axes::AxisScale::Log)
            .then(|| self.measure_text_ink_center_from_top("10", tick_font_size))
            .transpose()?;
        let layout = compute_colorbar_layout_metrics(
            width,
            tick_font_size,
            max_label_width,
            rotated_label_width,
        );

        for minor_value in &ticks.minor_values {
            let t = value_scale
                .normalized_position(*minor_value, vmin, vmax)
                .clamp(0.0, 1.0);
            let tick_y = y + height * (1.0 - t as f32);

            self.draw_line(
                x + width,
                tick_y,
                x + width + layout.minor_tick_width,
                tick_y,
                foreground_color,
                stroke_width * 0.8,
                LineStyle::Solid,
            )?;
        }

        for ((value, _), (label_text, ink_center_from_top)) in ticks
            .major_values
            .iter()
            .zip(ticks.major_labels.iter())
            .zip(measured_major_labels.iter())
        {
            // Map value to Y position (top = vmax, bottom = vmin)
            let t = value_scale
                .normalized_position(*value, vmin, vmax)
                .clamp(0.0, 1.0);
            let tick_y = y + height * (1.0 - t as f32);

            // Draw tick mark
            self.draw_line(
                x + width,
                tick_y,
                x + width + layout.major_tick_width,
                tick_y,
                foreground_color,
                stroke_width,
                LineStyle::Solid,
            )?;

            let anchor_center = colorbar_major_label_anchor_center_from_top(
                value_scale,
                label_text,
                *ink_center_from_top,
                log_decade_base_center,
            );
            let label_y = colorbar_major_label_top(tick_y, anchor_center);
            self.draw_text(
                label_text,
                x + layout.tick_label_x_offset,
                label_y,
                tick_font_size,
                foreground_color,
            )?;
        }

        // Draw colorbar label (rotated 90 degrees) if provided
        if let Some((label, label_center_x_offset)) =
            label.zip(layout.rotated_label_center_x_offset)
        {
            let label_x = x + label_center_x_offset;
            let label_y = y + height / 2.0;
            self.draw_text_rotated(label, label_x, label_y, label_font_size, foreground_color)?;
        }

        Ok(())
    }

    /// Consume the renderer and convert to an `Image`.
    ///
    /// The returned pixel buffer preserves tiny-skia's native premultiplied
    /// alpha representation so it can be composed back into other pixmaps
    /// without a lossy round-trip.
    pub fn into_image(self) -> Image {
        Image {
            width: self.width,
            height: self.height,
            pixels: self.pixmap.data().to_vec(),
        }
    }

    /// Save the current pixmap as a PNG with straight-alpha RGBA encoding.
    pub fn save_png<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        crate::export::write_bytes_atomic(path, &self.encode_png_bytes()?)
    }

    /// Encode the current pixmap as PNG bytes with straight-alpha RGBA encoding.
    pub fn encode_png_bytes(&self) -> Result<Vec<u8>> {
        let image = Image {
            width: self.width,
            height: self.height,
            pixels: self.pixmap.clone().take_demultiplied(),
        };
        crate::export::encode_rgba_png(&image)
    }

    /// Export as SVG (simplified - tiny-skia doesn't directly support SVG export)
    pub fn export_svg<P: AsRef<Path>>(&self, path: P, width: u32, height: u32) -> Result<()> {
        // For now, create a basic SVG placeholder
        // In a real implementation, we'd need to track draw commands and convert to SVG
        let svg_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg">
  <rect width="100%" height="100%" fill="{}"/>
  <text x="50%" y="50%" text-anchor="middle" font-family="Arial" font-size="16">
    Ruviz Plot ({} x {})
  </text>
</svg>"#,
            width, height, self.theme.background, width, height
        );

        crate::export::write_bytes_atomic(path, svg_content.as_bytes())
    }

    /// Get the width of the renderer
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get the height of the renderer  
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Draw a subplot image at the specified position
    pub fn draw_subplot(
        &mut self,
        subplot_image: crate::core::plot::Image,
        x: u32,
        y: u32,
    ) -> Result<()> {
        let subplot_png = subplot_image.encode_png()?;
        let subplot_pixmap = tiny_skia::Pixmap::decode_png(&subplot_png).map_err(|error| {
            PlottingError::RenderError(format!("Failed to decode subplot image: {error}"))
        })?;

        // Draw the subplot pixmap onto our main pixmap at the specified position
        self.pixmap.draw_pixmap(
            x as i32,
            y as i32,
            subplot_pixmap.as_ref(),
            &tiny_skia::PixmapPaint::default(),
            tiny_skia::Transform::identity(),
            None,
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests;
