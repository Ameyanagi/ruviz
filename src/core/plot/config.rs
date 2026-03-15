//! Plot configuration types

/// Backend types for rendering
#[allow(clippy::upper_case_acronyms)] // GPU is the standard industry acronym
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    /// Default Skia backend (CPU-based, good for <1K points)
    Skia,
    /// Parallel multi-threaded backend (good for 1K-100K points)
    Parallel,
    /// GPU-accelerated backend (good for >100K points)
    GPU,
    /// DataShader aggregation backend (good for >1M points)
    DataShader,
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
