//! Plot configuration types

/// Backend types that may be requested for raster rendering.
#[allow(clippy::upper_case_acronyms)] // GPU is the standard industry acronym
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    /// Reference CPU raster backend.
    Skia,
    /// Parallel CPU backend preference.
    Parallel,
    /// GPU backend preference.
    GPU,
    /// Density-aggregation backend for compatible scatter PNG output.
    DataShader,
}

impl BackendType {
    /// Stable lowercase name for diagnostics, benchmark output, and logs.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Skia => "skia",
            Self::Parallel => "parallel",
            Self::GPU => "gpu",
            Self::DataShader => "datashader",
        }
    }
}

/// Public raster operation whose backend is being resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BackendOperation {
    /// In-memory raster output, including prepared plots, subplot frames, and
    /// PNG bytes encoded from those images.
    RasterImage,
    /// The native `Plot::render_png_bytes` and `Plot::save` routing policy.
    Png,
    /// Interactive base-layer rasterization.
    Interactive,
}

/// Why a requested raster backend resolved to Skia.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BackendFallbackReason {
    /// The crate feature required by the requested backend is disabled.
    FeatureDisabled,
    /// No parity-approved public execution path exists for this operation.
    UnsupportedOperation,
    /// The requested backend is unavailable on the compilation target.
    UnsupportedTarget,
    /// The plot contains no series to render with the requested backend.
    EmptyPlot,
    /// The plot mixes Cartesian and non-Cartesian coordinate systems.
    MixedCoordinateSystems,
    /// At least one series type is unsupported by the requested backend.
    UnsupportedSeries,
    /// The requested backend does not support the configured axis scales.
    UnsupportedAxisScale,
    /// The requested backend cannot execute descending manual axis limits.
    ReversedAxisLimits,
}

/// Deterministic backend decision for a specific public raster operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendResolution {
    requested_backend: Option<BackendType>,
    actual_backend: BackendType,
    fallback_reason: Option<BackendFallbackReason>,
}

impl BackendResolution {
    pub(crate) const fn new(
        requested_backend: Option<BackendType>,
        actual_backend: BackendType,
        fallback_reason: Option<BackendFallbackReason>,
    ) -> Self {
        Self {
            requested_backend,
            actual_backend,
            fallback_reason,
        }
    }

    /// Explicit or auto-selected backend stored on the plot, if any.
    pub const fn requested_backend(self) -> Option<BackendType> {
        self.requested_backend
    }

    /// Backend that will execute for the selected operation.
    pub const fn actual_backend(self) -> BackendType {
        self.actual_backend
    }

    /// Reason the requested backend resolved to Skia, if it did.
    pub const fn fallback_reason(self) -> Option<BackendFallbackReason> {
        self.fallback_reason
    }

    /// Whether the stored backend choice could not execute.
    pub const fn used_fallback(self) -> bool {
        self.fallback_reason.is_some()
    }
}

/// Tick direction configuration
#[derive(Clone, Debug, PartialEq, Default)]
pub enum TickDirection {
    /// Ticks point inward into the plot area (default)
    #[default]
    Inside,
    /// Ticks point outward from the plot area
    Outside,
    /// Ticks straddle the plot border, extending both in and out
    InOut,
}

/// Tick side visibility configuration
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickSides {
    /// Show ticks on the top border of the plot area.
    pub top: bool,
    /// Show ticks on the bottom border of the plot area.
    pub bottom: bool,
    /// Show ticks on the left border of the plot area.
    pub left: bool,
    /// Show ticks on the right border of the plot area.
    pub right: bool,
}

impl TickSides {
    /// Show ticks on no sides.
    pub const fn none() -> Self {
        Self {
            top: false,
            bottom: false,
            left: false,
            right: false,
        }
    }

    /// Show ticks on all four sides.
    pub const fn all() -> Self {
        Self {
            top: true,
            bottom: true,
            left: true,
            right: true,
        }
    }

    /// Show ticks only on the bottom and left sides.
    pub const fn bottom_left() -> Self {
        Self {
            top: false,
            bottom: true,
            left: true,
            right: false,
        }
    }

    /// Return a copy with top ticks enabled or disabled.
    pub const fn with_top(mut self, enabled: bool) -> Self {
        self.top = enabled;
        self
    }

    /// Return a copy with bottom ticks enabled or disabled.
    pub const fn with_bottom(mut self, enabled: bool) -> Self {
        self.bottom = enabled;
        self
    }

    /// Return a copy with left ticks enabled or disabled.
    pub const fn with_left(mut self, enabled: bool) -> Self {
        self.left = enabled;
        self
    }

    /// Return a copy with right ticks enabled or disabled.
    pub const fn with_right(mut self, enabled: bool) -> Self {
        self.right = enabled;
        self
    }
}

impl Default for TickSides {
    fn default() -> Self {
        Self::all()
    }
}

/// Grid display mode for major and minor ticks
#[derive(Clone, Debug, PartialEq, Default)]
pub enum GridMode {
    /// Show grid lines only at major ticks
    #[default]
    MajorOnly,
    /// Show grid lines only at minor ticks
    MinorOnly,
    /// Show grid lines at both major and minor ticks
    Both,
}
