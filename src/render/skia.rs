use crate::{
    core::{
        ComputedMargins, CoordinateTransform, LayoutRect, Legend, LegendItem, LegendItemType,
        LegendSpacingPixels, LegendStyle, PlottingError, RenderScale, Result, SpacingConfig,
        SpineConfig, TextPosition,
        legend::{
            LEGACY_LEGEND_SWATCH_EDGE_DARK, LEGACY_LEGEND_SWATCH_EDGE_LIGHT,
            LEGACY_LEGEND_SWATCH_EDGE_WIDTH_PT, LegendLayout, LegendOccupancy, LegendPlacement,
            layout_legend, legacy_legend_swatch_edge, measure_legend_size,
        },
        plot::{Image, RenderDiagnostics, TextEngineMode, TickDirection, TickSides},
        pt_to_px,
    },
    render::{
        Color, FontConfig, FontFamily, FontWeight, LineStyle, MarkerStyle, TextRenderer, Theme,
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
    map_data_to_pixels_scaled, try_map_data_to_pixels_scaled,
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
    /// `(edge rgba, edge width in device pixels)`, `None` for a bare marker.
    ///
    /// The rim is baked into the sprite, so it has to be part of the identity of
    /// the sprite. Without it an edged batch would have to fall off the sprite
    /// compositor entirely, which is what used to make a marker rim expensive
    /// enough to be worth disabling by default.
    edge_bits: Option<(u32, u32)>,
    phase_x: u8,
    phase_y: u8,
}

/// Entries the process-wide marker sprite cache holds before it starts evicting.
///
/// Sized from [`SkiaRenderer::marker_subpixel_phases`]: one hot marker — a
/// single (style, size, colour, edge) tuple — occupies at most `phases²`
/// entries once a dense scatter has visited every sub-pixel phase, so the limit
/// has to be a multiple of that or a single series would evict the whole cache
/// on every frame. Two hot markers fit.
const GLOBAL_MARKER_SPRITE_CACHE_LIMIT: usize =
    2 * (SkiaRenderer::marker_subpixel_phases() as usize).pow(2);

/// Frame colour for the legacy `draw_legend*` panels (matplotlib `legend.edgecolor`).
const LEGACY_LEGEND_EDGE_COLOR: Color = Color {
    r: 204,
    g: 204,
    b: 204,
    a: 200,
};
/// Frame width for the legacy `draw_legend*` panels, in points.
const LEGACY_LEGEND_EDGE_WIDTH_PT: f32 = 0.8;

static GLOBAL_MARKER_SPRITE_CACHE: OnceLock<Mutex<HashMap<MarkerSpriteKey, Arc<MarkerSprite>>>> =
    OnceLock::new();

fn global_marker_sprite_cache() -> &'static Mutex<HashMap<MarkerSpriteKey, Arc<MarkerSprite>>> {
    GLOBAL_MARKER_SPRITE_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn insert_global_marker_sprite(
    global_cache: &mut HashMap<MarkerSpriteKey, Arc<MarkerSprite>>,
    key: MarkerSpriteKey,
    sprite: Arc<MarkerSprite>,
) -> Arc<MarkerSprite> {
    if let Some(existing) = global_cache.get(&key).cloned() {
        return existing;
    }

    if global_cache.len() >= GLOBAL_MARKER_SPRITE_CACHE_LIMIT
        && let Some(evicted_key) = global_cache.keys().next().copied()
    {
        global_cache.remove(&evicted_key);
    }

    global_cache.insert(key, Arc::clone(&sprite));
    sprite
}

impl MarkerSpriteKey {
    fn new(
        style: MarkerStyle,
        size: f32,
        color: Color,
        edge: Option<(Color, f32)>,
        phase_x: u8,
        phase_y: u8,
    ) -> Self {
        Self {
            style,
            size_bits: size.to_bits(),
            rgba_bits: u32::from_be_bytes([color.r, color.g, color.b, color.a]),
            edge_bits: edge.map(|(color, width_px)| {
                (
                    u32::from_be_bytes([color.r, color.g, color.b, color.a]),
                    width_px.to_bits(),
                )
            }),
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
    /// How the x tick label row is drawn; see [`XTickLabelPlan`].
    x_tick_label_plan: XTickLabelPlan,
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
        let font_family = FontFamily::from(theme.font_family.as_str());
        Self::with_font_family(width, height, theme, font_family)
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
            x_tick_label_plan: XTickLabelPlan::default(),
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

    /// Set how the x tick label row is drawn.
    ///
    /// The plan is resolved once, against the margin the layout actually
    /// granted, and then held here — so the row that was measured is the row
    /// that is drawn. Left at its default a row is horizontal and complete,
    /// which is what every caller that never measured one wants.
    pub fn set_x_tick_label_plan(&mut self, plan: XTickLabelPlan) {
        self.x_tick_label_plan = plan;
    }

    /// How the x tick label row is drawn.
    pub fn x_tick_label_plan(&self) -> XTickLabelPlan {
        self.x_tick_label_plan
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

    /// Set the font family used by plain and Typst text rendering.
    pub fn set_font_family<F>(&mut self, family: F)
    where
        F: Into<FontFamily>,
    {
        self.font_config.family = family.into();
    }

    /// Get the configured font family.
    pub fn font_family(&self) -> &FontFamily {
        &self.font_config.family
    }

    pub(crate) fn set_render_mode_diagnostics(&mut self, mode: &'static str) {
        self.render_diagnostics.render_mode = mode;
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

    /// Fetch (or build) the cached raster for one marker.
    ///
    /// `edge` is `(colour, width in **device pixels**)` — already scaled by the
    /// caller, exactly like the vector painter takes it — and is baked into the
    /// sprite, so an edged batch keeps the sprite fast path.
    pub(crate) fn marker_sprite(
        &mut self,
        style: MarkerStyle,
        size: f32,
        color: Color,
        edge: Option<(Color, f32)>,
        phase_x: u8,
        phase_y: u8,
    ) -> Result<Arc<MarkerSprite>> {
        let key = MarkerSpriteKey::new(style, size, color, edge, phase_x, phase_y);
        if let Some(sprite) = self.marker_sprite_cache.get(&key) {
            let sprite = Arc::clone(sprite);
            self.note_marker_sprite_cache();
            return Ok(sprite);
        }

        if let Ok(mut global_cache) = global_marker_sprite_cache().lock() {
            if let Some(sprite) = global_cache.get(&key).cloned() {
                self.marker_sprite_cache.insert(key, Arc::clone(&sprite));
                self.note_marker_sprite_cache();
                return Ok(sprite);
            }

            // Hold the global lock across creation to avoid duplicate same-key sprite work.
            // If parallel PNG workloads make unrelated misses contend here, switch to per-key slots.
            let sprite =
                Arc::new(self.create_marker_sprite(style, size, color, edge, phase_x, phase_y)?);
            let sprite = insert_global_marker_sprite(&mut global_cache, key, sprite);
            self.marker_sprite_cache.insert(key, Arc::clone(&sprite));
            self.note_marker_sprite_cache();
            return Ok(sprite);
        }

        let sprite =
            Arc::new(self.create_marker_sprite(style, size, color, edge, phase_x, phase_y)?);
        self.marker_sprite_cache.insert(key, Arc::clone(&sprite));
        self.note_marker_sprite_cache();
        Ok(sprite)
    }

    fn create_marker_sprite(
        &self,
        style: MarkerStyle,
        size: f32,
        color: Color,
        edge: Option<(Color, f32)>,
        phase_x: u8,
        phase_y: u8,
    ) -> Result<MarkerSprite> {
        let (origin, side) = Self::marker_sprite_geometry(style, size, edge);
        let mut sprite_renderer = SkiaRenderer::new(side, side, self.theme.clone())?;
        sprite_renderer.set_render_scale(self.render_scale);
        sprite_renderer.set_text_engine_mode(self.text_engine_mode);
        sprite_renderer.pixmap.fill(tiny_skia::Color::TRANSPARENT);

        let phase_step = 1.0 / Self::marker_subpixel_phases() as f32;
        let center_x = origin as f32 + phase_x as f32 * phase_step;
        let center_y = origin as f32 + phase_y as f32 * phase_step;

        sprite_renderer.draw_marker_styled_with_mask_vector(
            center_x, center_y, size, style, color, edge, None,
        )?;

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

    /// Sub-pixel positions a cached marker sprite is rasterised at, per axis.
    ///
    /// The sprite compositor snaps every marker centre to the nearest phase, so
    /// this is the only place the fast path disagrees with the vector painter:
    /// a marker lands up to `1 / (2 * PHASES)` device pixels off its exact
    /// position, and the anti-aliased boundary pixels shift with it.
    ///
    /// 64 (not 32) because a marker *rim* is a thin high-contrast feature and
    /// therefore samples that error far more harshly than a bare fill does: at
    /// 32 phases an edged batch showed ~10x the boundary noise of the same
    /// batch drawn one marker at a time, with per-channel deltas past 32. At 64
    /// the worst edged delta is the same as the worst edgeless one, i.e. the
    /// rim no longer costs accuracy. Squaring this bounds the per-batch sprite
    /// table and the cache limit below, so it cannot grow without thought.
    pub(crate) const fn marker_subpixel_phases() -> u8 {
        64
    }

    /// Sprite origin and side length for one marker.
    ///
    /// `edge` is `(colour, width in device pixels)`; a rim straddles the shape's
    /// boundary, so half of it lies outside the fill and the sprite has to be
    /// padded for it or the rim would be clipped off at the sprite border.
    pub(crate) fn marker_sprite_geometry(
        style: MarkerStyle,
        size: f32,
        edge: Option<(Color, f32)>,
    ) -> (i32, u32) {
        let radius = size * 0.5;
        let edge_half = edge
            .filter(|_| style.takes_edge())
            .map(|(_, width_px)| width_px * 0.5)
            .unwrap_or(0.0);
        let stroke_half = match style {
            MarkerStyle::SquareOpen => (size * 0.15).max(1.0) * 0.5,
            MarkerStyle::TriangleOpen | MarkerStyle::DiamondOpen => (size * 0.15).max(1.0) * 0.5,
            MarkerStyle::Plus | MarkerStyle::Cross => (size * 0.25).max(1.0) * 0.5,
            MarkerStyle::Star => (size * 0.22).max(1.0) * 0.5,
            _ => 0.5,
        }
        .max(edge_half);
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
        if x_min == x_max
            || (!matches!(scale, crate::axes::AxisScale::Log)
                && (x_max - x_min).abs() < f64::EPSILON)
        {
            plot_area.center_x()
        } else {
            let normalized = scale.normalized_position(x_value, x_min, x_max);
            plot_area.left + normalized as f32 * plot_area.width()
        }
    }

    fn y_label_center_scaled(
        plot_area: &LayoutRect,
        y_value: f64,
        y_min: f64,
        y_max: f64,
        scale: &crate::axes::AxisScale,
    ) -> f32 {
        if y_min == y_max
            || (!matches!(scale, crate::axes::AxisScale::Log)
                && (y_max - y_min).abs() < f64::EPSILON)
        {
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

        self.draw_axes_with_minor_ticks_styled(
            plot_area,
            x_major_ticks,
            y_major_ticks,
            x_minor_ticks,
            y_minor_ticks,
            tick_direction,
            tick_sides,
            &SpineConfig::default(),
            color,
            axis_width,
            major_tick_size,
            minor_tick_size,
            major_tick_width,
            minor_tick_width,
        )
    }

    /// Draw axis lines with caller-supplied axis and tick metrics in pixels.
    pub fn draw_axes_with_minor_ticks_styled(
        &mut self,
        plot_area: Rect,
        x_major_ticks: &[f32],
        y_major_ticks: &[f32],
        x_minor_ticks: &[f32],
        y_minor_ticks: &[f32],
        tick_direction: &TickDirection,
        tick_sides: &TickSides,
        spines: &SpineConfig,
        color: Color,
        axis_width: f32,
        major_tick_size: f32,
        minor_tick_size: f32,
        major_tick_width: f32,
        minor_tick_width: f32,
    ) -> Result<()> {
        fn snap_stroke_coord(coord: f32, width: f32) -> f32 {
            if !coord.is_finite() || !width.is_finite() {
                return coord;
            }
            let rounded_width = width.round().max(1.0) as i32;
            let offset = if rounded_width % 2 == 0 { 0.0 } else { 0.5 };
            (coord - offset).round() + offset
        }

        fn snap_endpoint(coord: f32) -> f32 {
            if coord.is_finite() {
                coord.round()
            } else {
                coord
            }
        }

        let spine_offset = self.render_scale.points_to_pixels(spines.offset.max(0.0));
        let plot_left = snap_endpoint(plot_area.left());
        let plot_right = snap_endpoint(plot_area.right());
        let plot_top = snap_endpoint(plot_area.top());
        let plot_bottom = snap_endpoint(plot_area.bottom());
        let bottom_spine_y = snap_stroke_coord(plot_area.bottom() + spine_offset, axis_width);
        let top_spine_y = snap_stroke_coord(plot_area.top() - spine_offset, axis_width);
        let left_spine_x = snap_stroke_coord(plot_area.left() - spine_offset, axis_width);
        let right_spine_x = snap_stroke_coord(plot_area.right() + spine_offset, axis_width);

        if spines.bottom {
            self.draw_line(
                plot_left,
                bottom_spine_y,
                plot_right,
                bottom_spine_y,
                color,
                axis_width,
                LineStyle::Solid,
            )?;
        }

        if spines.left {
            self.draw_line(
                left_spine_x,
                plot_top,
                left_spine_x,
                plot_bottom,
                color,
                axis_width,
                LineStyle::Solid,
            )?;
        }

        if spines.top {
            self.draw_line(
                plot_left,
                top_spine_y,
                plot_right,
                top_spine_y,
                color,
                axis_width,
                LineStyle::Solid,
            )?;
        }

        if spines.right {
            self.draw_line(
                right_spine_x,
                plot_top,
                right_spine_x,
                plot_bottom,
                color,
                axis_width,
                LineStyle::Solid,
            )?;
        }

        for (tick_size, tick_width, ticks) in [
            (major_tick_size, major_tick_width, x_major_ticks),
            (minor_tick_size, minor_tick_width, x_minor_ticks),
        ] {
            for &x in ticks {
                if x >= plot_area.left() && x <= plot_area.right() {
                    let x = snap_stroke_coord(x, tick_width);
                    if tick_sides.bottom && spines.bottom {
                        let (tick_start, tick_end) = Self::vertical_tick_span(
                            bottom_spine_y,
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
                    if tick_sides.top && spines.top {
                        let (tick_start, tick_end) =
                            Self::vertical_tick_span(top_spine_y, tick_size, tick_direction, true);
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
                    let y = snap_stroke_coord(y, tick_width);
                    if tick_sides.left && spines.left {
                        let (tick_start, tick_end) = Self::horizontal_tick_span(
                            left_spine_x,
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
                    if tick_sides.right && spines.right {
                        let (tick_start, tick_end) = Self::horizontal_tick_span(
                            right_spine_x,
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

        // Convert the density mask to tiny-skia's native tinted premultiplied
        // format. `Pixmap::data_mut` is premultiplied **RGBA**, not BGRA: this
        // used to write B, G, R, A and so swapped red and blue. It went
        // unnoticed because every theme's `foreground` is black, white or grey,
        // where the swap is invisible.
        let pixmap_data = datashader_pixmap.data_mut();
        for (i, chunk) in image.pixels.chunks_exact(4).enumerate() {
            let a = chunk[3];

            let alpha_f = a as f32 / 255.0;
            let premult_r = (tint.r as f32 * alpha_f).round() as u8;
            let premult_g = (tint.g as f32 * alpha_f).round() as u8;
            let premult_b = (tint.b as f32 * alpha_f).round() as u8;

            pixmap_data[i * 4] = premult_r;
            pixmap_data[i * 4 + 1] = premult_g;
            pixmap_data[i * 4 + 2] = premult_b;
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
        // Bucket 1 of the geometry policy in `primitives.rs`, and the exact
        // twin of `SvgRenderer::draw_text`: a label the axes cannot place is
        // skipped, not raised. Without this the glyph run is laid out at `NaN`
        // and every glyph quantises to 0 on the way to the pixmap, blitting the
        // label into the top-left corner where it reads as real content.
        if !Self::all_finite(&[x, y, size]) {
            return Ok(());
        }
        match self.text_engine_mode {
            TextEngineMode::Plain => {
                let config = FontConfig::new(self.font_config.family.clone(), size);
                self.text_renderer
                    .render_text(&mut self.pixmap, text, x, y, &config, color)
            }
            #[cfg(feature = "typst-math")]
            TextEngineMode::Typst => {
                let size_pt = self.typst_size_pt(size);
                let rendered = typst_text::render_raster_with_font_family(
                    text,
                    size_pt,
                    color,
                    0.0,
                    &self.font_config.family,
                    "Skia text rendering",
                )?;
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
        // See `draw_text`: an unplaceable label is skipped, not raised.
        if !Self::all_finite(&[x, y, size]) {
            return Ok(());
        }
        match self.text_engine_mode {
            TextEngineMode::Plain => {
                let config = FontConfig::new(self.font_config.family.clone(), size);
                self.text_renderer
                    .render_text_rotated(&mut self.pixmap, text, x, y, &config, color)
            }
            #[cfg(feature = "typst-math")]
            TextEngineMode::Typst => {
                let size_pt = self.typst_size_pt(size);
                let rendered = typst_text::render_raster_with_font_family(
                    text,
                    size_pt,
                    color,
                    -90.0,
                    &self.font_config.family,
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
        self.draw_text_centered_with_weight(text, center_x, y, size, color, FontWeight::Normal)
    }

    pub(crate) fn draw_text_centered_with_weight(
        &mut self,
        text: &str,
        center_x: f32,
        y: f32,
        size: f32,
        color: Color,
        weight: FontWeight,
    ) -> Result<()> {
        // See `draw_text`: an unplaceable label is skipped, not raised.
        if !Self::all_finite(&[center_x, y, size]) {
            return Ok(());
        }
        match self.text_engine_mode {
            TextEngineMode::Plain => {
                let config = FontConfig::new(self.font_config.family.clone(), size).weight(weight);
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
                let multiline_text = typst_text::with_explicit_line_breaks(text);
                let weighted_text = typst_text::with_font_weight(&multiline_text, weight);
                let aligned_text = typst_text::with_horizontal_alignment(
                    &weighted_text,
                    crate::core::TextAlign::Center,
                );
                let rendered = typst_text::render_raster_with_font_family(
                    &aligned_text,
                    size_pt,
                    color,
                    0.0,
                    &self.font_config.family,
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
        self.measure_text_with_weight(text, size, FontWeight::Normal)
    }

    pub(crate) fn measure_text_with_weight(
        &self,
        text: &str,
        size: f32,
        weight: FontWeight,
    ) -> Result<(f32, f32)> {
        match self.text_engine_mode {
            TextEngineMode::Plain => {
                let config = FontConfig::new(self.font_config.family.clone(), size).weight(weight);
                self.text_renderer.measure_text(text, &config)
            }
            #[cfg(feature = "typst-math")]
            TextEngineMode::Typst => {
                let size_pt = self.typst_size_pt(size);
                let multiline_text = typst_text::with_explicit_line_breaks(text);
                let weighted_text = typst_text::with_font_weight(&multiline_text, weight);
                let aligned_text = typst_text::with_horizontal_alignment(
                    &weighted_text,
                    crate::core::TextAlign::Center,
                );
                typst_text::measure_text_with_font_family(
                    &aligned_text,
                    size_pt,
                    self.theme.foreground,
                    0.0,
                    TypstBackendKind::Raster,
                    &self.font_config.family,
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
        self.draw_title_at_with_weight(pos, text, color, FontWeight::Normal)
    }

    pub(crate) fn draw_title_at_with_weight(
        &mut self,
        pos: &TextPosition,
        text: &str,
        color: Color,
        weight: FontWeight,
    ) -> Result<()> {
        self.draw_text_centered_with_weight(text, pos.x, pos.y, pos.size, color, weight)
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
    /// Draw axis tick labels and border on a linear axis pair.
    ///
    /// Thin wrapper over `draw_axis_labels_at_scaled` with linear
    /// scales — it exists only so callers that genuinely have no scale to hand
    /// keep working. It is deliberately not a second implementation.
    #[allow(clippy::too_many_arguments)]
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
        self.draw_axis_labels_at_scaled(
            plot_area,
            x_min,
            x_max,
            y_min,
            y_max,
            x_ticks,
            y_ticks,
            xtick_baseline_y,
            ytick_right_x,
            tick_size,
            color,
            dpi,
            show_tick_labels,
            draw_border,
            &crate::axes::AxisScale::Linear,
            &crate::axes::AxisScale::Linear,
        )
    }

    /// Draw the y-axis tick labels.
    ///
    /// This is the single implementation shared by the numeric and both
    /// categorical axis-label paths. It takes the scale by value rather than
    /// defaulting to linear so that a caller physically cannot draw y ticks
    /// without saying which scale they belong to — that omission is exactly how
    /// the categorical paths ended up labelling a log axis "1000" while the
    /// numeric path drew "10³".
    fn draw_y_tick_labels(
        &mut self,
        plot_area: &LayoutRect,
        y_ticks: &[f64],
        y_min: f64,
        y_max: f64,
        y_scale: &crate::axes::AxisScale,
        ytick_right_x: f32,
        tick_size: f32,
        color: Color,
    ) -> Result<()> {
        let y_labels = format_tick_labels_for_scale(y_ticks, y_scale);

        for (tick_value, label_text) in y_ticks.iter().zip(y_labels.iter()) {
            let y_pixel =
                Self::y_label_center_scaled(plot_area, *tick_value, y_min, y_max, y_scale);

            let label_snippet = self.generated_label(label_text);
            let (text_width, text_height) = self.measure_text(&label_snippet, tick_size)?;
            let label_x = (ytick_right_x - text_width).max(0.0);
            let centered_y = y_pixel - text_height / 2.0;
            self.draw_text(&label_snippet, label_x, centered_y, tick_size, color)?;
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

        if show_tick_labels {
            let x_labels = format_tick_labels_for_scale(x_ticks, x_scale);
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

            self.draw_y_tick_labels(
                plot_area,
                y_ticks,
                y_min,
                y_max,
                y_scale,
                ytick_right_x,
                tick_size,
                color,
            )?;
        }

        if draw_border {
            self.draw_plot_border(skia_plot_area, color, render_scale.reference_scale())?;
        }

        Ok(())
    }

    /// Pixel centre of every categorical slot, in axis order.
    ///
    /// One formula for the measurement, the raster row and the SVG row: a label
    /// measured at one x and drawn at another is the collision this row exists
    /// to avoid, dressed up as a rounding difference.
    pub fn categorical_label_centers(
        plot_area: &LayoutRect,
        x_positions: &[f64],
        x_min: f64,
        x_max: f64,
    ) -> Vec<f32> {
        x_positions
            .iter()
            .map(|&x_position| Self::x_label_center(plot_area, x_position, x_min, x_max))
            .collect()
    }

    /// Measure an x tick label row: how much room it needs, and how far apart
    /// its labels have to be spaced to stop overlapping.
    ///
    /// Labels are measured as the text engine will lay them out, and an empty
    /// label measures nothing — an unnamed slot holds its place on the axis
    /// without writing under it.
    pub fn measure_x_tick_row(
        &self,
        labels: &[String],
        centers: &[f32],
        size: f32,
        bounds: XTickRowBounds,
    ) -> Result<XTickRowMetrics> {
        let mut widths = Vec::with_capacity(labels.len());
        let mut heights = Vec::with_capacity(labels.len());
        let mut horizontal_extent = 0.0_f32;
        let mut max_label_width = 0.0_f32;

        for label in labels {
            if label.is_empty() {
                widths.push(0.0);
                heights.push(0.0);
                continue;
            }
            let (width, height) = self.measure_label_text(label, size)?;
            widths.push(width);
            heights.push(height);
            horizontal_extent = horizontal_extent.max(height);
            max_label_width = max_label_width.max(width);
        }

        let gap = size * X_TICK_LABEL_GAP_EM;
        // One gutter for the whole row: the same clearance a label keeps from
        // its neighbour it also keeps from the figure edge.
        let bounds = bounds.inset(gap);
        Ok(XTickRowMetrics {
            horizontal_extent,
            max_label_width,
            horizontal_stride: clearing_stride(centers, &widths, gap, bounds),
            // Turned a quarter turn, a label is only as wide as it is tall, so
            // its neighbours are cleared by its height rather than its width.
            rotated_stride: clearing_stride(centers, &heights, gap, bounds),
            bounds,
        })
    }

    /// Draw axis tick labels with a categorical x axis.
    ///
    /// Every categorical plot type — bar, box plot, violin, boxen — reaches this
    /// one drawer, with the slot centres from `CategoryAxis::harvest`. A
    /// bar chart's slots happen to be `0..n-1`; there used to be a second copy of
    /// this function that assumed that and could not express anything else, which
    /// is why a violin needed its own.
    ///
    /// A slot whose series carries no category name has an empty label and draws
    /// nothing — it still holds its place on the axis.
    ///
    /// The label row follows [`SkiaRenderer::x_tick_label_plan`], so ten region
    /// names turn a quarter turn instead of overlapping into one illegible run.
    ///
    /// # Arguments
    /// * `plot_area` - The computed plot area
    /// * `categories` - Category labels to draw, in axis order
    /// * `x_positions` - Slot centre for each category, in data space
    /// * `x_min` - Minimum x value (data space)
    /// * `x_max` - Maximum x value (data space)
    /// * `y_min`, `y_max` - Y data range
    /// * `y_ticks` - Y-axis tick values
    /// * Other arguments for positioning and styling
    #[allow(clippy::too_many_arguments)]
    pub fn draw_axis_labels_at_categorical(
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
        y_scale: &crate::axes::AxisScale,
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
            let centers = Self::categorical_label_centers(plot_area, x_positions, x_min, x_max);
            let plan = self.x_tick_label_plan;
            draw_x_tick_label_row(
                self,
                categories,
                &centers,
                xtick_baseline_y,
                tick_size,
                color,
                plan,
            )?;

            self.draw_y_tick_labels(
                plot_area,
                y_ticks,
                y_min,
                y_max,
                y_scale,
                ytick_right_x,
                tick_size,
                color,
            )?;
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

        // Frame the panel explicitly: the fill primitive no longer adds a border.
        self.draw_rectangle_styled(
            legend_bg.left(),
            legend_bg.top(),
            legend_bg.width(),
            legend_bg.height(),
            Some(Color::from_rgba(255, 255, 255, 200)),
            Some((LEGACY_LEGEND_EDGE_COLOR, LEGACY_LEGEND_EDGE_WIDTH_PT)),
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
            // Fill is exactly the series colour; the neutral edge is what keeps
            // a white/near-white key visible on the near-white panel.
            self.draw_rectangle_styled(
                color_rect.left(),
                color_rect.top(),
                color_rect.width(),
                color_rect.height(),
                Some(*color),
                Some((
                    legacy_legend_swatch_edge(*color),
                    LEGACY_LEGEND_SWATCH_EDGE_WIDTH_PT,
                )),
            )?;

            // Draw label text
            self.draw_text(
                label,
                legend_x + 20.0,
                legend_y,
                legend_size,
                Color::from_rgba(0, 0, 0, 255),
            )?;

            legend_y += legend_spacing;
        }

        Ok(())
    }

    /// Draw legend with configurable position.
    ///
    /// Accepts a [`LegendPosition`](crate::core::LegendPosition) or the
    /// deprecated [`Position`](crate::core::Position), which converts losslessly.
    pub fn draw_legend_positioned(
        &mut self,
        legend_items: &[(String, Color)],
        plot_area: Rect,
        position: impl Into<crate::core::LegendPosition>,
    ) -> Result<()> {
        let position = position.into();
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

        use crate::core::LegendPosition as LP;
        let (legend_x, legend_y) = match position {
            // `Best` defaults to upper-right in this legacy helper; full best
            // positioning lives in `draw_legend_full`. The `Outside*` variants
            // have no margin to expand into here, so they fall back to the
            // nearest inside placement.
            LP::Best | LP::UpperRight | LP::Right | LP::OutsideRight | LP::OutsideUpper => (
                plot_area.right() - legend_width - 10.0,
                plot_area.top() + 10.0,
            ),
            LP::UpperLeft | LP::OutsideLeft => (plot_area.left() + 10.0, plot_area.top() + 10.0),
            LP::UpperCenter => (center_x - legend_width / 2.0, plot_area.top() + 10.0),
            LP::CenterLeft => (plot_area.left() + 10.0, center_y - legend_height / 2.0),
            LP::Center => (
                center_x - legend_width / 2.0,
                center_y - legend_height / 2.0,
            ),
            LP::CenterRight => (
                plot_area.right() - legend_width - 10.0,
                center_y - legend_height / 2.0,
            ),
            LP::LowerLeft => (
                plot_area.left() + 10.0,
                plot_area.bottom() - legend_height - 10.0,
            ),
            LP::LowerCenter | LP::OutsideLower => (
                center_x - legend_width / 2.0,
                plot_area.bottom() - legend_height - 10.0,
            ),
            LP::LowerRight => (
                plot_area.right() - legend_width - 10.0,
                plot_area.bottom() - legend_height - 10.0,
            ),
            // `Custom` is a fraction of the plot area with Y growing upward,
            // matching `Legend::calculate_position`.
            LP::Custom { x, y, .. } => (
                plot_area.left() + x * plot_area.width(),
                plot_area.top() + (1.0 - y) * plot_area.height(),
            ),
        };

        // Draw legend background (simple rectangle)
        let legend_bg =
            Rect::from_xywh(legend_x - 10.0, legend_y - 5.0, legend_width, legend_height).ok_or(
                PlottingError::InvalidData {
                    message: "Invalid legend dimensions".to_string(),
                    position: None,
                },
            )?;

        // Frame the panel explicitly: the fill primitive no longer adds a border.
        self.draw_rectangle_styled(
            legend_bg.left(),
            legend_bg.top(),
            legend_bg.width(),
            legend_bg.height(),
            Some(Color::from_rgba(255, 255, 255, 200)),
            Some((LEGACY_LEGEND_EDGE_COLOR, LEGACY_LEGEND_EDGE_WIDTH_PT)),
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
            // Fill is exactly the series colour; the neutral edge is what keeps
            // a white/near-white key visible on the near-white panel.
            self.draw_rectangle_styled(
                color_rect.left(),
                color_rect.top(),
                color_rect.width(),
                color_rect.height(),
                Some(*color),
                Some((
                    legacy_legend_swatch_edge(*color),
                    LEGACY_LEGEND_SWATCH_EDGE_WIDTH_PT,
                )),
            )?;

            // Draw label text
            self.draw_text(
                label,
                legend_x + 20.0,
                item_y,
                legend_size,
                Color::from_rgba(0, 0, 0, 255),
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
    /// Draw a marker handle in the legend
    ///
    /// The fill is always exactly `color`. `edge` is the rim the plotted
    /// markers carry, as `(colour, width_in_points)`; `draw_marker_styled`
    /// scales the width, so the key matches the plot at any DPI.
    fn draw_legend_scatter_handle(
        &mut self,
        x: f32,
        y: f32,
        length: f32,
        color: Color,
        marker: &MarkerStyle,
        size: f32,
        edge: Option<(Color, f32)>,
    ) -> Result<()> {
        // Draw marker at center of handle area
        let center_x = x + length / 2.0;
        self.draw_marker_styled(center_x, y, size, *marker, color, edge)
    }

    /// Draw a bar handle in the legend
    ///
    /// Draws a filled rectangle to represent bar/histogram series.
    ///
    /// The fill is always exactly `color` — a legend key has to reproduce the
    /// series colour. `edge` is the stroke the corresponding patch is drawn
    /// with, as `(colour, width_in_points)`; the width goes through the render
    /// scale so the key matches the plot at any DPI. `None` draws a flat patch.
    fn draw_legend_bar_handle(
        &mut self,
        x: f32,
        y: f32,
        length: f32,
        height: f32,
        color: Color,
        edge: Option<(Color, f32)>,
    ) -> Result<()> {
        // Draw filled rectangle centered vertically
        let rect_y = y - height / 2.0;
        self.draw_rectangle_styled(x, rect_y, length, height, Some(color), edge)
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
        marker_edge: Option<(Color, f32)>,
    ) -> Result<()> {
        // Draw line first
        self.draw_legend_line_handle(x, y, length, color, line_style, line_width)?;
        // Draw marker on top at center
        self.draw_legend_scatter_handle(x, y, length, color, marker, marker_size, marker_edge)
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
            LegendItemType::Scatter { marker, size, edge } => {
                let scaled_size = self.points_to_pixels(*size);
                self.draw_legend_scatter_handle(
                    x,
                    y,
                    handle_length,
                    item.color,
                    marker,
                    scaled_size,
                    *edge,
                )?;
            }
            LegendItemType::LineMarker {
                line_style,
                line_width,
                marker,
                marker_size,
                marker_edge,
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
                    *marker_edge,
                )?;
            }
            LegendItemType::Bar { edge } | LegendItemType::Histogram { edge } => {
                let edge = *edge;
                self.draw_legend_bar_handle(x, y, handle_length, handle_height, item.color, edge)?;
            }
            LegendItemType::Area { edge_color } => {
                // Draw filled rectangle with optional edge
                self.draw_legend_bar_handle(x, y, handle_length, handle_height, item.color, None)?;
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
    /// Paint a legend frame: shadow, face and edge, from one [`LegendStyle`].
    ///
    /// `pub(crate)` so the 3D overlay paints its legend box with this exact
    /// code rather than a themed look-alike of it. `style` must already be in
    /// device pixels (see `Legend::scaled_for_render`).
    pub(crate) fn draw_legend_frame(
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
                // Flat fill: a shadow must never gain an outline of its own.
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

        // Draw background with alpha applied. Flat fill in both branches: the
        // frame border is drawn below from `style.edge_color`/`border_width`,
        // which is also what the rounded branch has always done.
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

    /// Size and place the legend through the one shared layout.
    ///
    /// `legend` must already be scaled for this renderer. The measurement
    /// callback is this backend's own, which is how a Typst-shaped label and a
    /// cosmic-text one stay honestly different without duplicating the layout.
    fn legend_layout(
        &self,
        items: &[LegendItem],
        legend: &Legend,
        plot_area: (f32, f32, f32, f32),
        placement: LegendPlacement<'_>,
    ) -> Result<LegendLayout> {
        layout_legend(items, legend, plot_area, placement, |text| {
            Ok(self.measure_label_text(text, legend.font_size)?.0)
        })
    }

    /// The room this legend needs, measured exactly as it will be drawn.
    ///
    /// The figure-level margin reservation calls this; it shares
    /// [`layout_legend`] with [`SkiaRenderer::draw_legend_full`], so an outside
    /// legend can no longer be reserved at one width and drawn at another.
    ///
    /// `legend` is in points and is scaled for this renderer internally.
    pub(crate) fn measure_legend(
        &self,
        items: &[LegendItem],
        legend: &Legend,
    ) -> Result<(f32, f32)> {
        let legend = legend.scaled_for_render(self.render_scale);
        measure_legend_size(items, &legend, |text| {
            Ok(self.measure_label_text(text, legend.font_size)?.0)
        })
    }

    /// Draw legend with full LegendItem support
    ///
    /// This is the new legend drawing method that properly renders different
    /// series types with their correct visual handles.
    ///
    /// `occupancy` is only consulted for
    /// [`LegendPosition::Best`](crate::core::LegendPosition::Best); `None`
    /// means "no idea where the data is", which degrades to `UpperRight`.
    pub fn draw_legend_full(
        &mut self,
        items: &[LegendItem],
        legend: &Legend,
        plot_area: Rect,
        occupancy: Option<&LegendOccupancy>,
    ) -> Result<()> {
        self.draw_legend_full_resolved(items, legend, plot_area, occupancy, None)
    }

    pub(crate) fn draw_legend_full_resolved(
        &mut self,
        items: &[LegendItem],
        legend: &Legend,
        plot_area: Rect,
        occupancy: Option<&LegendOccupancy>,
        resolved_rect: Option<(f32, f32, f32, f32)>,
    ) -> Result<()> {
        if items.is_empty() || !legend.enabled {
            return Ok(());
        }

        let legend = legend.scaled_for_render(self.render_scale);
        let bounds = (
            plot_area.left(),
            plot_area.top(),
            plot_area.right(),
            plot_area.bottom(),
        );
        let placement = LegendPlacement {
            reserved: resolved_rect,
            occupancy,
        };
        let layout = self.legend_layout(items, &legend, bounds, placement)?;

        self.draw_legend_frame(
            layout.x,
            layout.y,
            layout.width,
            layout.height,
            &legend.style,
        )?;

        if let (Some(title_layout), Some(title)) = (layout.title, legend.title.as_deref()) {
            self.draw_text_centered(
                title,
                title_layout.center_x,
                title_layout.top_y,
                layout.font_size,
                legend.text_color,
            )?;
        }

        for entry in &layout.entries {
            let item = &items[entry.item_index];
            self.draw_legend_handle(item, entry.handle_x, entry.handle_center_y, &layout.spacing)?;
            self.draw_text(
                &item.label,
                entry.label_x,
                entry.label_top_y,
                layout.font_size,
                legend.text_color,
            )?;
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
        crate::render::colorbar::draw_colorbar(
            self,
            &crate::render::colorbar::ColorbarSpec {
                colormap,
                vmin,
                vmax,
                x,
                y,
                width,
                height,
                value_scale,
                label,
                foreground_color,
                tick_font_size,
                label_font_size,
                show_log_subticks,
            },
        )
    }

    /// Consume the renderer and convert to an `Image`.
    ///
    /// Tiny-skia's native premultiplied buffer is normalized to [`Image`]'s
    /// canonical straight-alpha representation.
    /// `Pixmap::take` hands over the pixmap's own buffer, so the normalization
    /// happens in place. Copying it out first (`data().to_vec()`) cost a second
    /// full-frame pass — 20 MB of memcpy per frame at a 2800x1800 backing size.
    pub fn into_image(self) -> Image {
        Image::from_premultiplied_rgba(self.width, self.height, self.pixmap.take())
    }

    /// Consume the renderer and take tiny-skia's native premultiplied buffer as
    /// a [`RenderedLayer`], with no conversion and no copy.
    ///
    /// Prefer this over [`into_image`](Self::into_image) for presentation paths
    /// that upload to a GPU texture: the straight-alpha normalization
    /// `into_image` performs is a full-frame divide that premultiplying
    /// toolkits immediately undo.
    pub fn into_rendered_layer(self) -> crate::core::plot::RenderedLayer {
        crate::core::plot::RenderedLayer::from_premultiplied_pixels(
            self.width,
            self.height,
            self.pixmap.take(),
        )
    }

    /// Consume the renderer and convert to an `Image` with straight-alpha
    /// (demultiplied) RGBA pixels.
    ///
    /// Use this when the buffer will be composed by straight-alpha blenders
    /// (e.g. the interactive overlay compositor) rather than tiny-skia.
    pub fn into_image_demultiplied(self) -> Image {
        Image::from_straight_rgba(self.width, self.height, self.pixmap.take_demultiplied())
    }

    /// Save the current pixmap as a PNG with straight-alpha RGBA encoding.
    pub fn save_png<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        crate::export::write_bytes_atomic(path, &self.encode_png_bytes()?)
    }

    /// Encode the current pixmap as PNG bytes with straight-alpha RGBA encoding.
    pub fn encode_png_bytes(&self) -> Result<Vec<u8>> {
        let image = Image::from_straight_rgba(
            self.width,
            self.height,
            self.pixmap.clone().take_demultiplied(),
        );
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

    /// Draw a subplot image at the specified position.
    ///
    /// Alias for [`Self::draw_image_layer`]. Prefer `draw_image_layer`, which
    /// borrows instead of consuming.
    pub fn draw_subplot(
        &mut self,
        subplot_image: crate::core::plot::Image,
        x: u32,
        y: u32,
    ) -> Result<()> {
        self.draw_image_layer(&subplot_image, x, y)
    }

    /// Compose an RGBA image onto the canvas.
    ///
    /// This is the only way an [`Image`] is put on
    /// the canvas — [`Self::draw_subplot`] is a thin alias — so the 2D subplot
    /// compositor and the 3D overlay compositor cannot drift apart.
    ///
    /// It used to go through `encode_png` + `decode_png`, which cost a full
    /// deflate *and* inflate of the whole canvas for every composited frame
    /// (~11 MB per 1920x1440 3D orbit frame) purely to change alpha
    /// representation. The canonical straight-alpha input is premultiplied
    /// directly here.
    pub fn draw_image_layer(
        &mut self,
        image: &crate::core::plot::Image,
        x: u32,
        y: u32,
    ) -> Result<()> {
        let expected = (image.width as usize)
            .saturating_mul(image.height as usize)
            .saturating_mul(4);
        if image.pixels.len() != expected {
            return Err(PlottingError::RenderError(
                "image layer pixel buffer does not match its dimensions".to_string(),
            ));
        }

        let premultiplied = image.pixels_in_alpha_mode(crate::core::plot::AlphaMode::Premultiplied);

        let size = tiny_skia::IntSize::from_wh(image.width, image.height)
            .ok_or(PlottingError::OutOfMemory)?;
        let layer =
            Pixmap::from_vec(premultiplied.into_owned(), size).ok_or(PlottingError::OutOfMemory)?;
        self.pixmap.draw_pixmap(
            x as i32,
            y as i32,
            layer.as_ref(),
            &tiny_skia::PixmapPaint::default(),
            tiny_skia::Transform::identity(),
            None,
        );

        Ok(())
    }
}

/// Gutter kept between two neighbouring x tick labels, in ems of their size.
const X_TICK_LABEL_GAP_EM: f32 = 0.35;

/// How the x tick label row is oriented.
///
/// Ten region names under one axis run into each other at any font size a
/// figure would actually use, so the row has to be able to turn — and it turns
/// as a *row*: every label horizontal or every label rotated, never the mix a
/// per-label rule would produce.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum XTickRotation {
    /// Horizontal while the labels fit, a quarter turn when they stop fitting,
    /// and every k-th label when even a rotated row does not fit the margin.
    #[default]
    Auto,
    /// Always horizontal; colliding labels are thinned to every k-th.
    Horizontal,
    /// Always a quarter turn counter-clockwise.
    Vertical,
}

/// The horizontal range one x tick label row's ink may occupy, in pixels.
///
/// The first and last labels of a categorical axis are centred on slots that
/// sit close to the plot area's edges, so a label wider than the outer margin
/// runs off the canvas — a 35-character category name under the first bar of a
/// 400 px figure is not an edge case. A label that would fall outside is slid
/// back inside instead of being cut off.
///
/// The same range is applied when the row is *measured*, which is the point of
/// having a type for it: [`label_left`](Self::label_left) is the one formula
/// `clearing_stride` and `draw_x_tick_label_row` both ask, so sliding an
/// end label inwards can never create the overlap the stride was chosen to
/// avoid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct XTickRowBounds {
    /// Leftmost pixel the row's ink may touch.
    pub left: f32,
    /// One past the rightmost pixel the row's ink may touch.
    pub right: f32,
}

impl XTickRowBounds {
    /// No clamping at all — what a row measured against no particular canvas
    /// gets, and what [`XTickLabelPlan::default`] carries.
    pub const UNBOUNDED: Self = Self {
        left: f32::NEG_INFINITY,
        right: f32::INFINITY,
    };

    /// The full width of a `width` pixel canvas.
    pub fn canvas(width: f32) -> Self {
        Self {
            left: 0.0,
            right: width,
        }
    }

    /// The same range pulled `inset` pixels in from both edges.
    ///
    /// A label flush against the figure edge reads as clipped even when every
    /// glyph is present, so the row keeps the same gutter from the canvas that
    /// it keeps from its neighbours. An unbounded range stays unbounded.
    pub fn inset(&self, inset: f32) -> Self {
        if self.right - self.left <= inset * 2.0 {
            return *self;
        }
        Self {
            left: self.left + inset,
            right: self.right - inset,
        }
    }

    /// Where a label `extent` pixels wide and centred on `center` starts.
    ///
    /// Centred when it fits, slid inwards when it does not, and pinned to the
    /// left edge when it is wider than the whole range — a label too wide for
    /// the canvas has to lose one end, and losing the tail is the readable
    /// choice.
    pub fn label_left(&self, center: f32, extent: f32) -> f32 {
        (center - extent / 2.0)
            .min(self.right - extent)
            .max(self.left)
    }
}

impl Default for XTickRowBounds {
    fn default() -> Self {
        Self::UNBOUNDED
    }
}

/// What one x tick label row measures, before it is decided how to draw it.
///
/// Produced by [`SkiaRenderer::measure_x_tick_row`]; turned into an
/// [`XTickLabelPlan`] by [`XTickRowMetrics::plan`].
#[derive(Clone, Debug, PartialEq)]
pub struct XTickRowMetrics {
    /// Vertical pixels a horizontal row occupies.
    pub horizontal_extent: f32,
    /// The widest label, in pixels — and so the vertical pixels a rotated row
    /// occupies, since a quarter turn trades a label's width for its height.
    pub max_label_width: f32,
    /// Smallest stride at which a horizontal row stops overlapping.
    pub horizontal_stride: usize,
    /// Smallest stride at which a rotated row stops overlapping.
    pub rotated_stride: usize,
    /// The range the strides above were measured against, carried into the
    /// plan so the row that was measured is the row that is drawn.
    pub bounds: XTickRowBounds,
}

impl XTickRowMetrics {
    /// Whether the caller has to find out if a rotated row fits.
    ///
    /// Answering that costs a trial layout, so it is only worth asking when
    /// rotation is on the table at all.
    pub fn wants_rotation(&self, rotation: XTickRotation) -> bool {
        match rotation {
            XTickRotation::Horizontal => false,
            XTickRotation::Vertical => true,
            XTickRotation::Auto => self.horizontal_stride > 1,
        }
    }

    /// The plan this row is drawn with.
    ///
    /// `rotated_fits` answers "does a row [`max_label_width`] pixels tall fit
    /// the bottom margin the layout grants?" — the caller asks the layout,
    /// because only the layout knows what the margin config allows. An explicit
    /// [`XTickRotation::Vertical`] is honoured either way: a knob that silently
    /// does nothing is worse than one that costs a little room.
    ///
    /// [`max_label_width`]: Self::max_label_width
    pub fn plan(&self, rotation: XTickRotation, rotated_fits: bool) -> XTickLabelPlan {
        let rotated = match rotation {
            XTickRotation::Horizontal => false,
            XTickRotation::Vertical => true,
            XTickRotation::Auto => self.horizontal_stride > 1 && rotated_fits,
        };
        if rotated {
            XTickLabelPlan {
                rotated: true,
                stride: self.rotated_stride,
                extent: self.max_label_width,
                bounds: self.bounds,
            }
        } else {
            XTickLabelPlan {
                rotated: false,
                stride: self.horizontal_stride,
                extent: self.horizontal_extent,
                bounds: self.bounds,
            }
        }
    }
}

/// The resolved orientation and thinning of one x tick label row.
///
/// [`extent`](Self::extent) is the vertical room the row needs, and it is what
/// the bottom margin has to be reserved from *before* the plot area is
/// computed. Reserve it afterwards and the labels are clipped instead of
/// overlapping, which is not an improvement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct XTickLabelPlan {
    /// Whether the row is drawn a quarter turn counter-clockwise.
    pub rotated: bool,
    /// Only every `stride`-th label is drawn; `1` draws them all.
    pub stride: usize,
    /// Vertical pixels the row occupies.
    pub extent: f32,
    /// The horizontal range the row's ink is kept inside; see
    /// [`XTickRowBounds`].
    pub bounds: XTickRowBounds,
}

impl Default for XTickLabelPlan {
    /// Every label, horizontal, reserving nothing, clamped to nothing — what a
    /// row that was never measured is entitled to assume.
    fn default() -> Self {
        Self {
            rotated: false,
            stride: 1,
            extent: 0.0,
            bounds: XTickRowBounds::UNBOUNDED,
        }
    }
}

/// Smallest stride at which the drawn labels stop overlapping.
///
/// `extents` is each label's size along the axis: its width for a horizontal
/// row, its height for a rotated one. A label with no extent draws nothing and
/// so collides with nothing.
///
/// `bounds` is where the labels will actually be drawn, not where they would
/// like to be: an end label slid off the canvas edge is measured where it lands.
fn clearing_stride(centers: &[f32], extents: &[f32], gap: f32, bounds: XTickRowBounds) -> usize {
    let count = centers.len().min(extents.len());
    if count <= 1 {
        return 1;
    }
    (1..=count)
        .find(|&stride| stride_clears(centers, extents, gap, stride, bounds))
        .unwrap_or(count)
}

fn stride_clears(
    centers: &[f32],
    extents: &[f32],
    gap: f32,
    stride: usize,
    bounds: XTickRowBounds,
) -> bool {
    let mut previous_right: Option<f32> = None;
    for (&center, &extent) in centers.iter().zip(extents.iter()).step_by(stride) {
        if extent <= 0.0 || !center.is_finite() {
            continue;
        }
        let left = bounds.label_left(center, extent);
        let right = left + extent;
        if let Some(previous) = previous_right {
            if left < previous + gap {
                return false;
            }
            previous_right = Some(previous.max(right));
        } else {
            previous_right = Some(right);
        }
    }
    true
}

/// Draw one x tick label row onto any backend, following `plan`.
///
/// The raster and SVG backends share this body, so a figure cannot be labelled
/// one way as a PNG and another way as an SVG — the SVG twin used not to
/// measure its category labels at all. `top_y` is the top of the row in both
/// orientations.
///
/// The canvas is [`ColorbarCanvas`](crate::render::colorbar::ColorbarCanvas),
/// the crate's one backend-neutral text canvas, named for its first client.
pub(crate) fn draw_x_tick_label_row<C>(
    canvas: &mut C,
    labels: &[String],
    centers: &[f32],
    top_y: f32,
    size: f32,
    color: Color,
    plan: XTickLabelPlan,
) -> Result<()>
where
    C: crate::render::colorbar::ColorbarCanvas + ?Sized,
{
    let stride = plan.stride.max(1);
    // A thinned row writes only every stride-th name, and an unnamed slot holds
    // its place on the axis without writing under it.
    for (label, &center) in labels.iter().zip(centers.iter()).step_by(stride) {
        if label.is_empty() {
            continue;
        }
        let snippet = canvas.colorbar_label_snippet(label);
        let (width, height) = canvas.colorbar_measure_text(&snippet, size)?;
        if plan.rotated {
            // A quarter turn trades the label's width for its height, so the
            // row hangs from `top_y` and takes up only `height` sideways. The
            // rotated primitive centres its block on the x it is given.
            let left = plan.bounds.label_left(center, height);
            canvas.colorbar_text_rotated(
                &snippet,
                left + height / 2.0,
                top_y + width / 2.0,
                size,
                color,
            )?;
        } else {
            canvas.colorbar_text(
                &snippet,
                plan.bounds.label_left(center, width),
                top_y,
                size,
                color,
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;

/// The raster backend's colorbar primitives.
///
/// The geometry lives in [`crate::render::colorbar::draw_colorbar`]; this only
/// says how each primitive is put on a pixmap.
impl crate::render::colorbar::ColorbarCanvas for SkiaRenderer {
    fn colorbar_points_to_pixels(&self, points: f32) -> f32 {
        self.points_to_pixels(points)
    }

    fn colorbar_logical_pixels_to_pixels(&self, pixels: f32) -> f32 {
        self.logical_pixels_to_pixels(pixels)
    }

    fn colorbar_label_snippet<'a>(&self, text: &'a str) -> std::borrow::Cow<'a, str> {
        self.generated_label(text)
    }

    fn colorbar_measure_text(&self, text: &str, size: f32) -> Result<(f32, f32)> {
        self.measure_text(text, size)
    }

    fn colorbar_measure_ink_center_from_top(&self, text: &str, size: f32) -> Result<f32> {
        self.measure_text_ink_center_from_top(text, size)
    }

    fn colorbar_fill_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    ) -> Result<()> {
        self.draw_solid_rectangle(x, y, width, height, color)
    }

    fn colorbar_stroke_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
        stroke_width: f32,
    ) -> Result<()> {
        self.draw_rectangle_outline(x, y, width, height, color, stroke_width)
    }

    fn colorbar_line(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: Color,
        stroke_width: f32,
    ) -> Result<()> {
        self.draw_line(x1, y1, x2, y2, color, stroke_width, LineStyle::Solid)
    }

    fn colorbar_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color) -> Result<()> {
        self.draw_text(text, x, y, size, color)
    }

    fn colorbar_text_rotated(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
    ) -> Result<()> {
        self.draw_text_rotated(text, x, y, size, color)
    }
}

#[cfg(test)]
mod legend_patch_tests {
    use super::*;

    fn pixel_rgba(image: &Image, x: u32, y: u32) -> [u8; 4] {
        let idx = ((y * image.width + x) * 4) as usize;
        [
            image.pixels[idx],
            image.pixels[idx + 1],
            image.pixels[idx + 2],
            image.pixels[idx + 3],
        ]
    }

    /// Any pixel in the box that is clearly darker than the white panel/fill.
    fn has_dark_pixel(image: &Image, x0: u32, y0: u32, x1: u32, y1: u32) -> bool {
        (y0..y1).any(|y| {
            (x0..x1).any(|x| {
                let pixel = pixel_rgba(image, x, y);
                pixel[3] > 0 && pixel[0] < 200 && pixel[1] < 200 && pixel[2] < 200
            })
        })
    }

    fn handle_spacing(length: f32, height: f32) -> LegendSpacingPixels {
        LegendSpacingPixels {
            handle_length: length,
            handle_height: height,
            handle_text_pad: 10.0,
            label_spacing: 7.0,
            border_pad: 6.0,
            border_axes_pad: 10.0,
            column_spacing: 20.0,
        }
    }

    #[test]
    fn legacy_swatch_edge_contrasts_with_the_fill() {
        assert_eq!(
            legacy_legend_swatch_edge(Color::WHITE),
            LEGACY_LEGEND_SWATCH_EDGE_DARK
        );
        assert_eq!(
            legacy_legend_swatch_edge(Color::from_gray(250)),
            LEGACY_LEGEND_SWATCH_EDGE_DARK
        );
        assert_eq!(
            legacy_legend_swatch_edge(Color::BLACK),
            LEGACY_LEGEND_SWATCH_EDGE_LIGHT
        );
        // A transparent fill renders as the near-white panel, so it needs the
        // dark neutral even though its own channels are dark.
        assert_eq!(
            legacy_legend_swatch_edge(Color::from_rgba(0, 0, 0, 0)),
            LEGACY_LEGEND_SWATCH_EDGE_DARK
        );
    }

    #[test]
    fn legacy_legend_white_swatch_keeps_a_visible_contour() {
        let mut renderer = SkiaRenderer::new(400, 200, Theme::default()).unwrap();
        let plot_area = Rect::from_xywh(0.0, 0.0, 400.0, 200.0).unwrap();

        renderer
            .draw_legend(&[("white series".to_string(), Color::WHITE)], plot_area)
            .expect("legacy legend should render");

        let image = renderer.into_image();
        // Swatch occupies (250, 22) .. (262, 34); the stroke straddles the edge.
        // The panel frame (240/380, 15/45) and the label (x >= 270) stay outside.
        assert!(
            has_dark_pixel(&image, 246, 18, 267, 38),
            "a white swatch on the near-white panel must still have a contour"
        );
        // ... while the fill stays exactly the series colour.
        let interior = pixel_rgba(&image, 256, 28);
        assert_eq!(
            [interior[0], interior[1], interior[2]],
            [255, 255, 255],
            "the swatch fill must not be tinted"
        );
    }

    #[test]
    fn bar_and_histogram_handles_stroke_their_edge() {
        for item in [
            LegendItem::bar_with_edge("bars", Color::WHITE, Some((Color::BLACK, 1.5))),
            LegendItem::histogram_with_edge("hist", Color::WHITE, Some((Color::BLACK, 1.5))),
        ] {
            let mut renderer = SkiaRenderer::new(60, 40, Theme::default()).unwrap();
            renderer.pixmap.fill(Color::WHITE.to_tiny_skia_color());
            renderer
                .draw_legend_handle(&item, 10.0, 20.0, &handle_spacing(30.0, 14.0))
                .expect("patch handle should render");

            let image = renderer.into_image();
            assert!(
                has_dark_pixel(&image, 0, 0, 60, 40),
                "{:?} handle must stroke its edge so the key matches the plot",
                item.item_type
            );
            let interior = pixel_rgba(&image, 25, 20);
            assert_eq!(
                [interior[0], interior[1], interior[2]],
                [255, 255, 255],
                "the handle fill must stay exactly the series colour"
            );
        }
    }

    #[test]
    fn bar_handle_without_edge_stays_flat() {
        let mut renderer = SkiaRenderer::new(60, 40, Theme::default()).unwrap();
        renderer.pixmap.fill(Color::WHITE.to_tiny_skia_color());
        renderer
            .draw_legend_handle(
                &LegendItem::bar("bars", Color::WHITE),
                10.0,
                20.0,
                &handle_spacing(30.0, 14.0),
            )
            .expect("patch handle should render");

        let image = renderer.into_image();
        assert!(
            !has_dark_pixel(&image, 0, 0, 60, 40),
            "a patch with no configured edge must not grow an implicit one"
        );
    }

    /// The bug this layout exists to kill.
    ///
    /// The frame used to be sized as `label.len() * font_size * 0.6` — a
    /// **byte** count against a guessed advance. `"WWWWWWWWWW"` is ten bytes of
    /// glyphs each far wider than 0.6 em, so the label ran out of the frame;
    /// `"日本語"` is three glyphs in nine bytes, so the frame was drawn about
    /// three times too wide. Now the frame comes from the same measurement the
    /// renderer draws with, so the label has to fit inside it.
    #[test]
    fn legend_frame_fits_the_measured_label_not_its_byte_count() {
        let renderer = SkiaRenderer::new(400, 300, Theme::default()).unwrap();
        let legend = Legend {
            enabled: true,
            position: crate::core::LegendPosition::UpperLeft,
            ..Default::default()
        };
        let scaled = legend.scaled_for_render(renderer.render_scale());

        for label in ["WWWWWWWWWW", "日本語ラベル", "Ünïcödé", "iiii"] {
            let items = vec![LegendItem::line(label, Color::BLUE, LineStyle::Solid, 1.5)];
            let layout = renderer
                .legend_layout(
                    &items,
                    &scaled,
                    (0.0, 0.0, 400.0, 300.0),
                    LegendPlacement::default(),
                )
                .expect("legend layout");
            let text_width = renderer
                .measure_label_text(label, scaled.font_size)
                .expect("measure")
                .0;
            let entry = layout.entries[0];
            let inner_right = layout.x + layout.width - layout.spacing.border_pad;

            assert!(
                entry.label_x + text_width <= inner_right + 0.01,
                "{label:?} runs past the frame: label ends at {}, frame ends at {inner_right}",
                entry.label_x + text_width
            );
            // The reservation and the drawing are the same call, so the size the
            // figure layout reserves is the size the frame is drawn at.
            assert_eq!(
                layout.size(),
                renderer.measure_legend(&items, &legend).expect("reserve")
            );
        }
    }
}

/// The x tick label row: what it measures, what it decides, and what it draws.
#[cfg(test)]
mod x_tick_label_row_tests {
    use super::*;

    const REGIONS: [&str; 10] = [
        "North America",
        "South America",
        "Western Europe",
        "Eastern Europe",
        "Middle East",
        "North Africa",
        "Sub-Saharan Africa",
        "Central Asia",
        "South East Asia",
        "Australasia",
    ];

    fn renderer() -> SkiaRenderer {
        SkiaRenderer::new(600, 400, Theme::default()).expect("renderer")
    }

    fn labels(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_string()).collect()
    }

    /// Evenly spaced slot centres across `[left, right]`, the way a categorical
    /// axis lays its slots out.
    fn centers(count: usize, left: f32, right: f32) -> Vec<f32> {
        let span = right - left;
        (0..count)
            .map(|index| left + span * (index as f32 + 0.5) / count as f32)
            .collect()
    }

    fn dark_pixels(image: &crate::core::plot::Image) -> Vec<(u32, u32)> {
        let mut found = Vec::new();
        for y in 0..image.height {
            for x in 0..image.width {
                let index = ((y * image.width + x) * 4) as usize;
                if image.pixels[index] < 128 {
                    found.push((x, y));
                }
            }
        }
        found
    }

    /// Ten region names in one 500 px axis cannot be drawn horizontally without
    /// overlapping — the figure this row exists for. Turned a quarter turn they
    /// clear each other completely, because a label's height is a fraction of
    /// its width.
    #[test]
    fn ten_region_names_collide_horizontally_and_clear_when_rotated() {
        let renderer = renderer();
        let names = labels(&REGIONS);
        let metrics = renderer
            .measure_x_tick_row(
                &names,
                &centers(REGIONS.len(), 60.0, 560.0),
                12.0,
                XTickRowBounds::UNBOUNDED,
            )
            .expect("measure");

        assert!(
            metrics.horizontal_stride > 1,
            "ten region names should not fit horizontally, got stride {}",
            metrics.horizontal_stride
        );
        assert_eq!(metrics.rotated_stride, 1, "rotated names clear each other");
        assert!(
            metrics.max_label_width > metrics.horizontal_extent,
            "a rotated row is taller than a horizontal one for these labels"
        );
    }

    /// Short names in the same axis are left alone: no rotation, no thinning,
    /// and the row reserves exactly the height it draws at.
    #[test]
    fn short_names_stay_horizontal_and_complete() {
        let renderer = renderer();
        let names = labels(&["A", "B", "C", "D"]);
        let metrics = renderer
            .measure_x_tick_row(
                &names,
                &centers(4, 60.0, 560.0),
                12.0,
                XTickRowBounds::UNBOUNDED,
            )
            .expect("measure");

        assert_eq!(metrics.horizontal_stride, 1);
        let plan = metrics.plan(XTickRotation::Auto, true);
        assert!(!plan.rotated);
        assert_eq!(plan.stride, 1);
        assert_eq!(plan.extent, metrics.horizontal_extent);
    }

    /// An empty slot label writes nothing, so it cannot collide with anything —
    /// otherwise a nameless slot would thin its named neighbours out.
    #[test]
    fn unnamed_slots_do_not_collide() {
        let renderer = renderer();
        let mut names = labels(&[""; 10]);
        names[0] = "Western Europe".to_string();
        let metrics = renderer
            .measure_x_tick_row(
                &names,
                &centers(10, 60.0, 560.0),
                12.0,
                XTickRowBounds::UNBOUNDED,
            )
            .expect("measure");

        assert_eq!(metrics.horizontal_stride, 1);
    }

    /// The policy in one place: `Auto` rotates only when the margin can hold a
    /// rotated row and falls back to every k-th label when it cannot, while the
    /// explicit settings are honoured either way.
    #[test]
    fn auto_rotates_only_when_the_rotated_row_fits() {
        let renderer = renderer();
        let names = labels(&REGIONS);
        let metrics = renderer
            .measure_x_tick_row(
                &names,
                &centers(REGIONS.len(), 60.0, 560.0),
                12.0,
                XTickRowBounds::UNBOUNDED,
            )
            .expect("measure");

        let rotated = metrics.plan(XTickRotation::Auto, true);
        assert!(rotated.rotated);
        assert_eq!(rotated.stride, metrics.rotated_stride);
        assert_eq!(rotated.extent, metrics.max_label_width);

        let thinned = metrics.plan(XTickRotation::Auto, false);
        assert!(!thinned.rotated);
        assert_eq!(thinned.stride, metrics.horizontal_stride);
        assert!(thinned.stride > 1);
        assert_eq!(thinned.extent, metrics.horizontal_extent);

        assert!(!metrics.plan(XTickRotation::Horizontal, true).rotated);
        assert!(metrics.plan(XTickRotation::Vertical, false).rotated);
        assert!(!metrics.wants_rotation(XTickRotation::Horizontal));
        assert!(metrics.wants_rotation(XTickRotation::Vertical));
    }

    /// The plan is what gets drawn: a stride of two draws half the names, and a
    /// rotated row hangs down from the same baseline instead of spreading
    /// sideways.
    #[test]
    fn the_plan_is_what_the_row_draws() {
        let names = labels(&REGIONS);
        let centers = centers(REGIONS.len(), 60.0, 560.0);

        let ink = |plan: XTickLabelPlan| {
            let mut renderer = renderer();
            draw_x_tick_label_row(
                &mut renderer,
                &names,
                &centers,
                100.0,
                12.0,
                Color::BLACK,
                plan,
            )
            .expect("draw");
            dark_pixels(&renderer.into_image())
        };

        let complete = ink(XTickLabelPlan::default());
        let thinned = ink(XTickLabelPlan {
            stride: 2,
            ..XTickLabelPlan::default()
        });
        assert!(
            thinned.len() * 4 < complete.len() * 3,
            "every second label should be dropped: {} vs {}",
            thinned.len(),
            complete.len()
        );

        let rotated = ink(XTickLabelPlan {
            rotated: true,
            stride: 1,
            ..XTickLabelPlan::default()
        });
        let lowest = |pixels: &[(u32, u32)]| pixels.iter().map(|&(_, y)| y).max().unwrap_or(0);
        assert!(
            lowest(&rotated) > lowest(&complete) + 20,
            "a rotated row hangs below a horizontal one: {} vs {}",
            lowest(&rotated),
            lowest(&complete)
        );
        assert!(
            rotated.iter().all(|&(_, y)| y >= 99),
            "a rotated row hangs from the baseline, it does not rise above it"
        );
    }

    /// The end labels of a categorical axis are centred on slots that sit near
    /// the plot area's edges, so a name wider than the outer margin runs off the
    /// canvas. It is slid back inside instead of being cut in half.
    #[test]
    fn end_labels_are_slid_inside_the_canvas_rather_than_cut() {
        let names = labels(&["A very long first category name indeed", "B", "C"]);
        // A narrow figure's slots: the first centre sits well inside the widest
        // name, which is the whole point.
        let centers = centers(3, 20.0, 320.0);
        let renderer = renderer();
        let bounds = XTickRowBounds::canvas(renderer.width() as f32);
        let metrics = renderer
            .measure_x_tick_row(&names, &centers, 12.0, bounds)
            .expect("measure");
        let plan = metrics.plan(XTickRotation::Horizontal, false);

        let (width, _) = renderer
            .measure_text(&names[0], 12.0)
            .expect("measure first label");
        assert!(
            centers[0] - width / 2.0 < 0.0,
            "the figure under test must have an end label wider than its margin"
        );
        assert_eq!(
            plan.bounds.label_left(centers[0], width),
            plan.bounds.left,
            "an over-hanging first label starts at the row's left gutter, not \
             outside the canvas"
        );
        assert!(
            plan.bounds.left > 0.0 && plan.bounds.right < renderer.width() as f32,
            "the row keeps a gutter from the figure edge"
        );

        let mut renderer = renderer;
        draw_x_tick_label_row(
            &mut renderer,
            &names,
            &centers,
            100.0,
            12.0,
            Color::BLACK,
            plan,
        )
        .expect("draw");
        let image = renderer.into_image();
        let ink = dark_pixels(&image);
        assert!(!ink.is_empty(), "the row should draw something");
        assert!(
            ink.iter().all(|&(x, _)| x < image.width),
            "no label ink may fall outside the canvas"
        );
        assert!(
            ink.iter().any(|&(x, _)| x < 20),
            "the slid label should still sit hard against the left edge"
        );
    }

    /// Sliding an end label inwards moves it towards its neighbour, so the
    /// stride has to be measured where the labels land — otherwise the row that
    /// was measured as clearing is drawn overlapping.
    #[test]
    fn the_stride_is_measured_where_the_labels_land() {
        let renderer = renderer();
        // Two wide names whose first is pushed right by the canvas edge: unclamped
        // they clear each other, clamped they do not.
        let names = labels(&["Sub-Saharan Africa", "Sub-Saharan Africa"]);
        let (width, _) = renderer.measure_text(&names[0], 12.0).expect("measure");
        let gap = 12.0 * X_TICK_LABEL_GAP_EM;
        // Place the first label so that half of it hangs off the canvas, and the
        // second exactly one clearing width to its right.
        let first = width / 4.0;
        let centers = vec![first, first + width + gap + 1.0];

        let unclamped = renderer
            .measure_x_tick_row(&names, &centers, 12.0, XTickRowBounds::UNBOUNDED)
            .expect("measure");
        assert_eq!(
            unclamped.horizontal_stride, 1,
            "where they are asked to be drawn, the two names clear each other"
        );

        let clamped = renderer
            .measure_x_tick_row(&names, &centers, 12.0, XTickRowBounds::canvas(600.0))
            .expect("measure");
        assert_eq!(
            clamped.horizontal_stride, 2,
            "slid inside the canvas the first name runs into the second, so the \
             row has to thin"
        );
    }
}
