//! Composite plot types — figures made of several panels.
//!
//! [`jointplot()`](fn@crate::plots::composite::jointplot) draws a bivariate
//! panel with a marginal distribution above it and another to its right;
//! [`pairplot()`](fn@crate::plots::composite::pairplot) draws every variable against every
//! other with a distribution on the diagonal. Both return the same
//! [`SubplotFigure`](crate::core::subplot::SubplotFigure) that
//! [`subplots`](crate::core::subplot::subplots) returns, so the figure-level
//! chain is the one already learned — `.suptitle(..)`, `.theme(..)`,
//! `.save(..)` — and more panels can be added with
//! [`add_axes`](crate::core::subplot::SubplotFigure::add_axes).
//!
//! ```rust,no_run
//! use ruviz::plots::composite::jointplot;
//!
//! let x: Vec<f64> = (0..200).map(|i| i as f64 * 0.05).collect();
//! let y: Vec<f64> = x.iter().map(|v| v.cos()).collect();
//!
//! jointplot(&x, &y, 800, 800)?.save("joint.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! These are **figure composers, not series**: they build a whole image out of
//! several `Plot`s, so they cannot join the
//! `Plot::new().<series>(..).label(..).save(..)` chain and are not counted in
//! the `Plot` builder's plot-type catalogue. `subplots` is not in that
//! catalogue either, for the same reason — a figure is not a series.
//!
//! The geometry each composer uses is public on its own:
//! [`joint_plot_layout()`](crate::plots::composite::joint_plot_layout) and
//! [`compute_pairplot_layout()`](crate::plots::composite::compute_pairplot_layout) return panel
//! rectangles in the figure-relative, lower-left-origin coordinates
//! [`add_axes`](crate::core::subplot::SubplotFigure::add_axes) takes, and
//! [`compute_marginal_histogram()`](crate::plots::composite::compute_marginal_histogram)
//! bins one margin.
//!
//! What is still not drawable, and errors rather than substituting something
//! else: [`JointKind::Reg`](crate::plots::composite::JointKind::Reg),
//! [`JointKind::Kde`](crate::plots::composite::JointKind::Kde),
//! [`JointKind::Resid`](crate::plots::composite::JointKind::Resid),
//! [`OffDiagKind::Reg`](crate::plots::composite::OffDiagKind::Reg) and
//! [`OffDiagKind::Kde`](crate::plots::composite::OffDiagKind::Kde), none of
//! which has a renderer anywhere in the crate.
//! [`PairPlotConfig::colors`](crate::plots::composite::PairPlotConfig::colors)
//! carries hue groups that are not implemented; its first entry colours the
//! whole matrix.
//!
//! Inset/zoom axes are *not* here; those are real and live on the `Plot`
//! builder as `inset_layout`/`inset_anchor`.

pub mod jointplot;
pub mod pairplot;

pub use jointplot::{
    JointKind, JointPlotConfig, JointPlotLayout, MarginalHistogram, compute_marginal_histogram,
    joint_plot_layout, jointplot, jointplot_with, panel_config,
};
pub use pairplot::{
    DiagKind, MAX_PAIRPLOT_VARIABLES, OffDiagKind, PairPlotCell, PairPlotConfig, PairPlotLayout,
    cell_variable_names, compute_pairplot_layout, pairplot, pairplot_with,
};

/// Pins the module doc's claim about what is and is not drawable here.
///
/// The predecessor of this module documented ~20 setters as inert and had one
/// test module holding that claim in place; that is how ~35 silently-inert
/// setters accumulated crate-wide in the first place. The claim has changed —
/// most of those fields now reach a renderer — so the tests changed with it,
/// and they still exist for the same reason: a factual statement about
/// behaviour in a doc comment needs something underneath it.
#[cfg(test)]
mod the_module_doc_is_true {
    use super::*;

    fn sample() -> (Vec<f64>, Vec<f64>) {
        let x: Vec<f64> = (0..80).map(|i| i as f64 * 0.25).collect();
        let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();
        (x, y)
    }

    /// Every appearance knob set to a non-default value.
    fn loud_joint_config() -> JointPlotConfig {
        JointPlotConfig::default()
            .marginal_hist(false)
            .marginal_kde(false)
            .rugplot(true)
            .bins(7)
            .color(crate::render::Color::from_rgb(1, 2, 3))
    }

    #[test]
    fn joint_plot_layout_still_depends_only_on_the_marginal_ratio() {
        let quiet = joint_plot_layout(JointPlotConfig::default().marginal_ratio);
        let loud = joint_plot_layout(loud_joint_config().marginal_ratio);

        assert_eq!(quiet, loud);

        // ...and it does move when the one geometric field moves.
        assert_ne!(quiet.main_bounds, joint_plot_layout(0.4).main_bounds);
    }

    #[test]
    fn the_appearance_fields_reach_the_figure() {
        let (x, y) = sample();

        // `marginal_hist`/`marginal_kde`/`rugplot` decide whether a marginal
        // panel exists at all, so the panel count is a behavioural witness.
        let blank = JointPlotConfig::default()
            .marginal_hist(false)
            .marginal_kde(false);
        assert_eq!(
            jointplot_with(&x, &y, 400, 400, blank.clone())
                .unwrap()
                .axes_count(),
            1
        );
        assert_eq!(
            jointplot_with(&x, &y, 400, 400, blank.rugplot(true))
                .unwrap()
                .axes_count(),
            3
        );
        assert_eq!(
            jointplot_with(&x, &y, 400, 400, loud_joint_config())
                .unwrap()
                .axes_count(),
            3
        );
    }

    #[test]
    fn the_kinds_without_a_renderer_say_so() {
        let (x, y) = sample();
        for kind in [JointKind::Reg, JointKind::Kde, JointKind::Resid] {
            assert!(
                jointplot_with(&x, &y, 400, 400, JointPlotConfig::default().kind(kind)).is_err(),
                "{kind:?} is documented as undrawable and must not silently \
                 render as something else"
            );
        }
        for kind in [OffDiagKind::Reg, OffDiagKind::Kde] {
            let config = PairPlotConfig::default().off_diag_kind(kind);
            assert!(pairplot_with(&[x.clone(), y.clone()], 400, 400, config).is_err());
        }
    }

    #[test]
    fn pairplot_layout_reads_only_the_three_triangle_flags() {
        let base = PairPlotConfig {
            vars: vec!["a".into(), "b".into(), "c".into()],
            ..Default::default()
        };
        let styled = PairPlotConfig {
            diag_kind: DiagKind::Kde,
            off_diag_kind: OffDiagKind::Reg,
            colors: Some(vec![crate::render::Color::from_rgb(9, 9, 9)]),
            scatter_size: 42.0,
            scatter_alpha: 0.1,
            bins: 99,
            ..base.clone()
        };

        let quiet = compute_pairplot_layout(3, &base);
        let loud = compute_pairplot_layout(3, &styled);
        assert_eq!(quiet.cells.len(), loud.cells.len());
        for (a, b) in quiet.cells.iter().zip(&loud.cells) {
            assert_eq!(a.bounds, b.bounds);
            assert_eq!(a.var_indices, b.var_indices);
        }

        // The three flags that are live really are live.
        let lower_only = PairPlotConfig {
            upper: false,
            diag: false,
            ..base
        };
        let lower_cells = compute_pairplot_layout(3, &lower_only).cells.len();
        assert!(lower_cells < quiet.cells.len());
    }

    #[test]
    fn both_composers_return_the_same_figure_type_as_subplots() {
        // Not a tautology in Rust's type system alone: the point of the claim
        // is that a composite can be extended and finished exactly like a
        // grid figure, which only holds if it *is* one.
        let (x, y) = sample();
        let extended = jointplot(&x, &y, 600, 600)
            .unwrap()
            .suptitle("joint")
            .add_axes([0.82, 0.82, 0.16, 0.16], crate::core::Plot::new())
            .unwrap();
        assert_eq!(extended.axes_count(), 4);

        let extended = pairplot(&[x, y], 600, 600)
            .unwrap()
            .theme(crate::render::Theme::dark())
            .add_axes([0.82, 0.82, 0.16, 0.16], crate::core::Plot::new())
            .unwrap();
        assert_eq!(extended.axes_count(), 5);
    }
}
