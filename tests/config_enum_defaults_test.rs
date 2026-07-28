//! Every public config enum must agree with the config that uses it.
//!
//! The crate's convention is `SomeConfig::default().field == FieldEnum::default()`.
//! Without a `Default` impl a caller cannot spell "the normal value" for a knob
//! the way they can for its siblings, and a `Default` that disagrees with the
//! config is worse than none: it documents a value the library never produces.

use ruviz::plots::boxplot::{BoxOrientation, BoxPlotConfig, OutlierMethod, WhiskerMethod};
use ruviz::plots::categorical::bar::{BarOrientation, StackedBarConfig};
use ruviz::plots::categorical::strip::{StripConfig, StripOrientation};
use ruviz::plots::categorical::swarm::{SwarmConfig, SwarmOrientation};
use ruviz::plots::composite::jointplot::{JointKind, JointPlotConfig};
use ruviz::plots::composite::pairplot::{DiagKind, OffDiagKind, PairPlotConfig};
use ruviz::plots::continuous::area::{
    AreaConfig, AreaInterpolation, StackBaseline, StackPlotConfig,
};
use ruviz::plots::continuous::hexbin::{HexbinConfig, ReduceFunction};
use ruviz::plots::discrete::{StemConfig, StemMarker, StemOrientation, StepConfig, StepWhere};
use ruviz::plots::distribution::boxen::{BoxenConfig, BoxenOrientation};
use ruviz::plots::distribution::violin::{BandwidthMethod, Orientation, ViolinConfig, ViolinScale};
use ruviz::plots::error::errorbar::{ErrorBarConfig, ErrorLineStyle};
use ruviz::plots::hierarchical::dendrogram::{DendrogramConfig, DendrogramOrientation};
use ruviz::plots::histogram::{BinMethod, HistogramConfig};

/// `matches!` rather than `assert_eq!` so this file does not force `PartialEq`
/// onto enums that have no reason to carry it.
macro_rules! assert_default_matches {
    ($config:expr, $field:ident, $enum:ty, $variant:pat) => {{
        assert!(
            matches!($config.$field, $variant),
            concat!(
                stringify!($enum),
                ": config default disagrees with the documented variant"
            )
        );
        assert!(
            matches!(<$enum>::default(), $variant),
            concat!(
                stringify!($enum),
                "::default() disagrees with the config default"
            )
        );
    }};
}

#[test]
fn histogram_bin_method_default_matches_config() {
    assert_default_matches!(
        HistogramConfig::default(),
        bin_method,
        BinMethod,
        BinMethod::Sturges
    );
}

#[test]
fn boxplot_enum_defaults_match_config() {
    let config = BoxPlotConfig::default();
    assert_default_matches!(config, outlier_method, OutlierMethod, OutlierMethod::IQR);
    assert_default_matches!(
        config,
        orientation,
        BoxOrientation,
        BoxOrientation::Vertical
    );
    assert_default_matches!(config, whisker_method, WhiskerMethod, WhiskerMethod::Tukey);
}

#[test]
// `ViolinConfig::scale` is deprecated (inert) but still present and still
// carries a value, so the `ViolinScale::default()` invariant still has to hold
// for it. The enum itself is not deprecated.
#[allow(deprecated)]
fn violin_enum_defaults_match_config() {
    let config = ViolinConfig::default();
    assert_default_matches!(config, bandwidth, BandwidthMethod, BandwidthMethod::Scott);
    assert_default_matches!(config, scale, ViolinScale, ViolinScale::Width);
    assert_default_matches!(config, orientation, Orientation, Orientation::Vertical);
}

#[test]
fn boxen_orientation_default_matches_config() {
    assert_default_matches!(
        BoxenConfig::default(),
        orient,
        BoxenOrientation,
        BoxenOrientation::Vertical
    );
}

#[test]
fn step_and_stem_enum_defaults_match_config() {
    assert_default_matches!(StepConfig::default(), where_step, StepWhere, StepWhere::Pre);
    let stem = StemConfig::default();
    assert_default_matches!(stem, marker, StemMarker, StemMarker::Circle);
    assert_default_matches!(
        stem,
        orientation,
        StemOrientation,
        StemOrientation::Vertical
    );
}

#[test]
fn hexbin_reduce_default_matches_config() {
    assert_default_matches!(
        HexbinConfig::default(),
        reduce_fn,
        ReduceFunction,
        ReduceFunction::Count
    );
}

#[test]
fn area_enum_defaults_match_config() {
    assert_default_matches!(
        AreaConfig::default(),
        interpolation,
        AreaInterpolation,
        AreaInterpolation::Linear
    );
    assert_default_matches!(
        StackPlotConfig::default(),
        baseline,
        StackBaseline,
        StackBaseline::Zero
    );
}

#[test]
fn error_bar_line_style_default_matches_config() {
    assert_default_matches!(
        ErrorBarConfig::default(),
        line_style,
        ErrorLineStyle,
        ErrorLineStyle::Solid
    );
}

#[test]
fn categorical_enum_defaults_match_config() {
    assert_default_matches!(
        StackedBarConfig::default(),
        orientation,
        BarOrientation,
        BarOrientation::Vertical
    );
    assert_default_matches!(
        StripConfig::default(),
        orientation,
        StripOrientation,
        StripOrientation::Vertical
    );
    assert_default_matches!(
        SwarmConfig::default(),
        orientation,
        SwarmOrientation,
        SwarmOrientation::Vertical
    );
}

#[test]
fn composite_enum_defaults_match_config() {
    assert_default_matches!(
        JointPlotConfig::default(),
        kind,
        JointKind,
        JointKind::Scatter
    );
    let pair = PairPlotConfig::default();
    assert_default_matches!(pair, diag_kind, DiagKind, DiagKind::Hist);
    assert_default_matches!(pair, off_diag_kind, OffDiagKind, OffDiagKind::Scatter);
}

#[test]
fn dendrogram_orientation_default_matches_config() {
    assert_default_matches!(
        DendrogramConfig::default(),
        orientation,
        DendrogramOrientation,
        DendrogramOrientation::Top
    );
}
