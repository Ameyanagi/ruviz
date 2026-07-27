//! Plot type implementations
//!
//! **29 plot types are reachable from the [`Plot`](crate::core::Plot)
//! builder**, plus 4 more from `Plot3D` when the `3d` feature is enabled — 33
//! in total. The first table below is the complete list. If a type is not in
//! it, it has no builder entry point and *cannot* be drawn through the public
//! API, however many types this source tree appears to contain.
//!
//! Neither table is maintained by hand. `catalog_is_true` — the test module at
//! the bottom of this file — reads the builder's own source and fails if a
//! method named here does not exist, if a type listed as unreachable has
//! quietly grown a builder, or if a documented count disagrees with the number
//! of rows. The catalog cannot silently drift from the API again, which is how
//! "30+ plot types" came to be printed above a table with three unreachable
//! rows in it.
//!
//! ## Core Traits
//!
//! All plot types implement the core traits defined in [`traits`]:
//!
//! - [`PlotCompute`]: Data transformation
//! - [`PlotData`]: Common data interface
//! - [`PlotRender`]: Rendering to canvas
//!
//! ## Available plot types
//!
//! Every one of these joins the same chain:
//! `Plot::new().<series>(..).label(..).color(..).legend_best().save(..)`.
//!
//! The one exception is marked in the table: `fill_between` shades the gap
//! between two curves, which is an *annotation* on a figure rather than a series
//! of its own, so it returns the `Plot` and is chained after a series
//! (`.line(&x, &y).fill_between(&x, &lo, &hi).label("band")`) rather than in
//! place of one.
//!
//! | Category | Module | Plot types | `Plot` builder method |
//! |----------|--------|------------|-----------------------|
//! | Basic | [`basic`] | Line, Scatter, Bar | `line`, `scatter`, `bar` |
//! | Statistical | [`histogram`], [`boxplot`] | Histogram, Box Plot | `histogram`, `boxplot` |
//! | Distribution | [`distribution`] | KDE, ECDF, Violin, Boxen, Rug | `kde`, `ecdf`, `violin`, `boxen`, `rug` |
//! | Categorical | [`categorical`] | Strip, Swarm, Grouped Bar, Stacked Bar | `strip`, `swarm`, `grouped_bar`, `stacked_bar` |
//! | Composition | [`composition`] | Pie, Donut | `pie`, `donut` |
//! | Continuous | [`continuous`] | Contour, Area, Fill Between, Hexbin, Stacked Area | `contour`, `area`, `fill_between` (annotation), `hexbin`, `stacked_area` |
//! | Discrete | [`discrete`] | Step, Stem | `step`, `stem` |
//! | Grid | [`heatmap`] | Heatmap | `heatmap` |
//! | Hierarchical | [`hierarchical`] | Dendrogram | `dendrogram` |
//! | Error | [`error`] | Error Bars | `error_bars`, `error_bars_xy` |
//! | Polar | [`polar`] | Polar Line, Radar | `polar_line`, `radar` |
//! | Vector | [`vector`] | Quiver | `quiver` |
//! | 3D (`3d` feature) | `three_d` | Scatter3D, Line3D, Surface, Wireframe | `Plot3D::{scatter3d, line3d, surface, wireframe}` |
//!
//! Grouped bar, stacked bar and stacked area take N named value columns over
//! one shared axis — `.grouped_bar(&categories, &[("Q1", &q1), ("Q2", &q2)])` —
//! and push one ordinary series per column, so each column gets its own palette
//! slot, its own legend entry and the same `.color()`/`.label()` rules as a
//! line. The chain is unchanged; only the series count is.
//!
//! Joint plots and pair plots are *figures*, not series:
//! [`composite::jointplot()`] and [`composite::pairplot()`] return a
//! [`SubplotFigure`](crate::core::SubplotFigure) like
//! [`subplots`](crate::core::subplots) does, so they are not in the count above.
//!
//! ## Not available: implemented but unreachable
//!
//! The following live in this tree but have **no builder entry point**, so a
//! user cannot draw them with the chain above. They are documented here so the
//! table above is not mistaken for a partial list — not as advertised features.
//!
//! The `Renderer` column is the honest distinction. Where it says yes, the type
//! implements [`PlotRender`] and you can draw it by calling
//! [`PlotCompute`] and [`PlotRender::render`] yourself against a
//! [`PlotArea`]; the missing piece is only the one-line `Plot::` entry point.
//! Where it says no, there is nothing to draw with at all.
//!
//! | Type | Module | Compute | Renderer | `Plot` builder |
//! |------|--------|---------|----------|----------------|
//! | 2D KDE | [`distribution`] | yes | **no** | **no** |
//! | Reg plot, Resid plot | `regression` | yes | **no** | **no** |
//!
//! Sankey diagrams and streamplots are **not implemented in any form**. The
//! empty `flow` module that used to advertise them on docs.rs is gone.

pub mod traits;

// Basic plot types (line, scatter, bar)
pub mod basic;

pub mod boxplot;
pub mod heatmap;
pub mod histogram;
pub mod statistics;

// New plot type categories (placeholders for now)
pub mod categorical;
pub mod composition;
pub mod continuous;
pub mod discrete;
pub mod distribution;
pub mod error;
pub mod hierarchical;
pub mod polar;
#[cfg(feature = "3d")]
pub mod three_d;
pub mod vector;

// ---------------------------------------------------------------------------
// Unwired plot families.
//
// The rule, applied to every module in this file: **a module is `#[doc(hidden)]`
// if and only if it has no renderer at all.** A type with a `PlotRender` impl is
// something a user can draw today by driving the trait directly, so its docs are
// worth reading and its module stays visible — with an honest "no builder" row
// in the second table above if it has no `Plot::` entry point yet. A module that
// returns only rectangles or only numbers has nothing to draw with, so listing
// it on docs.rs advertises a plot type that does not exist.
//
// `composite` used to be hidden under that rule and no longer is: `jointplot`
// and `pairplot` draw, through `SubplotFigure::add_axes`. `regression` below is
// still in the second category. It stays `pub` because its functions are
// correct, usable, and referenced by docs/guide/04_plot_types.md; it is hidden
// so docs.rs stops implying it is a plot type. Un-hide it at the point where it
// grows a renderer — see Phase 10 of
// docs/roadmaps/ruviz-audit-remediation-plan.md.
//
// Why `#[doc(hidden)]` and not an `unstable-plots` cargo feature: a feature is a
// promise about a *compilation* boundary, and there is nothing here to compile
// out. These are pure functions over `&[f64]` with no dependencies, no cfg
// sites and no cost when unused; gating them would add a declared feature that
// gates only visibility, which tests/feature_hygiene_test.rs exists to forbid,
// and would break the guide snippets and tests/config_enum_defaults_test.rs
// that call them today. `#[doc(hidden)]` costs nothing, breaks nothing, and
// removes the only actual harm — docs.rs listing a plot type you cannot draw.
//
// Nothing is deprecated here: the compute functions are correct and useful on
// their own, they are simply not a rendering feature yet.
// ---------------------------------------------------------------------------

/// Multi-panel composites (joint plot, pair plot). Figure composers built on
/// [`SubplotFigure::add_axes`](crate::core::subplot::SubplotFigure::add_axes).
pub mod composite;

/// Regression plots (regplot, residplot). Compute only — no renderer and no
/// `Plot` builder; not reachable as a plot type.
#[doc(hidden)]
pub mod regression;

// Core trait exports
pub use traits::{
    AxisScaleSupport, ComputedSeries, ComputedStyle, PlotArea, PlotCompute, PlotConfig, PlotData,
    PlotPrimitive, PlotRender, StyledShape, draw_primitives, draw_primitives_svg,
};

// Basic plot config exports
pub use basic::{BarConfig, BarOrientation, LineConfig, ScatterConfig};

// Distribution plot exports
pub use distribution::{
    BandwidthMethod, Boxen, BoxenConfig, BoxenData, BoxenOrientation, Ecdf, EcdfConfig, EcdfData,
    EcdfStat, Kde, KdeConfig, KdeData, Violin, ViolinConfig, ViolinData, compute_boxen,
    compute_ecdf, compute_kde,
};

pub use boxplot::{
    BoxPlotConfig, BoxPlotData, CATEGORY_SLOT_HALF_WIDTH, OutlierMethod, WhiskerMethod,
    calculate_box_plot, category_slot_span,
};
pub use heatmap::{
    ColorbarFontSizes, HeatmapConfig, HeatmapData, HeatmapOrigin, Interpolation, process_heatmap,
    process_heatmap_flat,
};
pub use histogram::{BinMethod, HistogramConfig, HistogramData, calculate_histogram};
pub use statistics::{iqr, mean, median, percentile, std_dev};

// Contour plot exports
pub use continuous::contour::{
    ContourConfig, ContourInterpolation, ContourPlotData, compute_contour_plot,
};
pub use discrete::{StemConfig, StemMarker, StemOrientation, StepConfig, StepWhere};

// Pie chart exports
pub use composition::pie::{DEFAULT_DONUT_INNER_RADIUS, PieConfig, PieData};

// Polar and Radar exports
pub use polar::polar_plot::{PolarPlotConfig, PolarPlotData, compute_polar_plot};
pub use polar::radar::{
    RadarConfig, RadarPlotData, compute_radar_chart, compute_radar_chart_with_labels,
};
#[cfg(feature = "3d")]
pub use three_d::{
    Line3DConfig, Scatter3DConfig, Surface3DConfig, SurfaceSampling, SurfaceShading,
    Wireframe3DConfig,
};
pub use vector::{
    Quiver, QuiverArrow, QuiverConfig, QuiverInput, QuiverPivot, QuiverPlotData, compute_quiver,
    quiver_range,
};

/// Checks that the module documentation above is true.
///
/// The catalog is the crate's most load-bearing prose: it is the only place a
/// user is told which of the ~34 plot types in this tree they can actually
/// draw. It was wrong before (a "30+ plot types" headline over a table listing
/// Hexbin, Grouped Bar and Stacked Bar, none of which had a builder), and prose
/// has no compiler. So the list lives here, as data, and the doc table is
/// checked against it and against the builder's own source.
///
/// A type that has a renderer but no builder lives in `AWAITING_A_BUILDER`
/// instead, and is asserted to *stay* unreachable — so wiring one is what makes
/// a test fail, with the edits to make spelled out in the message.
///
/// Add a plot type by adding one row to `CATALOG` and one row to the doc
/// table. Miss either half and these tests fail with the exact line to fix.
#[cfg(test)]
mod catalog_is_true {
    /// Every plot type reachable from `Plot`, with the builder methods that
    /// draw it. One entry per *type*, so the length is the advertised count.
    const CATALOG: &[(&str, &[&str])] = &[
        ("Line", &["line"]),
        ("Scatter", &["scatter"]),
        ("Bar", &["bar"]),
        ("Histogram", &["histogram"]),
        ("Box Plot", &["boxplot"]),
        ("KDE", &["kde"]),
        ("ECDF", &["ecdf"]),
        ("Violin", &["violin"]),
        ("Boxen", &["boxen"]),
        ("Pie", &["pie"]),
        ("Donut", &["donut"]),
        ("Contour", &["contour"]),
        ("Area", &["area"]),
        ("Fill Between", &["fill_between"]),
        ("Step", &["step"]),
        ("Stem", &["stem"]),
        ("Heatmap", &["heatmap"]),
        ("Error Bars", &["error_bars", "error_bars_xy"]),
        ("Polar Line", &["polar_line"]),
        ("Radar", &["radar"]),
        ("Quiver", &["quiver"]),
        ("Rug", &["rug"]),
        ("Strip", &["strip"]),
        ("Swarm", &["swarm"]),
        ("Hexbin", &["hexbin"]),
        ("Dendrogram", &["dendrogram"]),
        ("Grouped Bar", &["grouped_bar"]),
        ("Stacked Bar", &["stacked_bar"]),
        ("Stacked Area", &["stacked_area"]),
    ];

    /// Types that have a renderer but no `Plot` builder, with the method name
    /// they would take.
    ///
    /// **Empty**: every renderable type in this tree is now reachable from the
    /// builder. The constant and its two tests stay because the mechanism is
    /// the point — add a renderer without a builder and you list it here, and
    /// `no_builder_exists_yet_for_the_types_listed_as_unreachable` asserts the
    /// method is *absent* until you wire it, then fails with the four edits
    /// that go with wiring it. Without it, wiring `Plot::rug` would leave rug
    /// advertised as unreachable in the second doc table and missing from the
    /// first, silently. The types left in the `Not available` table (2D KDE,
    /// reg plot, resid plot) have no renderer at all, so they do not belong
    /// here.
    const AWAITING_A_BUILDER: &[(&str, &str)] = &[];

    /// Plot types reachable only from `Plot3D`, behind the `3d` feature.
    const CATALOG_3D: &[(&str, &str)] = &[
        ("Scatter3D", "scatter3d"),
        ("Line3D", "line3d"),
        ("Surface", "surface"),
        ("Wireframe", "wireframe"),
    ];

    /// Every file that may define a `Plot`/`PlotBuilder` series method. If a new
    /// builder method lands in a file that is not listed here, add it — the
    /// failure message points at this constant.
    const BUILDER_SOURCES: &[&str] = &[
        include_str!("../core/plot/series_api.rs"),
        include_str!("../core/plot/series_builders.rs"),
        include_str!("../core/plot/builder.rs"),
        include_str!("../core/plot/annotations.rs"),
    ];

    /// The `//!` block at the top of this file and nothing else, with the
    /// comment markers and line wrapping removed.
    ///
    /// Stopping at the first non-`//!` line keeps a name in `CATALOG` below from
    /// being mistaken for the doc claiming it. Unwrapping keeps an assertion
    /// about a sentence from also being an assertion about where it wraps.
    fn module_doc() -> String {
        include_str!("mod.rs")
            .lines()
            .take_while(|line| line.starts_with("//!"))
            .map(|line| line.trim_start_matches("//!").trim())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// True when `source` declares `pub fn <method>`, and not merely a longer
    /// name that starts with it (`bar` must not be satisfied by `bar_source`).
    fn declares(source: &str, method: &str) -> bool {
        let needle = format!("pub fn {method}");
        source.match_indices(&needle).any(|(at, _)| {
            let rest = &source[at + needle.len()..];
            matches!(rest.chars().next(), Some('(' | '<'))
        })
    }

    fn has_builder_method(method: &str) -> bool {
        BUILDER_SOURCES.iter().any(|s| declares(s, method))
    }

    #[test]
    fn every_catalogued_plot_type_has_a_builder_method() {
        for (plot_type, methods) in CATALOG {
            for method in *methods {
                assert!(
                    has_builder_method(method),
                    "the plots module doc advertises `{plot_type}` as drawable \
                     via `Plot::{method}`, but no `pub fn {method}` exists in \
                     any BUILDER_SOURCES file. Wire the method, drop the row \
                     from CATALOG and the doc table, or add the file that \
                     defines it to BUILDER_SOURCES."
                );
            }
        }
    }

    #[test]
    fn no_builder_exists_yet_for_the_types_listed_as_unreachable() {
        for (plot_type, method) in AWAITING_A_BUILDER {
            assert!(
                !has_builder_method(method),
                "`Plot::{method}` now exists, so `{plot_type}` is reachable and \
                 the docs in this file are out of date. Four edits: move it from \
                 AWAITING_A_BUILDER to CATALOG, add a row to the first doc \
                 table, delete its row from the `Not available` table, and bump \
                 both counts in the opening paragraph."
            );
        }
    }

    #[test]
    fn the_unreachable_types_are_listed_as_unreachable() {
        let doc = module_doc();
        let (_, unreachable) = doc
            .split_once("## Not available: implemented but unreachable")
            .expect("the module doc must keep its `Not available` section");
        for (plot_type, _) in AWAITING_A_BUILDER {
            assert!(
                unreachable.contains(*plot_type),
                "`{plot_type}` has no builder but is not listed in the \
                 `Not available` table, so a reader has no way to learn that"
            );
        }
    }

    /// The crate-root doc and the README each state the catalogue size, and both
    /// drifted once already: `src/lib.rs` still said "21 types from `Plot`" and
    /// listed hexbin, strip, swarm and dendrogram as undrawable *after* they had
    /// builders, because this module's own tests only ever read this module.
    /// docs.rs renders `src/lib.rs`, so that stale claim was the first thing a
    /// user read. Assert against every file that states the count, not just the
    /// nearest one.
    #[test]
    fn every_document_that_states_the_catalogue_size_agrees_with_the_api() {
        let expected = CATALOG.len();
        let claim = format!("{expected} types from [`Plot`]");
        let crate_doc = include_str!("../lib.rs");
        assert!(
            crate_doc.contains(&claim),
            "src/lib.rs must say \"{claim}\" — it is the docs.rs landing page, \
             and the builder now exposes {expected} plot types"
        );

        // Asserting one exact sentence is not a guard: the same fact is stated
        // more than once, in different words. An earlier version of this test
        // checked only the sentence above, and the feature bullet near the top
        // of src/lib.rs went on saying "19 Plot Types" for three commits after
        // the catalogue reached 26. Find EVERY "<n> plot type(s)" claim in the
        // documents and require each number to be one we actually stand behind.
        let with_3d = expected + CATALOG_3D.len();
        for (doc_name, doc) in [
            ("src/lib.rs", crate_doc),
            ("README.md", include_str!("../../README.md")),
        ] {
            let lowered = doc.to_lowercase();
            for (offset, _) in lowered.match_indices("plot type") {
                // Walk back over the whitespace and grab the preceding token.
                let head = &lowered[..offset];
                let token: String = head
                    .trim_end_matches(|c: char| c.is_whitespace() || c == '*')
                    .chars()
                    .rev()
                    .take_while(char::is_ascii_digit)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                let Ok(claimed) = token.parse::<usize>() else {
                    continue; // not a numeric claim, e.g. "each plot type"
                };
                assert!(
                    claimed == expected || claimed == with_3d,
                    "{doc_name} claims \"{claimed} plot types\", but the builder \
                     exposes {expected} ({with_3d} with the `3d` feature). Every \
                     statement of the count has to move together."
                );
            }
        }

        let readme = include_str!("../../README.md");
        assert!(
            readme.contains(&format!("{expected} plot types"))
                || readme.contains(&format!("All {expected} ")),
            "README.md must state the catalogue size as {expected}"
        );

        // Every type that HAS a builder must not still be advertised as
        // undrawable anywhere. This is the specific drift that shipped.
        for (plot_type, methods) in CATALOG {
            let Some(method) = methods.first() else {
                continue;
            };
            if !has_builder_method(method) {
                continue;
            }
            for (doc_name, doc) in [("src/lib.rs", crate_doc), ("README.md", readme)] {
                let Some((_, after)) = doc.split_once("no builder method") else {
                    continue;
                };
                let sentence = after.split('.').next().unwrap_or("");
                assert!(
                    !sentence.contains(&plot_type.to_lowercase()),
                    "{doc_name} still lists `{plot_type}` as having no builder \
                     method, but `Plot::{method}` exists"
                );
            }
        }
    }

    #[cfg(feature = "3d")]
    #[test]
    fn every_catalogued_3d_plot_type_has_a_builder_method() {
        const PLOT3D_BUILDER: &str = include_str!("../core/plot3d/builder.rs");
        for (plot_type, method) in CATALOG_3D {
            assert!(
                PLOT3D_BUILDER.contains(&format!("pub fn {method}")),
                "the plots module doc advertises `{plot_type}` via \
                 `Plot3D::{method}`, but no `pub fn {method}` exists in \
                 src/core/plot3d/builder.rs"
            );
        }
    }

    #[test]
    fn the_advertised_counts_match_the_catalog() {
        let doc = module_doc();
        let two_d = CATALOG.len();
        let three_d = CATALOG_3D.len();
        let total = two_d + three_d;

        let headline = format!("**{two_d} plot types are reachable");
        assert!(
            doc.contains(&headline),
            "the module doc must open with `{headline}...`; CATALOG has \
             {two_d} entries. Update whichever is wrong."
        );
        let total_claim = format!("— {total} in total");
        assert!(
            doc.contains(&total_claim),
            "the module doc must claim `{total_claim}` \
             ({two_d} from `Plot` + {three_d} from `Plot3D`)"
        );
    }

    #[test]
    fn the_doc_table_lists_every_catalogued_method() {
        let doc = module_doc();
        for (plot_type, methods) in CATALOG {
            for method in *methods {
                let named = doc.contains(&format!("`{method}`"));
                let chained = doc.contains(&format!(".{method}("));
                assert!(
                    named || chained,
                    "`Plot::{method}` draws `{plot_type}` but is not named in \
                     the doc table above, so a reader cannot find it"
                );
            }
        }
    }

    #[test]
    fn the_unreachable_table_does_not_claim_a_wired_type() {
        let doc = module_doc();
        let (_, unreachable) = doc
            .split_once("## Not available: implemented but unreachable")
            .expect("the module doc must keep its `Not available` section");
        for (plot_type, _) in CATALOG {
            assert!(
                !unreachable.contains(&format!("| {plot_type} ")),
                "`{plot_type}` has a builder method but is still listed as \
                 unreachable; delete its row from the second table"
            );
        }
    }
}
