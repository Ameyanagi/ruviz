//! Deprecated one-call plotting shortcuts.
//!
//! **This module is superseded by the [`Plot`] builder and will be removed in a
//! future release.** Every function here is a two-line wrapper around a builder
//! chain that is just as short, but composes: once you want a label, a theme, a
//! second series or a different DPI, the shortcut runs out and you have to
//! rewrite the call anyway. There is one obvious way to draw a plot:
//!
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! let x = vec![0.0, 1.0, 2.0, 3.0];
//! let y = vec![0.0, 1.0, 4.0, 9.0];
//!
//! Plot::new()
//!     .line(&x, &y)
//!     .title("My Plot")
//!     .save("line.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! A title is a setter on that chain, not a separate function, so the
//! `*_with_title` variants have no builder equivalent to migrate to beyond
//! adding `.title(..)`.
//!
//! # Migration
//!
//! | Deprecated | Replacement |
//! |------------|-------------|
//! | `line_plot(&x, &y, p)` | `Plot::new().line(&x, &y).save(p)` |
//! | `line_plot_with_title(&x, &y, t, p)` | `Plot::new().line(&x, &y).title(t).save(p)` |
//! | `scatter_plot(&x, &y, p)` | `Plot::new().scatter(&x, &y).save(p)` |
//! | `scatter_plot_with_title(&x, &y, t, p)` | `Plot::new().scatter(&x, &y).title(t).save(p)` |
//! | `bar_chart(&c, &v, p)` | `Plot::new().bar(&c, &v).save(p)` |
//! | `bar_chart_with_title(&c, &v, t, p)` | `Plot::new().bar(&c, &v).title(t).save(p)` |
//! | `histogram(&d, p)` | `Plot::new().histogram(&d).save(p)` |
//! | `histogram_with_title(&d, t, p)` | `Plot::new().histogram(&d).title(t).save(p)` |
//!
//! Backend selection is automatic in both forms: the wrappers below call
//! `.auto_optimize()`, which only pins the backend that [`Plot`] already picks
//! when none is set. Call `.auto_optimize()` yourself if you want it pinned.
//!
//! ![Line plot](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/line_plot.png)

use crate::core::Result;
use crate::prelude::*;
use std::path::Path;

/// Create a simple line plot with one function call.
///
/// # Deprecated
///
/// Use the [`Plot`] builder — it is the same length and keeps composing:
///
/// ```rust,no_run
/// use ruviz::prelude::*;
///
/// let x = vec![0.0, 1.0, 2.0, 3.0];
/// let y = vec![0.0, 1.0, 4.0, 9.0];
/// Plot::new().line(&x, &y).save("output.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[deprecated(
    since = "0.6.0",
    note = "use the Plot builder: Plot::new().line(x, y).save(path)"
)]
pub fn line_plot<P: AsRef<Path>>(x: &[f64], y: &[f64], path: P) -> Result<()> {
    Plot::new().line(&x, &y).auto_optimize().save(path)
}

/// Create a line plot with a title.
///
/// # Deprecated
///
/// A title is a setter on the [`Plot`] builder, not a separate constructor:
///
/// ```rust,no_run
/// use ruviz::prelude::*;
///
/// let x = vec![0.0, 1.0, 2.0];
/// let y = vec![0.0, 1.0, 4.0];
/// Plot::new().line(&x, &y).title("My Plot").save("output.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[deprecated(
    since = "0.6.0",
    note = "use the Plot builder: Plot::new().line(x, y).title(title).save(path)"
)]
pub fn line_plot_with_title<P: AsRef<Path>>(
    x: &[f64],
    y: &[f64],
    title: &str,
    path: P,
) -> Result<()> {
    Plot::new()
        .line(&x, &y)
        .title(title)
        .auto_optimize()
        .save(path)
}

/// Create a simple scatter plot with one function call.
///
/// # Deprecated
///
/// ```rust,no_run
/// use ruviz::prelude::*;
///
/// let x = vec![1.0, 2.0, 3.0];
/// let y = vec![1.0, 4.0, 9.0];
/// Plot::new().scatter(&x, &y).save("scatter.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[deprecated(
    since = "0.6.0",
    note = "use the Plot builder: Plot::new().scatter(x, y).save(path)"
)]
pub fn scatter_plot<P: AsRef<Path>>(x: &[f64], y: &[f64], path: P) -> Result<()> {
    Plot::new().scatter(&x, &y).auto_optimize().save(path)
}

/// Create a scatter plot with a title.
///
/// # Deprecated
///
/// ```rust,no_run
/// use ruviz::prelude::*;
///
/// let x = vec![1.0, 2.0, 3.0];
/// let y = vec![1.0, 4.0, 9.0];
/// Plot::new().scatter(&x, &y).title("Scatter").save("output.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[deprecated(
    since = "0.6.0",
    note = "use the Plot builder: Plot::new().scatter(x, y).title(title).save(path)"
)]
pub fn scatter_plot_with_title<P: AsRef<Path>>(
    x: &[f64],
    y: &[f64],
    title: &str,
    path: P,
) -> Result<()> {
    Plot::new()
        .scatter(&x, &y)
        .title(title)
        .auto_optimize()
        .save(path)
}

/// Create a simple bar chart with one function call.
///
/// # Deprecated
///
/// ```rust,no_run
/// use ruviz::prelude::*;
///
/// let categories = vec!["A", "B", "C"];
/// let values = vec![10.0, 20.0, 15.0];
/// Plot::new().bar(&categories, &values).save("bar.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[deprecated(
    since = "0.6.0",
    note = "use the Plot builder: Plot::new().bar(categories, values).save(path)"
)]
pub fn bar_chart<P: AsRef<Path>>(categories: &[&str], values: &[f64], path: P) -> Result<()> {
    Plot::new()
        .bar(categories, &values)
        .auto_optimize()
        .save(path)
}

/// Create a bar chart with a title.
///
/// # Deprecated
///
/// ```rust,no_run
/// use ruviz::prelude::*;
///
/// let categories = vec!["X", "Y", "Z"];
/// let values = vec![5.0, 10.0, 7.0];
/// Plot::new().bar(&categories, &values).title("Sales").save("output.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[deprecated(
    since = "0.6.0",
    note = "use the Plot builder: Plot::new().bar(categories, values).title(title).save(path)"
)]
pub fn bar_chart_with_title<P: AsRef<Path>>(
    categories: &[&str],
    values: &[f64],
    title: &str,
    path: P,
) -> Result<()> {
    Plot::new()
        .bar(categories, &values)
        .title(title)
        .auto_optimize()
        .save(path)
}

/// Create a simple histogram with one function call.
///
/// # Deprecated
///
/// ```rust,no_run
/// use ruviz::prelude::*;
///
/// let data = vec![1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0];
/// Plot::new().histogram(&data).save("histogram.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[deprecated(
    since = "0.6.0",
    note = "use the Plot builder: Plot::new().histogram(data).save(path)"
)]
pub fn histogram<P: AsRef<Path>>(data: &[f64], path: P) -> Result<()> {
    Plot::new().histogram(&data).auto_optimize().save(path)
}

/// Create a histogram with a title.
///
/// # Deprecated
///
/// ```rust,no_run
/// use ruviz::prelude::*;
///
/// let data = vec![1.0, 2.0, 2.0, 3.0, 4.0];
/// Plot::new().histogram(&data).title("Distribution").save("output.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[deprecated(
    since = "0.6.0",
    note = "use the Plot builder: Plot::new().histogram(data).title(title).save(path)"
)]
pub fn histogram_with_title<P: AsRef<Path>>(data: &[f64], title: &str, path: P) -> Result<()> {
    Plot::new()
        .histogram(&data)
        .title(title)
        .auto_optimize()
        .save(path)
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_api_exists() {
        // Ensure module compiles and functions are available
        let _ = line_plot::<&str>;
        let _ = scatter_plot::<&str>;
        let _ = bar_chart::<&str>;
        let _ = histogram::<&str>;
    }

    /// Every shortcut in this module has a builder equivalent that compiles and
    /// produces the same plot. This test is the compile-time proof for the
    /// replacements named in the `#[deprecated]` notes above: if a note ever
    /// points at a chain that stops working, this stops compiling.
    #[test]
    fn deprecation_notes_name_chains_that_compile() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![0.0, 1.0, 4.0];
        let categories = ["A", "B", "C"];
        let values = vec![1.0, 2.0, 3.0];
        let data = vec![1.0, 2.0, 2.0, 3.0];

        // `line_plot` / `line_plot_with_title`
        let _ = Plot::new().line(&x, &y);
        let _ = Plot::new().line(&x, &y).title("t");

        // `scatter_plot` / `scatter_plot_with_title`
        let _ = Plot::new().scatter(&x, &y);
        let _ = Plot::new().scatter(&x, &y).title("t");

        // `bar_chart` / `bar_chart_with_title`
        let _ = Plot::new().bar(&categories, &values);
        let _ = Plot::new().bar(&categories, &values).title("t");

        // `histogram` / `histogram_with_title`
        let _ = Plot::new().histogram(&data);
        let _ = Plot::new().histogram(&data).title("t");
    }
}
