//! Images placed in data coordinates and sampled through the plot's axes.
//!
//! Unlike a scalar heatmap, an image supplies its own straight-alpha RGBA
//! colors. Its extent describes pixel **edges**; each source pixel occupies an
//! equally sized cell in data space, including when an axis is logarithmic.

use std::sync::Arc;

use crate::core::{Image, PlottingError, Result};
use crate::plots::traits::{PlotArea, PlotConfig, PlotData};

/// Which data-space edge contains the first row of an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageOrigin {
    /// Row zero touches the upper (`y_max`) edge of the extent.
    #[default]
    Upper,
    /// Row zero touches the lower (`y_min`) edge of the extent.
    Lower,
}

/// How image colors are sampled between source pixel centers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ImageInterpolation {
    /// Use the color of the source cell containing the sample.
    #[default]
    Nearest,
    /// Interpolate neighboring colors in premultiplied-alpha space.
    ///
    /// Samples between an outer pixel center and its edge repeat that pixel's
    /// color. Premultiplied interpolation prevents hidden RGB values in fully
    /// transparent pixels from creating colored fringes.
    Bilinear,
}

/// Placement and sampling settings for a plot-integrated RGBA image.
///
/// The default extent is `(0, width, 0, height)`, describing the outer pixel
/// edges. An explicit extent must have finite, strictly ascending edges and
/// finite spans. Reverse the plot's axis limits to reverse its displayed
/// direction; [`ImageOrigin`] selects which data-space edge contains row zero.
#[derive(Debug, Clone, Default)]
pub struct ImageConfig {
    /// Outer pixel edges as `(x_min, x_max, y_min, y_max)`.
    pub extent: Option<(f64, f64, f64, f64)>,
    /// Location of source row zero; defaults to [`ImageOrigin::Upper`].
    pub origin: ImageOrigin,
    /// Sampling policy; defaults to [`ImageInterpolation::Nearest`].
    pub interpolation: ImageInterpolation,
}

impl ImageConfig {
    /// Create image settings with the default extent, origin, and sampling.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the outer pixel edges in data coordinates.
    #[must_use]
    pub fn extent(mut self, x_min: f64, x_max: f64, y_min: f64, y_max: f64) -> Self {
        self.extent = Some((x_min, x_max, y_min, y_max));
        self
    }

    /// Choose which data-space edge contains source row zero.
    #[must_use]
    pub fn origin(mut self, origin: ImageOrigin) -> Self {
        self.origin = origin;
        self
    }

    /// Choose how colors are sampled between source pixel centers.
    #[must_use]
    pub fn interpolation(mut self, interpolation: ImageInterpolation) -> Self {
        self.interpolation = interpolation;
        self
    }
}

impl PlotConfig for ImageConfig {}

/// Validated image data, shared without copying its source pixel buffer.
#[derive(Debug, Clone)]
pub(crate) struct ImageData {
    pub(crate) image: Arc<Image>,
    pub(crate) config: ImageConfig,
    pub(crate) x_extent: (f64, f64),
    pub(crate) y_extent: (f64, f64),
}

impl ImageData {
    /// Validate the source and resolve its default data-space extent.
    pub(crate) fn new(image: Arc<Image>, config: ImageConfig) -> Result<Self> {
        if image.width == 0 || image.height == 0 {
            return Err(PlottingError::InvalidInput(
                "image dimensions must be nonzero".into(),
            ));
        }
        let expected_bytes = rgba_byte_count(image.width, image.height)?;
        if image.pixels.len() != expected_bytes {
            return Err(PlottingError::InvalidInput(format!(
                "image buffer must contain exactly {expected_bytes} RGBA bytes for {}x{} pixels, got {}",
                image.width,
                image.height,
                image.pixels.len()
            )));
        }

        let (x_min, x_max, y_min, y_max) =
            config
                .extent
                .unwrap_or((0.0, f64::from(image.width), 0.0, f64::from(image.height)));
        if !valid_extent(x_min, x_max) || !valid_extent(y_min, y_max) {
            return Err(PlottingError::InvalidInput(
                "image extent edges and spans must be finite, with x_min < x_max and y_min < y_max"
                    .into(),
            ));
        }

        Ok(Self {
            image,
            config,
            x_extent: (x_min, x_max),
            y_extent: (y_min, y_max),
        })
    }

    /// Sample the visible image into a cropped canvas-pixel layer.
    ///
    /// The returned origin is in canvas pixels. Each destination pixel center
    /// is inverse-mapped through the plot area's axis scales before sampling
    /// the source, so reversed and nonlinear axes use the same projection as
    /// other plot elements. `alpha` multiplies the source alpha channel.
    pub(crate) fn render_layer(
        &self,
        area: &PlotArea,
        alpha: f32,
    ) -> Result<Option<(Image, u32, u32)>> {
        if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
            return Err(PlottingError::InvalidAlpha(alpha));
        }
        if alpha == 0.0 {
            return Ok(None);
        }
        validate_plot_area(area)?;

        let visible_x = intersect(
            self.x_extent,
            (area.x_min.min(area.x_max), area.x_min.max(area.x_max)),
        );
        let visible_y = intersect(
            self.y_extent,
            (area.y_min.min(area.y_max), area.y_min.max(area.y_max)),
        );
        let (Some((x_min, x_max)), Some((y_min, y_max))) = (visible_x, visible_y) else {
            return Ok(None);
        };

        let (screen_x0, screen_y0) = area.data_to_screen(x_min, y_min);
        let (screen_x1, screen_y1) = area.data_to_screen(x_max, y_max);
        if ![screen_x0, screen_y0, screen_x1, screen_y1]
            .iter()
            .all(|edge| edge.is_finite())
        {
            return Err(PlottingError::InvalidInput(
                "image extent cannot be projected through the plot axes".into(),
            ));
        }

        let left = screen_x0.min(screen_x1).max(area.x).max(0.0);
        let right = screen_x0.max(screen_x1).min(area.x + area.width);
        let top = screen_y0.min(screen_y1).max(area.y).max(0.0);
        let bottom = screen_y0.max(screen_y1).min(area.y + area.height);
        if left >= right || top >= bottom {
            return Ok(None);
        }

        // Half-open bounds include precisely those canvas pixel centers that
        // lie within the projected extent, without painting over the axes.
        let x_start = pixel_boundary(left)?;
        let x_end = pixel_boundary(right)?;
        let y_start = pixel_boundary(top)?;
        let y_end = pixel_boundary(bottom)?;
        let width = x_end.saturating_sub(x_start);
        let height = y_end.saturating_sub(y_start);
        if width == 0 || height == 0 {
            return Ok(None);
        }

        let byte_count = rgba_byte_count(width, height)?;
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(byte_count)
            .map_err(|_| PlottingError::OutOfMemory)?;
        pixels.resize(byte_count, 0);

        for (row_index, row) in pixels.chunks_exact_mut(width as usize * 4).enumerate() {
            let screen_y = (f64::from(y_start) + row_index as f64 + 0.5) as f32;
            for (column_index, pixel) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                let screen_x = (f64::from(x_start) + column_index as f64 + 0.5) as f32;
                let (data_x, data_y) = area.screen_to_data(screen_x, screen_y);
                if let Some(color) = self.sample(data_x, data_y, alpha) {
                    pixel.copy_from_slice(&color);
                }
            }
        }

        Ok(Some((
            Image::from_straight_rgba(width, height, pixels),
            x_start,
            y_start,
        )))
    }

    /// Sample a data-space point, returning `None` outside the image extent.
    fn sample(&self, data_x: f64, data_y: f64, alpha: f32) -> Option<[u8; 4]> {
        let u = (data_x - self.x_extent.0) / (self.x_extent.1 - self.x_extent.0);
        let v = (data_y - self.y_extent.0) / (self.y_extent.1 - self.y_extent.0);
        if !u.is_finite()
            || !v.is_finite()
            || !(0.0..=1.0).contains(&u)
            || !(0.0..=1.0).contains(&v)
        {
            return None;
        }
        let source_v = match self.config.origin {
            ImageOrigin::Upper => 1.0 - v,
            ImageOrigin::Lower => v,
        };
        match self.config.interpolation {
            ImageInterpolation::Nearest => {
                let x = (u * f64::from(self.image.width)) as u32;
                let y = (source_v * f64::from(self.image.height)) as u32;
                let mut color =
                    self.pixel(x.min(self.image.width - 1), y.min(self.image.height - 1));
                color[3] = (f32::from(color[3]) * alpha).round() as u8;
                Some(color)
            }
            ImageInterpolation::Bilinear => Some(self.sample_bilinear(u, source_v, alpha)),
        }
    }

    fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        // Construction validated the full buffer size; all callers clamp
        // indices to the validated nonzero dimensions.
        let offset = (y as usize * self.image.width as usize + x as usize) * 4;
        let pixel = &self.image.pixels[offset..offset + 4];
        [pixel[0], pixel[1], pixel[2], pixel[3]]
    }

    fn sample_bilinear(&self, u: f64, v: f64, alpha: f32) -> [u8; 4] {
        let x = (u * f64::from(self.image.width) - 0.5).clamp(0.0, f64::from(self.image.width - 1));
        let y =
            (v * f64::from(self.image.height) - 0.5).clamp(0.0, f64::from(self.image.height - 1));
        let x0 = x.floor() as u32;
        let y0 = y.floor() as u32;
        let x1 = x0.saturating_add(1).min(self.image.width - 1);
        let y1 = y0.saturating_add(1).min(self.image.height - 1);
        let wx = x - f64::from(x0);
        let wy = y - f64::from(y0);
        let samples = [
            (self.pixel(x0, y0), (1.0 - wx) * (1.0 - wy)),
            (self.pixel(x1, y0), wx * (1.0 - wy)),
            (self.pixel(x0, y1), (1.0 - wx) * wy),
            (self.pixel(x1, y1), wx * wy),
        ];

        let mut premultiplied = [0.0; 3];
        let mut source_alpha = 0.0;
        for (color, weight) in samples {
            let weighted_alpha = f64::from(color[3]) / 255.0 * weight;
            source_alpha += weighted_alpha;
            for channel in 0..3 {
                premultiplied[channel] += f64::from(color[channel]) * weighted_alpha;
            }
        }
        if source_alpha == 0.0 {
            return [0; 4];
        }

        [
            (premultiplied[0] / source_alpha).round().clamp(0.0, 255.0) as u8,
            (premultiplied[1] / source_alpha).round().clamp(0.0, 255.0) as u8,
            (premultiplied[2] / source_alpha).round().clamp(0.0, 255.0) as u8,
            (source_alpha * f64::from(alpha) * 255.0)
                .round()
                .clamp(0.0, 255.0) as u8,
        ]
    }
}

impl PlotData for ImageData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        (self.x_extent, self.y_extent)
    }

    fn is_empty(&self) -> bool {
        // ImageData can only be constructed from a nonempty, valid source.
        false
    }
}

fn valid_extent(min: f64, max: f64) -> bool {
    min.is_finite() && max.is_finite() && min < max && (max - min).is_finite()
}

fn rgba_byte_count(width: u32, height: u32) -> Result<usize> {
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|count| count.checked_mul(4))
        .ok_or_else(|| PlottingError::InvalidInput("image RGBA buffer size overflows usize".into()))
}

fn intersect(first: (f64, f64), second: (f64, f64)) -> Option<(f64, f64)> {
    let min = first.0.max(second.0);
    let max = first.1.min(second.1);
    (min < max).then_some((min, max))
}

fn pixel_boundary(screen_edge: f32) -> Result<u32> {
    let boundary = (f64::from(screen_edge) - 0.5).ceil().max(0.0);
    if !boundary.is_finite() || boundary > f64::from(u32::MAX) {
        return Err(PlottingError::InvalidInput(
            "projected image exceeds the supported canvas coordinates".into(),
        ));
    }
    Ok(boundary as u32)
}

fn validate_plot_area(area: &PlotArea) -> Result<()> {
    let geometry_valid = [area.x, area.y, area.width, area.height]
        .iter()
        .all(|value| value.is_finite())
        && area.width > 0.0
        && area.height > 0.0
        && (area.x + area.width).is_finite()
        && (area.y + area.height).is_finite();
    let axes_valid = area.x_min != area.x_max
        && area.y_min != area.y_max
        && area.x_scale.is_valid_value(area.x_min)
        && area.x_scale.is_valid_value(area.x_max)
        && area.y_scale.is_valid_value(area.y_min)
        && area.y_scale.is_valid_value(area.y_max);
    if !geometry_valid || !axes_valid {
        return Err(PlottingError::InvalidInput(
            "image rendering requires a finite nonempty plot area and valid axis limits".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::AxisScale;

    const RED: [u8; 4] = [255, 0, 0, 255];
    const GREEN: [u8; 4] = [0, 255, 0, 255];
    const BLUE: [u8; 4] = [0, 0, 255, 255];
    const WHITE: [u8; 4] = [255, 255, 255, 255];

    fn source(width: u32, height: u32, pixels: &[[u8; 4]]) -> Arc<Image> {
        Arc::new(Image::new(width, height, pixels.concat()))
    }

    fn render(data: &ImageData, area: &PlotArea) -> (Image, u32, u32) {
        data.render_layer(area, 1.0).unwrap().unwrap()
    }

    #[test]
    fn defaults_preserve_rows_and_pixel_edges() {
        let data =
            ImageData::new(source(2, 2, &[RED, GREEN, BLUE, WHITE]), ImageConfig::new()).unwrap();
        assert_eq!(data.data_bounds(), ((0.0, 2.0), (0.0, 2.0)));
        assert!(!data.is_empty());
        let area = PlotArea::new(0.0, 0.0, 2.0, 2.0, 0.0, 2.0, 0.0, 2.0);
        let (layer, x, y) = render(&data, &area);
        assert_eq!((x, y, layer.width, layer.height), (0, 0, 2, 2));
        assert_eq!(layer.pixels, [RED, GREEN, BLUE, WHITE].concat());
    }

    #[test]
    fn lower_origin_places_row_zero_at_lower_data_edge() {
        let data = ImageData::new(
            source(2, 2, &[RED, GREEN, BLUE, WHITE]),
            ImageConfig::new().origin(ImageOrigin::Lower),
        )
        .unwrap();
        let area = PlotArea::new(0.0, 0.0, 2.0, 2.0, 0.0, 2.0, 0.0, 2.0);
        assert_eq!(
            render(&data, &area).0.pixels,
            [BLUE, WHITE, RED, GREEN].concat()
        );
    }

    #[test]
    fn reversing_axes_reverses_the_image_without_changing_its_origin() {
        let data =
            ImageData::new(source(2, 2, &[RED, GREEN, BLUE, WHITE]), ImageConfig::new()).unwrap();
        let area = PlotArea::new(0.0, 0.0, 2.0, 2.0, 2.0, 0.0, 2.0, 0.0);
        assert_eq!(
            render(&data, &area).0.pixels,
            [WHITE, BLUE, GREEN, RED].concat()
        );
    }

    #[test]
    fn explicit_extent_places_and_crops_the_canvas_layer() {
        let data = ImageData::new(
            source(2, 1, &[RED, BLUE]),
            ImageConfig::new().extent(2.0, 4.0, 0.0, 1.0),
        )
        .unwrap();
        let area = PlotArea::new(10.0, 20.0, 8.0, 2.0, 0.0, 4.0, 0.0, 1.0);
        let (layer, x, y) = render(&data, &area);
        assert_eq!((x, y, layer.width, layer.height), (14, 20, 4, 2));
        assert_eq!(
            layer.pixels,
            [RED, RED, BLUE, BLUE, RED, RED, BLUE, BLUE].concat()
        );
        let outside = PlotArea::new(0.0, 0.0, 8.0, 2.0, 5.0, 9.0, 0.0, 1.0);
        assert!(data.render_layer(&outside, 1.0).unwrap().is_none());
    }

    #[test]
    fn clipped_axis_limits_preserve_source_cell_locations() {
        let data =
            ImageData::new(source(4, 1, &[RED, GREEN, BLUE, WHITE]), ImageConfig::new()).unwrap();
        let area = PlotArea::new(0.0, 0.0, 2.0, 1.0, 1.0, 3.0, 0.0, 1.0);
        assert_eq!(render(&data, &area).0.pixels, [GREEN, BLUE].concat());
    }

    #[test]
    fn fractional_plot_edges_sample_only_contained_pixel_centers() {
        let data = ImageData::new(source(1, 1, &[RED]), ImageConfig::new()).unwrap();
        let area = PlotArea::new(0.8, 1.8, 2.0, 2.0, 0.0, 1.0, 0.0, 1.0);
        let (layer, x, y) = render(&data, &area);
        assert_eq!((x, y, layer.width, layer.height), (1, 2, 2, 2));
        assert_eq!(layer.pixels, [RED; 4].concat());
    }

    #[test]
    fn log_axes_inverse_map_destination_pixel_centers() {
        let data = ImageData::new(
            source(4, 1, &[RED, GREEN, BLUE, WHITE]),
            ImageConfig::new().extent(1.0, 101.0, 0.0, 1.0),
        )
        .unwrap();
        let area = PlotArea::new(0.0, 0.0, 4.0, 1.0, 1.0, 101.0, 0.0, 1.0)
            .with_scales(AxisScale::Log, AxisScale::Linear);
        assert_eq!(
            render(&data, &area).0.pixels,
            [RED, RED, RED, BLUE].concat()
        );
        let reversed = PlotArea {
            x_min: 101.0,
            x_max: 1.0,
            ..area
        };
        assert_eq!(
            render(&data, &reversed).0.pixels,
            [BLUE, RED, RED, RED].concat()
        );
    }

    #[test]
    fn image_crossing_zero_is_clipped_by_positive_log_limits() {
        let data = ImageData::new(
            source(2, 1, &[RED, BLUE]),
            ImageConfig::new().extent(-1.0, 1.0, 0.0, 1.0),
        )
        .unwrap();
        let area = PlotArea::new(0.0, 0.0, 2.0, 1.0, 0.01, 1.0, 0.0, 1.0)
            .with_scales(AxisScale::Log, AxisScale::Linear);
        assert_eq!(render(&data, &area).0.pixels, [BLUE, BLUE].concat());
    }

    #[test]
    fn series_alpha_multiplies_source_alpha() {
        let data =
            ImageData::new(source(1, 1, &[[100, 150, 200, 128]]), ImageConfig::new()).unwrap();
        let area = PlotArea::new(0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 1.0);
        let (layer, _, _) = data.render_layer(&area, 0.5).unwrap().unwrap();
        assert_eq!(layer.pixels, [100, 150, 200, 64]);
        assert!(data.render_layer(&area, 0.0).unwrap().is_none());
        for alpha in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
            assert!(matches!(
                data.render_layer(&area, alpha),
                Err(PlottingError::InvalidAlpha(_))
            ));
        }
    }

    #[test]
    fn bilinear_smooths_centers_and_repeats_edge_colors() {
        let data = ImageData::new(
            source(2, 1, &[RED, BLUE]),
            ImageConfig::new().interpolation(ImageInterpolation::Bilinear),
        )
        .unwrap();
        let area = PlotArea::new(0.0, 0.0, 3.0, 1.0, 0.0, 2.0, 0.0, 1.0);
        assert_eq!(
            render(&data, &area).0.pixels,
            [RED, [128, 0, 128, 255], BLUE].concat()
        );
    }

    #[test]
    fn bilinear_interpolates_premultiplied_alpha_without_color_halos() {
        let data = ImageData::new(
            source(2, 1, &[RED, [0, 0, 255, 0]]),
            ImageConfig::new().interpolation(ImageInterpolation::Bilinear),
        )
        .unwrap();
        let area = PlotArea::new(0.0, 0.0, 3.0, 1.0, 0.0, 2.0, 0.0, 1.0);
        assert_eq!(
            render(&data, &area).0.pixels,
            [RED, [255, 0, 0, 128], [0; 4]].concat()
        );
        let (layer, _, _) = data.render_layer(&area, 0.5).unwrap().unwrap();
        assert_eq!(&layer.pixels[4..8], &[255, 0, 0, 64]);
    }

    #[test]
    fn bilinear_uses_both_dimensions_and_origin() {
        let data = ImageData::new(
            source(2, 2, &[RED, GREEN, BLUE, WHITE]),
            ImageConfig::new()
                .origin(ImageOrigin::Lower)
                .interpolation(ImageInterpolation::Bilinear),
        )
        .unwrap();
        assert_eq!(data.sample(1.0, 1.0, 1.0), Some([128, 128, 128, 255]));
        assert_eq!(data.sample(0.5, 0.5, 1.0), Some(RED));
        assert_eq!(data.sample(0.5, 1.5, 1.0), Some(BLUE));
        assert_eq!(data.sample(-0.1, 0.5, 1.0), None);
    }

    #[test]
    fn invalid_sources_are_rejected_without_allocating() {
        for image in [
            Image::new(0, 1, vec![]),
            Image::new(1, 0, vec![]),
            Image::new(1, 1, vec![0; 3]),
            Image::new(1, 1, vec![0; 5]),
            Image::new(u32::MAX, u32::MAX, vec![]),
        ] {
            assert!(ImageData::new(Arc::new(image), ImageConfig::new()).is_err());
        }
    }

    #[test]
    fn invalid_extents_are_rejected() {
        for extent in [
            (1.0, 1.0, 0.0, 1.0),
            (1.0, 0.0, 0.0, 1.0),
            (0.0, 1.0, 1.0, 0.0),
            (f64::NAN, 1.0, 0.0, 1.0),
            (0.0, f64::INFINITY, 0.0, 1.0),
            (0.0, 1.0, f64::NEG_INFINITY, 1.0),
            (-f64::MAX, f64::MAX, 0.0, 1.0),
        ] {
            let config = ImageConfig {
                extent: Some(extent),
                ..ImageConfig::default()
            };
            assert!(ImageData::new(source(1, 1, &[RED]), config).is_err());
        }
    }

    #[test]
    fn invalid_plot_areas_return_errors() {
        let data = ImageData::new(source(1, 1, &[RED]), ImageConfig::new()).unwrap();
        let area = PlotArea::new(0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 1.0);
        for invalid in [
            PlotArea { width: 0.0, ..area },
            PlotArea {
                x: f32::NAN,
                ..area
            },
            PlotArea { y_max: 0.0, ..area },
            area.with_scales(AxisScale::Log, AxisScale::Linear),
        ] {
            assert!(data.render_layer(&invalid, 1.0).is_err());
        }
    }
}
