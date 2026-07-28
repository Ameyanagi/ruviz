//! Pair plots: an n×n matrix of bivariate panels with a distribution on the
//! diagonal.
//!
//! [`pairplot`] assembles the figure; [`compute_pairplot_layout`] is the
//! geometry it uses, exposed on its own for anyone placing the cells by hand
//! with [`SubplotFigure::add_axes`](crate::core::subplot::SubplotFigure::add_axes).

use crate::core::subplot::{FigureRect, SubplotFigure, figure};
use crate::core::{IntoPlot, Plot, PlottingError, Result};
use crate::data::NumericData1D;
use crate::render::Color;

use super::jointplot::{Marginal, MarginalAxis, padded_range, panel, resolved_color};

/// The largest matrix a pair plot will assemble.
///
/// The same ceiling [`GridSpec`](crate::core::GridSpec) puts on a subplot grid:
/// beyond it the cells are too small to read and the render cost is quadratic.
pub const MAX_PAIRPLOT_VARIABLES: usize = 10;

/// Configuration for pair plot
#[derive(Debug, Clone)]
pub struct PairPlotConfig {
    /// Variable names
    ///
    /// Used to label the bottom row and left column. An empty list leaves the
    /// cells unlabelled.
    pub vars: Vec<String>,
    /// Plot type for diagonal
    pub diag_kind: DiagKind,
    /// Plot type for off-diagonal
    ///
    /// [`OffDiagKind::Scatter`] is drawable; `Reg` and `Kde` have no renderer
    /// behind them yet and [`pairplot_with`] reports that rather than quietly
    /// substituting a scatter.
    pub off_diag_kind: OffDiagKind,
    /// Colors for different hue groups
    ///
    /// Hue grouping is not implemented; the first entry, if any, colours the
    /// whole matrix.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiagKind {
    /// Histogram
    #[default]
    Hist,
    /// Kernel density estimate
    Kde,
    /// No plot on diagonal
    None,
}

/// Type of plot on off-diagonal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OffDiagKind {
    /// Scatter plot
    #[default]
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

    /// Set scatter point size
    pub fn scatter_size(mut self, size: f32) -> Self {
        self.scatter_size = size;
        self
    }

    /// Set scatter opacity
    pub fn scatter_alpha(mut self, alpha: f32) -> Self {
        self.scatter_alpha = alpha;
        self
    }

    /// Set the number of bins on the diagonal
    pub fn bins(mut self, bins: usize) -> Self {
        self.bins = bins.max(2);
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

    /// The one colour the matrix is drawn in.
    fn color(&self) -> Color {
        resolved_color(
            self.colors
                .as_ref()
                .and_then(|colors| colors.first())
                .copied(),
        )
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
    /// Bounds in figure-relative coordinates, ready for
    /// [`SubplotFigure::add_axes`](crate::core::subplot::SubplotFigure::add_axes)
    pub bounds: FigureRect,
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
/// PairPlotLayout with cell positions, in figure-relative coordinates whose
/// origin is the lower-left corner: row 0 is the **top** row.
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
                    bounds: FigureRect::new(x, y, cell_size, cell_size),
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

/// Draw a pair plot: every variable scattered against every other, with its
/// own distribution on the diagonal.
///
/// `columns` is one slice per variable, all the same length — the shape
/// seaborn's `pairplot(DataFrame)` gets from its frame.
///
/// Returns the same [`SubplotFigure`] that
/// [`subplots`](crate::core::subplot::subplots) returns, so the figure-level
/// chain — `.suptitle(..)`, `.theme(..)`, `.save(..)` — is the one you already
/// know.
///
/// Every cell in a column shares that column's x limits and every cell in a
/// row shares that row's y limits, so the matrix reads as one picture rather
/// than n² independent plots.
///
/// # Example
///
/// ```rust,no_run
/// use ruviz::plots::composite::pairplot;
///
/// let sepal: Vec<f64> = (0..60).map(|i| 4.0 + (i % 13) as f64 * 0.2).collect();
/// let petal: Vec<f64> = (0..60).map(|i| 1.0 + (i % 7) as f64 * 0.3).collect();
/// let width: Vec<f64> = (0..60).map(|i| 2.0 + (i % 5) as f64 * 0.1).collect();
///
/// pairplot(&[sepal, petal, width], 900, 900)?
///     .suptitle("Iris measurements")
///     .save("pairplot.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn pairplot<D>(columns: &[D], width: u32, height: u32) -> Result<SubplotFigure>
where
    D: NumericData1D,
{
    pairplot_with(columns, width, height, PairPlotConfig::default())
}

/// Draw a pair plot with an explicit [`PairPlotConfig`].
///
/// Spelled like the rest of the crate's `_with` entry points: the bare name
/// takes the defaults, the `_with` name takes the config.
///
/// # Errors
///
/// - [`PlottingError::EmptyDataSet`] when there are no variables or no rows.
/// - [`PlottingError::DataLengthMismatch`] when the columns are ragged.
/// - [`PlottingError::InvalidInput`] for more than
///   [`MAX_PAIRPLOT_VARIABLES`] variables, or for an
///   [`OffDiagKind`] that has no renderer yet.
///
/// # Example
///
/// ```rust,no_run
/// use ruviz::plots::composite::{DiagKind, PairPlotConfig, pairplot_with};
///
/// let a: Vec<f64> = (0..40).map(|i| i as f64).collect();
/// let b: Vec<f64> = a.iter().map(|v| v * 0.5).collect();
///
/// pairplot_with(
///     &[a, b],
///     700,
///     700,
///     PairPlotConfig::new()
///         .diag_kind(DiagKind::Kde)
///         .vars(vec!["a".into(), "b".into()])
///         .lower_only(),
/// )?
/// .save("pairplot_lower.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn pairplot_with<D>(
    columns: &[D],
    width: u32,
    height: u32,
    config: PairPlotConfig,
) -> Result<SubplotFigure>
where
    D: NumericData1D,
{
    let values = collect_columns(columns)?;
    let n_vars = values.len();
    if n_vars > MAX_PAIRPLOT_VARIABLES {
        return Err(PlottingError::InvalidInput(format!(
            "A pair plot draws {n_vars}x{n_vars} panels; {MAX_PAIRPLOT_VARIABLES} \
             variables is the ceiling"
        )));
    }

    let color = config.color();
    let ranges: Vec<(f64, f64)> = values.iter().map(|column| padded_range(column)).collect();
    let layout = compute_pairplot_layout(n_vars, &config);

    let mut assembled = figure(width, height)?;
    for cell in &layout.cells {
        let (x_var, y_var) = cell.var_indices;
        let Some(plot) = cell_panel(cell, &values, &ranges, &config, color)? else {
            continue;
        };

        // Label the outer edge only, exactly as seaborn does: the bottom row
        // carries the x names, the left column the y names.
        let named = |index: usize| config.vars.get(index).cloned();
        let plot = match (cell.row + 1 == n_vars, named(x_var)) {
            (true, Some(name)) => plot.xlabel(name),
            _ => plot,
        };
        let plot = match (cell.col == 0, named(y_var)) {
            (true, Some(name)) => plot.ylabel(name),
            _ => plot,
        };

        assembled = assembled.add_axes(cell.bounds, plot)?;
    }

    Ok(assembled)
}

/// Build one cell, or `None` when the cell is deliberately blank.
fn cell_panel(
    cell: &PairPlotCell,
    values: &[Vec<f64>],
    ranges: &[(f64, f64)],
    config: &PairPlotConfig,
    color: Color,
) -> Result<Option<Plot>> {
    let (x_var, y_var) = cell.var_indices;

    if cell.is_diagonal {
        let (hist, kde) = match config.diag_kind {
            DiagKind::Hist => (true, false),
            DiagKind::Kde => (false, true),
            DiagKind::None => return Ok(None),
        };
        return Ok(Marginal {
            values: &values[x_var],
            range: ranges[x_var],
            axis: MarginalAxis::X,
            hist,
            kde,
            rug: false,
            bins: config.bins,
            color,
        }
        .axes());
    }

    match config.off_diag_kind {
        OffDiagKind::Scatter => Ok(Some(
            panel()
                .scatter(&values[x_var], &values[y_var])
                .marker_size(config.scatter_size)
                .alpha(config.scatter_alpha)
                .color(color)
                .into_plot()
                .xlim(ranges[x_var].0, ranges[x_var].1)
                .ylim(ranges[y_var].0, ranges[y_var].1),
        )),
        unsupported => Err(PlottingError::InvalidInput(format!(
            "OffDiagKind::{unsupported:?} has no renderer yet, so a pair plot \
             cannot draw it. Use OffDiagKind::Scatter."
        ))),
    }
}

fn collect_columns<D: NumericData1D>(columns: &[D]) -> Result<Vec<Vec<f64>>> {
    if columns.is_empty() {
        return Err(PlottingError::EmptyDataSet);
    }

    let values: Vec<Vec<f64>> = columns
        .iter()
        .map(|column| column.try_collect_f64())
        .collect::<std::result::Result<_, _>>()?;

    let rows = values[0].len();
    if rows == 0 {
        return Err(PlottingError::EmptyDataSet);
    }
    if let Some(ragged) = values.iter().find(|column| column.len() != rows) {
        return Err(PlottingError::DataLengthMismatch {
            x_len: rows,
            y_len: ragged.len(),
            series_index: None,
        });
    }

    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn columns(count: usize) -> Vec<Vec<f64>> {
        (0..count)
            .map(|c| {
                (0..60)
                    .map(|i| ((i + c * 7) % 17) as f64 + c as f64)
                    .collect()
            })
            .collect()
    }

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

        // Every cell must be placeable as-is
        for cell in &layout.cells {
            cell.bounds.validate().unwrap();
        }
    }

    #[test]
    fn row_zero_is_the_top_row() {
        let layout = compute_pairplot_layout(3, &PairPlotConfig::default());
        let top = layout.cells.iter().find(|c| c.row == 0).unwrap();
        let bottom = layout.cells.iter().find(|c| c.row == 2).unwrap();

        assert!(
            top.bounds.y > bottom.bounds.y,
            "figure coordinates measure y upwards, so row 0 must sit highest"
        );
    }

    #[test]
    fn test_cell_variable_names() {
        let var_names = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let cell = PairPlotCell {
            row: 1,
            col: 0,
            var_indices: (0, 1),
            is_diagonal: false,
            bounds: FigureRect::new(0.0, 0.0, 0.5, 0.5),
        };

        let (x_name, y_name) = cell_variable_names(&cell, &var_names);
        assert_eq!(x_name, "A");
        assert_eq!(y_name, "B");
    }

    #[test]
    fn pairplot_places_one_axes_per_cell() {
        let data = columns(3);
        let figure = pairplot(&data, 900, 900).unwrap();

        assert_eq!(figure.axes_count(), 9);
        assert_eq!(figure.subplot_count(), 0);

        let lower = pairplot_with(&data, 900, 900, PairPlotConfig::new().lower_only()).unwrap();
        assert_eq!(lower.axes_count(), 6);
    }

    #[test]
    fn a_blank_diagonal_leaves_its_cells_empty() {
        let data = columns(3);
        let config = PairPlotConfig::new().diag_kind(DiagKind::None);
        let figure = pairplot_with(&data, 900, 900, config).unwrap();

        assert_eq!(
            figure.axes_count(),
            6,
            "DiagKind::None must skip the three diagonal cells"
        );
    }

    #[test]
    fn pairplot_rejects_input_it_cannot_draw() {
        let data = columns(2);

        assert!(matches!(
            pairplot(&Vec::<Vec<f64>>::new(), 400, 400),
            Err(PlottingError::EmptyDataSet)
        ));
        assert!(matches!(
            pairplot(&[vec![1.0, 2.0], vec![1.0]], 400, 400),
            Err(PlottingError::DataLengthMismatch { .. })
        ));
        assert!(matches!(
            pairplot(&columns(MAX_PAIRPLOT_VARIABLES + 1), 400, 400),
            Err(PlottingError::InvalidInput(_))
        ));
        for kind in [OffDiagKind::Reg, OffDiagKind::Kde] {
            let err = pairplot_with(&data, 400, 400, PairPlotConfig::new().off_diag_kind(kind))
                .expect_err("an unrendered kind must be reported, not substituted");
            assert!(matches!(err, PlottingError::InvalidInput(_)), "{kind:?}");
        }
    }

    #[test]
    fn pairplot_renders_every_cell() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pairplot.png");
        let data = columns(2);

        pairplot_with(
            &data,
            600,
            600,
            PairPlotConfig::new().vars(vec!["a".into(), "b".into()]),
        )
        .unwrap()
        .save(&path)
        .unwrap();

        let image = image::open(&path).unwrap().to_rgba8();
        assert_eq!(image.dimensions(), (600, 600));

        // Each of the four quadrants holds one cell, so each must have ink.
        let (half_w, half_h) = (image.width() / 2, image.height() / 2);
        let mut quadrants = [0_usize; 4];
        for (x, y, pixel) in image.enumerate_pixels() {
            if pixel.0[..3].iter().all(|channel| *channel > 245) {
                continue;
            }
            quadrants[usize::from(x >= half_w) + 2 * usize::from(y >= half_h)] += 1;
        }
        for (index, ink) in quadrants.iter().enumerate() {
            assert!(*ink > 200, "quadrant {index} is blank, ink={ink}");
        }
    }
}
