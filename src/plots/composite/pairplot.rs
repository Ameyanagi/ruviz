//! Pair plot implementations
//!
//! Provides pairwise scatter plot matrices.

use crate::render::Color;

/// Configuration for pair plot
#[derive(Debug, Clone)]
pub struct PairPlotConfig {
    /// Variable names
    pub vars: Vec<String>,
    /// Plot type for diagonal
    pub diag_kind: DiagKind,
    /// Plot type for off-diagonal
    pub off_diag_kind: OffDiagKind,
    /// Colors for different hue groups
    pub colors: Option<Vec<Color>>,
    /// Scatter point size
    pub scatter_size: f32,
    /// Scatter alpha
    pub scatter_alpha: f32,
    /// Number of histogram/KDE bins
    pub bins: usize,
    /// Show upper triangle
    pub upper: bool,
    /// Show lower triangle
    pub lower: bool,
    /// Show diagonal
    pub diag: bool,
}

/// Type of plot on diagonal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagKind {
    /// Histogram
    Hist,
    /// Kernel density estimate
    Kde,
    /// No plot on diagonal
    None,
}

/// Type of plot on off-diagonal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffDiagKind {
    /// Scatter plot
    Scatter,
    /// Regression plot
    Reg,
    /// KDE contour
    Kde,
}

impl Default for PairPlotConfig {
    fn default() -> Self {
        Self {
            vars: vec![],
            diag_kind: DiagKind::Hist,
            off_diag_kind: OffDiagKind::Scatter,
            colors: None,
            scatter_size: 3.0,
            scatter_alpha: 0.5,
            bins: 20,
            upper: true,
            lower: true,
            diag: true,
        }
    }
}

impl PairPlotConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set variable names
    pub fn vars(mut self, vars: Vec<String>) -> Self {
        self.vars = vars;
        self
    }

    /// Set diagonal plot type
    pub fn diag_kind(mut self, kind: DiagKind) -> Self {
        self.diag_kind = kind;
        self
    }

    /// Set off-diagonal plot type
    pub fn off_diag_kind(mut self, kind: OffDiagKind) -> Self {
        self.off_diag_kind = kind;
        self
    }

    /// Set colors
    pub fn colors(mut self, colors: Vec<Color>) -> Self {
        self.colors = Some(colors);
        self
    }

    /// Show only lower triangle
    pub fn lower_only(mut self) -> Self {
        self.upper = false;
        self.lower = true;
        self
    }

    /// Show only upper triangle
    pub fn upper_only(mut self) -> Self {
        self.upper = true;
        self.lower = false;
        self
    }
}

/// Cell position in pair plot grid
#[derive(Debug, Clone)]
pub struct PairPlotCell {
    /// Row index
    pub row: usize,
    /// Column index
    pub col: usize,
    /// Variable indices (x_var, y_var)
    pub var_indices: (usize, usize),
    /// Whether this is a diagonal cell
    pub is_diagonal: bool,
    /// Bounds (x, y, width, height) as fractions
    pub bounds: (f64, f64, f64, f64),
}

/// Computed pair plot layout
#[derive(Debug, Clone)]
pub struct PairPlotLayout {
    /// Number of variables
    pub n_vars: usize,
    /// Cells to render
    pub cells: Vec<PairPlotCell>,
    /// Gap between cells
    pub gap: f64,
}

/// Compute pair plot layout
///
/// # Arguments
/// * `n_vars` - Number of variables
/// * `config` - Pair plot configuration
///
/// # Returns
/// PairPlotLayout with cell positions
pub fn compute_pairplot_layout(n_vars: usize, config: &PairPlotConfig) -> PairPlotLayout {
    if n_vars == 0 {
        return PairPlotLayout {
            n_vars: 0,
            cells: vec![],
            gap: 0.02,
        };
    }

    let gap = 0.02;
    let cell_size = (1.0 - gap * (n_vars + 1) as f64) / n_vars as f64;
    let mut cells = Vec::new();

    for row in 0..n_vars {
        for col in 0..n_vars {
            let is_diagonal = row == col;
            let is_upper = col > row;
            let is_lower = col < row;

            // Check if this cell should be rendered
            let should_render = (is_diagonal && config.diag)
                || (is_upper && config.upper)
                || (is_lower && config.lower);

            if should_render {
                let x = gap + col as f64 * (cell_size + gap);
                let y = gap + (n_vars - 1 - row) as f64 * (cell_size + gap);

                cells.push(PairPlotCell {
                    row,
                    col,
                    var_indices: (col, row),
                    is_diagonal,
                    bounds: (x, y, cell_size, cell_size),
                });
            }
        }
    }

    PairPlotLayout { n_vars, cells, gap }
}

/// Get variable pair for a cell
pub fn cell_variable_names<'a>(cell: &PairPlotCell, var_names: &'a [String]) -> (&'a str, &'a str) {
    let default = "";
    let x_name = var_names
        .get(cell.var_indices.0)
        .map(|s| s.as_str())
        .unwrap_or(default);
    let y_name = var_names
        .get(cell.var_indices.1)
        .map(|s| s.as_str())
        .unwrap_or(default);
    (x_name, y_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pairplot_layout() {
        let config = PairPlotConfig::default();
        let layout = compute_pairplot_layout(3, &config);

        assert_eq!(layout.n_vars, 3);
        // 3x3 = 9 cells total
        assert_eq!(layout.cells.len(), 9);
    }

    #[test]
    fn test_pairplot_lower_only() {
        let config = PairPlotConfig::default().lower_only();
        let layout = compute_pairplot_layout(3, &config);

        // Lower triangle: 3 cells + diagonal: 3 cells = 6 cells
        assert_eq!(layout.cells.len(), 6);

        // No upper cells
        for cell in &layout.cells {
            assert!(cell.col <= cell.row);
        }
    }

    #[test]
    fn test_pairplot_cell_bounds() {
        let config = PairPlotConfig::default();
        let layout = compute_pairplot_layout(2, &config);

        // All cells should be within [0, 1]
        for cell in &layout.cells {
            assert!(cell.bounds.0 >= 0.0 && cell.bounds.0 <= 1.0);
            assert!(cell.bounds.1 >= 0.0 && cell.bounds.1 <= 1.0);
        }
    }

    #[test]
    fn test_cell_variable_names() {
        let var_names = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let cell = PairPlotCell {
            row: 1,
            col: 0,
            var_indices: (0, 1),
            is_diagonal: false,
            bounds: (0.0, 0.0, 0.5, 0.5),
        };

        let (x_name, y_name) = cell_variable_names(&cell, &var_names);
        assert_eq!(x_name, "A");
        assert_eq!(y_name, "B");
    }
}
