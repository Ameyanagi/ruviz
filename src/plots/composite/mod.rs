//! Composite plot types — **layout math only, not drawable**.
//!
//! [`joint_plot_layout`] and [`compute_pairplot_layout`] return normalised
//! panel rectangles, and [`compute_marginal_histogram`] bins one margin. There
//! is **no renderer and no `Plot` builder method** for either joint plots or
//! pair plots: you get rectangles, and placing content in them is your job.
//!
//! Because nothing here draws, **every field on [`JointPlotConfig`] and
//! [`PairPlotConfig`] that describes appearance is inert** — `rugplot`,
//! `scatter_size`, `color`, `diag_kind` and the rest are values you can read
//! back, not instructions anything obeys. `upper`, `lower` and `diag` are the
//! only config fields any function here reads; `marginal_ratio` and `bins` are
//! values you pass to the free functions yourself. The rest are not
//! individually `#[deprecated]`: the whole module is unwired, and marking
//! twenty setters one at a time would state the same fact twenty times while
//! implying the unmarked ones work. The `config_fields_that_are_inert` test
//! module below pins that claim.
//!
//! Inset/zoom axes are *not* here; those are real and live on the `Plot`
//! builder as `inset_layout`/`inset_anchor`.
//!
//! Wiring this up is Phase 10 of
//! `docs/roadmaps/ruviz-audit-remediation-plan.md`. It is blocked on `add_axes`
//! — arbitrary-rectangle axes — which does not exist: a joint plot is three
//! axes in one figure, and `Plot` models exactly one. `subplots` cannot stand
//! in, because the marginal panels must share the main panel's data limits.

pub mod jointplot;
pub mod pairplot;

pub use jointplot::{
    JointKind, JointPlotConfig, JointPlotLayout, MarginalHistogram, compute_marginal_histogram,
    joint_plot_layout,
};
pub use pairplot::{
    DiagKind, OffDiagKind, PairPlotCell, PairPlotConfig, PairPlotLayout, cell_variable_names,
    compute_pairplot_layout,
};

/// Pins the module doc's claim that the appearance fields here do nothing.
///
/// The point is not to bless the situation. It is that "this setter is inert"
/// is a factual claim about behaviour, and a factual claim in a doc comment
/// with no test behind it is how ~35 silently-inert setters accumulated in the
/// first place. When someone wires these plot types up, these tests fail, and
/// the failure is the reminder to delete the paragraph above.
#[cfg(test)]
mod config_fields_that_are_inert {
    use super::*;

    /// Every appearance knob set to a non-default value.
    fn loud_joint_config() -> JointPlotConfig {
        JointPlotConfig::default()
            .kind(JointKind::Hex)
            .marginal_hist(false)
            .marginal_kde(false)
            .rugplot(true)
            .bins(7)
            .color(crate::render::Color::from_rgb(1, 2, 3))
    }

    #[test]
    fn joint_plot_layout_depends_only_on_the_marginal_ratio() {
        let quiet = joint_plot_layout(JointPlotConfig::default().marginal_ratio);
        let loud = joint_plot_layout(loud_joint_config().marginal_ratio);

        assert_eq!(quiet.main_bounds, loud.main_bounds);
        assert_eq!(quiet.x_marginal_bounds, loud.x_marginal_bounds);
        assert_eq!(quiet.y_marginal_bounds, loud.y_marginal_bounds);

        // ...and it does move when the one live field moves.
        let wider = joint_plot_layout(0.4);
        assert_ne!(quiet.main_bounds, wider.main_bounds);
    }

    #[test]
    fn rugplot_is_doubly_inert() {
        // `rugplot` is a bool nothing reads, on a plot type nothing draws.
        let config = JointPlotConfig::default().rugplot(true);
        assert!(config.rugplot);
        assert_eq!(
            joint_plot_layout(config.marginal_ratio).main_bounds,
            joint_plot_layout(JointPlotConfig::default().marginal_ratio).main_bounds
        );
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
}
