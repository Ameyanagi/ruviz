# Legend API Design - User-Friendly Chain Methods

## Current Issues
- Inconsistent API (boolean vs Position)
- Limited configuration options
- Verbose enum usage
- No styling control

## Proposed Chain Method API

### Basic Usage (Simple & Intuitive)
```rust
// Simple cases - most common usage
Plot::new()
    .line(&x, &y)
    .legend()                    // Default: top-right, auto-style
    .save("plot.png")?;

Plot::new()
    .line(&x, &y)
    .legend_off()               // Explicit disable
    .save("plot.png")?;
```

### Position Chain Methods (Readable & Discoverable)
```rust
Plot::new()
    .line(&x, &y)
    .legend_top_left()          // Intuitive position methods
    .save("plot.png")?;

Plot::new()
    .line(&x, &y)
    .legend_bottom_right()      // Clear and readable
    .save("plot.png")?;

Plot::new()
    .line(&x, &y)
    .legend_at(0.25, 0.75)      // Custom position with coordinates
    .save("plot.png")?;
```

### Full Configuration Chain (Advanced Control)
```rust
Plot::new()
    .line(&x, &y)
    .legend()
        .position_top_right()
        .font_size(14.0)
        .background_color(Color::new(255, 255, 255))
        .background_alpha(0.9)
        .border_color(Color::new(0, 0, 0))
        .padding(10.0)
        .spacing(15.0)
    .save("plot.png")?;
```

### Smart Auto-Positioning (Intelligent Defaults)
```rust
Plot::new()
    .line(&x, &y)
    .legend_auto()              // Automatically choose best position
    .save("plot.png")?;

Plot::new()
    .line(&x, &y)
    .legend_avoid_data()        // Position to avoid data overlap
    .save("plot.png")?;
```

### Style Presets (Quick Professional Looks)
```rust
Plot::new()
    .line(&x, &y)
    .legend_minimal()           // Clean, minimal style
    .save("plot.png")?;

Plot::new()
    .line(&x, &y)
    .legend_publication()       // Publication-ready style
    .save("plot.png")?;

Plot::new()
    .line(&x, &y)
    .legend_scientific()        // Scientific journal style
    .save("plot.png")?;
```

## Implementation Strategy

### 1. Builder Pattern with Fluent Interface
```rust
pub struct LegendBuilder {
    plot: Plot,
    config: LegendConfig,
}

impl LegendBuilder {
    pub fn position_top_right(mut self) -> Self { ... }
    pub fn font_size(mut self, size: f32) -> Self { ... }
    pub fn background_color(mut self, color: Color) -> Self { ... }
    // ... more configuration methods
}

impl From<LegendBuilder> for Plot {
    fn from(builder: LegendBuilder) -> Self {
        builder.plot
    }
}
```

### 2. Enhanced LegendConfig
```rust
#[derive(Clone, Debug)]
pub struct LegendConfig {
    enabled: bool,
    position: Position,
    font_size: Option<f32>,
    background_color: Option<Color>,
    background_alpha: f32,
    border_color: Option<Color>,
    border_width: f32,
    padding: f32,
    spacing: f32,
    auto_position: bool,
    style_preset: LegendStyle,
}

#[derive(Clone, Debug)]
pub enum LegendStyle {
    Default,
    Minimal,
    Publication,
    Scientific,
    Custom,
}
```

### 3. Convenience Methods on Plot
```rust
impl Plot {
    // Simple methods
    pub fn legend(self) -> LegendBuilder { ... }
    pub fn legend_off(mut self) -> Self { ... }
    
    // Position shortcuts
    pub fn legend_top_left(mut self) -> Self { ... }
    pub fn legend_top_right(mut self) -> Self { ... }
    pub fn legend_bottom_left(mut self) -> Self { ... }
    pub fn legend_bottom_right(mut self) -> Self { ... }
    pub fn legend_at(mut self, x: f32, y: f32) -> Self { ... }
    
    // Auto-positioning
    pub fn legend_auto(mut self) -> Self { ... }
    pub fn legend_avoid_data(mut self) -> Self { ... }
    
    // Style presets
    pub fn legend_minimal(mut self) -> Self { ... }
    pub fn legend_publication(mut self) -> Self { ... }
    pub fn legend_scientific(mut self) -> Self { ... }
}
```

## Benefits

1. **Intuitive**: `legend_top_right()` vs `legend(Position::TopRight)`
2. **Discoverable**: IDE autocomplete shows all legend options
3. **Flexible**: Simple cases are simple, complex cases are possible
4. **Consistent**: All legend methods follow same pattern
5. **Extensible**: Easy to add new styles and options
6. **Backward Compatible**: Can keep existing API during transition

## Migration Path

1. Implement new API alongside existing one
2. Add deprecation warnings to old API
3. Update all examples and tests
4. Remove old API in next major version

## Example Comparisons

### Before (Current)
```rust
use ruviz::core::position::Position;

Plot::new()
    .line(&x, &y)
    .legend(Position::TopRight)  // Verbose, requires import
    .save("plot.png")?;
```

### After (Proposed)
```rust
Plot::new()
    .line(&x, &y)
    .legend_top_right()          // Clear, no imports needed
    .save("plot.png")?;
```

This design makes the API much more user-friendly while maintaining full flexibility for advanced users.