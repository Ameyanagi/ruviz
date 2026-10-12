//! Image-series construction and eager sampling of mathematical fields.

use super::*;
use crate::plots::{ImageConfig, ImageInterpolation, ImageOrigin};

impl Plot {
    /// Place straight-alpha RGBA pixels within the plot's data coordinates.
    ///
    /// The image is copied once and shared by subsequent plot clones. Source
    /// pixels are row-major; by default row zero is adjacent to the upper data
    /// edge and the outer extent is `(0, width, 0, height)`. Configure
    /// [`ImageConfig::extent`] and [`ImageConfig::origin`] explicitly for a
    /// mathematical field. Axis reversal does not change row identity.
    ///
    /// Images preserve their supplied colors and alpha; scalar normalization
    /// and colorbars belong to [`Plot::heatmap`]. Add line/scatter series after
    /// the image to draw overlays. The default interpolation is nearest.
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let pixels = Image::new(2, 1, vec![255, 0, 0, 255, 0, 0, 255, 128]);
    /// Plot::new()
    ///     .image(&pixels)
    ///     .extent(-1.0, 1.0, -0.5, 0.5)
    ///     .origin(ImageOrigin::Lower)
    ///     .data_aspect(1.0)
    ///     .save("image.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn image(self, image: &Image) -> PlotBuilder<ImageConfig> {
        self.image_with(image, ImageConfig::default())
    }

    /// Add an RGBA image using a previously configured placement policy.
    pub fn image_with(mut self, image: &Image, config: ImageConfig) -> PlotBuilder<ImageConfig> {
        self.layout.grid_style.visible = false;
        PlotBuilder::new(self, PlotInput::Image(Arc::new(image.clone())), config)
    }

    /// Sample a scalar function at the centers of a regular grid of cells.
    ///
    /// `width` is the column count, `height` the row count, and `extent` is
    /// `(xmin, xmax, ymin, ymax)` at the outer cell edges. Sampling is eager,
    /// row-major, and lower-origin. Each callback runs exactly once per source
    /// cell, independent of later rendering, output DPI, or zoom. The returned
    /// heatmap builder retains the physical extent and lower origin.
    ///
    /// Nonfinite samples follow heatmap masking rules. For logarithmic values,
    /// the caller must choose how to handle zeros and infinite poles.
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    /// Plot::new()
    ///     .heatmap_fn(200, 160, (-2.0, 2.0, -1.0, 1.0), |x, y| {
    ///         (-x * x - y * y).exp()
    ///     })
    ///     .cmap("viridis")
    ///     .save("field.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn heatmap_fn<F>(
        mut self,
        width: u32,
        height: u32,
        extent: (f64, f64, f64, f64),
        function: F,
    ) -> PlotBuilder<crate::plots::HeatmapConfig>
    where
        F: FnMut(f64, f64) -> f64,
    {
        let flat = match sample_field(width, height, extent, function) {
            Ok(flat) => flat,
            Err(error) => {
                self.set_pending_ingestion_error(error);
                Vec::new()
            }
        };
        let values: Vec<Vec<f64>> = if width == 0 {
            Vec::new()
        } else {
            flat.chunks(width as usize).map(<[f64]>::to_vec).collect()
        };
        self.heatmap_with(
            &values,
            crate::plots::HeatmapConfig::new()
                .extent(extent.0, extent.1, extent.2, extent.3)
                .origin(crate::plots::HeatmapOrigin::Lower),
        )
    }

    /// Sample caller-defined colors into a plot-integrated RGBA image.
    ///
    /// Uses the same eager, lower-origin cell-center convention as
    /// [`Plot::heatmap_fn`]. The function can map any computation, including a
    /// complex number's phase and magnitude, to [`Color`]. A transparent color
    /// masks a sample. No complex-number representation is imposed.
    pub fn image_fn<F>(
        mut self,
        width: u32,
        height: u32,
        extent: (f64, f64, f64, f64),
        mut function: F,
    ) -> PlotBuilder<ImageConfig>
    where
        F: FnMut(f64, f64) -> Color,
    {
        let colors = match sample_field(width, height, extent, |x, y| {
            let color = function(x, y);
            [color.r, color.g, color.b, color.a]
        }) {
            Ok(colors) => colors,
            Err(error) => {
                self.set_pending_ingestion_error(error);
                Vec::new()
            }
        };
        let pixels = colors.into_flattened();
        self.layout.grid_style.visible = false;
        PlotBuilder::new(
            self,
            PlotInput::Image(Arc::new(Image::new(width, height, pixels))),
            ImageConfig::new()
                .extent(extent.0, extent.1, extent.2, extent.3)
                .origin(ImageOrigin::Lower),
        )
    }
}

fn sample_field<T, F>(
    width: u32,
    height: u32,
    (xmin, xmax, ymin, ymax): (f64, f64, f64, f64),
    mut function: F,
) -> Result<Vec<T>>
where
    F: FnMut(f64, f64) -> T,
{
    if width == 0 || height == 0 {
        return Err(PlottingError::InvalidInput(
            "function sampling requires nonzero width and height".into(),
        ));
    }
    let dx = xmax - xmin;
    let dy = ymax - ymin;
    if ![xmin, xmax, ymin, ymax, dx, dy]
        .into_iter()
        .all(f64::is_finite)
        || dx <= 0.0
        || dy <= 0.0
    {
        return Err(PlottingError::InvalidInput(
            "sampling extent must be finite with xmin < xmax and ymin < ymax".into(),
        ));
    }
    let count = (width as usize)
        .checked_mul(height as usize)
        .ok_or(PlottingError::OutOfMemory)?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| PlottingError::OutOfMemory)?;
    for row in 0..height {
        let y = ymin + dy * ((f64::from(row) + 0.5) / f64::from(height));
        for col in 0..width {
            let x = xmin + dx * ((f64::from(col) + 0.5) / f64::from(width));
            values.push(function(x, y));
        }
    }
    Ok(values)
}

impl PlotBuilder<ImageConfig> {
    /// Set the outer image edges as `(xmin, xmax, ymin, ymax)`.
    ///
    /// Extents must be finite and increasing. Reverse displayed axes using
    /// `xlim`/`ylim` or axis inversion, independently of source row origin.
    pub fn extent(mut self, xmin: f64, xmax: f64, ymin: f64, ymax: f64) -> Self {
        self.config = self.config.extent(xmin, xmax, ymin, ymax);
        self
    }

    /// Set the physical Y edge adjacent to source row zero.
    pub fn origin(mut self, origin: ImageOrigin) -> Self {
        self.config = self.config.origin(origin);
        self
    }

    /// Set nearest-neighbor or premultiplied-alpha bilinear interpolation.
    pub fn interpolation(mut self, interpolation: ImageInterpolation) -> Self {
        self.config = self.config.interpolation(interpolation);
        self
    }

    pub(super) fn finalize(mut self) -> Plot {
        let PlotInput::Image(image) = self.input else {
            self.plot
                .set_pending_ingestion_error(PlottingError::InvalidInput(
                    "image series requires RGBA image data".into(),
                ));
            return self.plot;
        };
        match crate::plots::image::ImageData::new(image, self.config) {
            Ok(data) => self
                .plot
                .push_builder_series(super::series_api::series_from_style(
                    SeriesType::Image {
                        data: Arc::new(data),
                    },
                    self.style,
                )),
            Err(error) => {
                self.plot.set_pending_ingestion_error(error);
                self.plot
            }
        }
    }
}
