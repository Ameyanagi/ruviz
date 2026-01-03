use ruviz::core::plot::Plot;
use ruviz::render::Color;

#[test]
fn test_default_tick_system() {
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y_data = vec![0.0, 2.0, 4.0, 6.0, 8.0];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Default Tick System")
        .xlabel("X Axis")
        .ylabel("Y Axis")
        .line(&x_data, &y_data)
        .save("test_output/default_ticks.png");

    assert!(result.is_ok(), "Default tick system should work");
}

#[test]
fn test_ticks_inside_direction() {
    let x_data = vec![0.0, 5.0, 10.0, 15.0, 20.0];
    let y_data = vec![0.0, 25.0, 50.0, 75.0, 100.0];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Ticks Inside (Default)")
        .xlabel("Time (seconds)")
        .ylabel("Value")
        .tick_direction_inside() // Default behavior, ticks point inward
        .line(&x_data, &y_data)
        .save("test_output/ticks_inside.png");

    assert!(result.is_ok(), "Inside tick direction should work");
}

#[test]
fn test_ticks_outside_direction() {
    let x_data = vec![0.0, 5.0, 10.0, 15.0, 20.0];
    let y_data = vec![0.0, 25.0, 50.0, 75.0, 100.0];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Ticks Outside")
        .xlabel("Time (seconds)")
        .ylabel("Value")
        .tick_direction_outside() // Ticks point outward
        .line(&x_data, &y_data)
        .save("test_output/ticks_outside.png");

    assert!(result.is_ok(), "Outside tick direction should work");
}

#[test]
fn test_major_minor_ticks() {
    let x_data = vec![0.0, 2.5, 5.0, 7.5, 10.0];
    let y_data = vec![0.0, 12.5, 25.0, 37.5, 50.0];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Major and Minor Ticks")
        .xlabel("Position")
        .ylabel("Measurement")
        .major_ticks(5) // 5 major ticks
        .minor_ticks(4) // 4 minor ticks between each major tick
        .line(&x_data, &y_data)
        .save("test_output/major_minor_ticks.png");

    assert!(result.is_ok(), "Major and minor ticks should work");
}

#[test]
fn test_custom_tick_configuration() {
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![0.0, 4.0, 8.0, 12.0, 16.0, 20.0];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Custom Tick Configuration")
        .xlabel("X Values")
        .ylabel("Y Values")
        .tick_direction_outside()
        .major_ticks_x(6) // 6 major ticks on X axis
        .minor_ticks_x(2) // 2 minor ticks between major ticks on X
        .major_ticks_y(5) // 5 major ticks on Y axis
        .minor_ticks_y(3) // 3 minor ticks between major ticks on Y
        .line(&x_data, &y_data)
        .save("test_output/custom_tick_config.png");

    assert!(result.is_ok(), "Custom tick configuration should work");
}

#[test]
fn test_grid_with_major_ticks_only() {
    let x_data = vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0];
    let y_data = vec![0.0, 10.0, 20.0, 30.0, 40.0, 50.0];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Grid with Major Ticks Only")
        .xlabel("X Axis")
        .ylabel("Y Axis")
        .grid(true)
        .grid_major_only() // Grid only at major ticks
        .major_ticks(6)
        .minor_ticks(4)
        .line(&x_data, &y_data)
        .save("test_output/grid_major_only.png");

    assert!(result.is_ok(), "Grid with major ticks only should work");
}

#[test]
fn test_grid_with_minor_ticks_only() {
    let x_data = vec![0.0, 1.5, 3.0, 4.5, 6.0];
    let y_data = vec![0.0, 7.5, 15.0, 22.5, 30.0];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Grid with Minor Ticks Only")
        .xlabel("X Axis")
        .ylabel("Y Axis")
        .grid(true)
        .grid_minor_only() // Grid only at minor ticks
        .major_ticks(4)
        .minor_ticks(3)
        .line(&x_data, &y_data)
        .save("test_output/grid_minor_only.png");

    assert!(result.is_ok(), "Grid with minor ticks only should work");
}

#[test]
fn test_grid_with_both_major_and_minor() {
    let x_data = vec![0.0, 2.0, 4.0, 6.0, 8.0];
    let y_data = vec![0.0, 16.0, 32.0, 48.0, 64.0];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Grid with Both Major and Minor Ticks")
        .xlabel("Time")
        .ylabel("Value")
        .grid(true)
        .grid_both() // Grid at both major and minor ticks
        .major_ticks(5)
        .minor_ticks(2)
        .line(&x_data, &y_data)
        .save("test_output/grid_both_ticks.png");

    assert!(
        result.is_ok(),
        "Grid with both major and minor ticks should work"
    );
}

#[test]
fn test_tick_labels_positioning() {
    let x_data = vec![0.0, 10.0, 20.0, 30.0, 40.0, 50.0];
    let y_data = vec![0.0, 100.0, 200.0, 300.0, 400.0, 500.0];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Tick Labels Properly Positioned")
        .xlabel("Input Values")
        .ylabel("Output Values")
        .tick_direction_outside()
        .major_ticks(6)
        .minor_ticks(4) // Should not have labels
        .line(&x_data, &y_data)
        .save("test_output/tick_labels_positioning.png");

    assert!(result.is_ok(), "Tick labels should be properly positioned");
}

#[test]
fn test_scatter_with_improved_ticks() {
    let x_data = vec![1.5, 3.2, 5.7, 8.1, 10.4];
    let y_data = vec![2.3, 6.7, 12.1, 18.9, 25.6];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Scatter Plot with Improved Ticks")
        .xlabel("X Position")
        .ylabel("Y Position")
        .tick_direction_inside()
        .major_ticks(5)
        .minor_ticks(2)
        .grid(true)
        .grid_major_only()
        .scatter(&x_data, &y_data)
        .color(Color::new(255, 0, 0)) // Red dots
        .marker_size(8.0)
        .save("test_output/scatter_improved_ticks.png");

    assert!(
        result.is_ok(),
        "Scatter plot with improved ticks should work"
    );
}

#[test]
fn test_tight_layout_like_matplotlib() {
    let x_data = vec![0.0, 5.0, 10.0, 15.0, 20.0, 25.0];
    let y_data = vec![0.0, 100.0, 200.0, 300.0, 400.0, 500.0];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Tight Layout Test - Auto Margin Adjustment")
        .xlabel("Very Long X Axis Label That Should Not Be Cut Off")
        .ylabel("Very Long Y Axis Label That Should Be Fully Visible")
        .tight_layout(true) // Automatically adjust margins
        .tick_direction_outside()
        .major_ticks(6)
        .minor_ticks(4)
        .grid(true)
        .grid_both()
        .line(&x_data, &y_data)
        .save("test_output/tight_layout_test.png");

    assert!(result.is_ok(), "Tight layout should work like matplotlib");
}
