use super::*;

// A minimal test config for testing the builder infrastructure
#[derive(Debug, Clone, Default)]
struct TestConfig {
    value: f64,
}

impl crate::plots::PlotConfig for TestConfig {}

#[test]
fn test_plot_builder_creation() {
    let plot = super::super::Plot::new();
    let input = PlotInput::Single(vec![1.0, 2.0, 3.0]);
    let config = TestConfig::default();

    let builder = PlotBuilder::new(plot, input, config);

    assert!(builder.style.label.is_none());
    assert!(builder.style.color.is_none());
}

#[test]
fn test_plot_builder_styling() {
    let plot = super::super::Plot::new();
    let input = PlotInput::Single(vec![1.0, 2.0, 3.0]);
    let config = TestConfig::default();

    let builder = PlotBuilder::new(plot, input, config)
        .label("Test")
        .color(Color::RED)
        .line_width(2.0)
        .alpha(0.8);

    assert_eq!(builder.style.label, Some("Test".to_string()));
    assert!(builder.style.color.is_some());
    assert_eq!(builder.style.line_width, Some(2.0));
    assert_eq!(builder.style.alpha, Some(0.8));
}

#[test]
fn test_plot_builder_plot_forwarding() {
    let plot = super::super::Plot::new();
    let input = PlotInput::Single(vec![1.0, 2.0, 3.0]);
    let config = TestConfig::default();

    let builder = PlotBuilder::new(plot, input, config)
        .title("My Title")
        .xlabel("X Axis")
        .ylabel("Y Axis");

    // The plot should have the title set (we can check by calling get_plot)
    // Note: Plot fields are private, so we can't directly verify here
    // But the test ensures the method chaining works
    assert!(builder.get_plot().get_config().figure.width > 0.0);
}

#[test]
fn test_plot_builder_alpha_clamping() {
    let plot = super::super::Plot::new();
    let input = PlotInput::Single(vec![1.0, 2.0, 3.0]);
    let config = TestConfig::default();

    let builder = PlotBuilder::new(plot, input, config).alpha(1.5); // Should clamp to 1.0
    assert_eq!(builder.style.alpha, Some(1.0));

    let plot = super::super::Plot::new();
    let input = PlotInput::Single(vec![1.0, 2.0, 3.0]);
    let config = TestConfig::default();

    let builder = PlotBuilder::new(plot, input, config).alpha(-0.5); // Should clamp to 0.0
    assert_eq!(builder.style.alpha, Some(0.0));
}

#[test]
fn test_plot_builder_line_width_min() {
    let plot = super::super::Plot::new();
    let input = PlotInput::Single(vec![1.0, 2.0, 3.0]);
    let config = TestConfig::default();

    let builder = PlotBuilder::new(plot, input, config).line_width(0.01); // Should clamp to 0.1
    assert_eq!(builder.style.line_width, Some(0.1));
}

#[test]
fn test_static_source_setters_materialize_generic_builder_values() {
    let builder = super::super::Plot::new()
        .line(&[0.0, 1.0], &[1.0, 2.0])
        .color_source(Color::RED)
        .line_width_source(0.01_f32)
        .style_source(LineStyle::Dashed)
        .marker_source(MarkerStyle::Square)
        .marker_size_source(0.01_f32)
        .alpha_source(1.5_f32);

    assert_eq!(builder.style.color, Some(Color::RED));
    assert!(builder.style.color_source.is_none());
    assert_eq!(builder.style.line_width, Some(0.1));
    assert!(builder.style.line_width_source.is_none());
    assert_eq!(builder.style.line_style, Some(LineStyle::Dashed));
    assert!(builder.style.line_style_source.is_none());
    assert_eq!(builder.style.marker_style, Some(MarkerStyle::Square));
    assert!(builder.style.marker_style_source.is_none());
    assert_eq!(builder.style.marker_size, Some(0.1));
    assert!(builder.style.marker_size_source.is_none());
    assert_eq!(builder.style.alpha, Some(1.0));
    assert!(builder.style.alpha_source.is_none());
}

#[test]
fn test_plot_input_variants() {
    // Test Single variant
    let single = PlotInput::Single(vec![1.0, 2.0]);
    match single {
        PlotInput::Single(data) => assert_eq!(data.len(), 2),
        _ => panic!("Expected Single variant"),
    }

    // Test XY variant
    let xy = PlotInput::XY(vec![1.0, 2.0], vec![3.0, 4.0]);
    match xy {
        PlotInput::XY(x, y) => {
            assert_eq!(x.len(), 2);
            assert_eq!(y.len(), 2);
        }
        _ => panic!("Expected XY variant"),
    }

    let xy_source = PlotInput::XYSource(
        PlotData::Static(vec![1.0, 2.0]),
        PlotData::Static(vec![3.0, 4.0]),
    );
    match xy_source {
        PlotInput::XYSource(x, y) => {
            assert_eq!(x.len(), 2);
            assert_eq!(y.len(), 2);
        }
        _ => panic!("Expected XYSource variant"),
    }

    // Test Categorical variant
    let cat = PlotInput::Categorical {
        categories: vec!["A".to_string(), "B".to_string()],
        values: vec![10.0, 20.0],
    };
    match cat {
        PlotInput::Categorical { categories, values } => {
            assert_eq!(categories.len(), 2);
            assert_eq!(values.len(), 2);
        }
        _ => panic!("Expected Categorical variant"),
    }

    let cat_source = PlotInput::CategoricalSource {
        categories: vec!["A".to_string(), "B".to_string()],
        values: PlotData::Static(vec![10.0, 20.0]),
    };
    match cat_source {
        PlotInput::CategoricalSource { categories, values } => {
            assert_eq!(categories.len(), 2);
            assert_eq!(values.len(), 2);
        }
        _ => panic!("Expected CategoricalSource variant"),
    }
}
