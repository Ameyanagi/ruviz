/// Line style enumeration for plot lines and borders
///
/// Defines different visual styles for drawing lines in plots.
///
/// # Example
///
/// ```rust,no_run
/// use ruviz::prelude::*;
/// use ruviz::render::LineStyle;
///
/// let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
/// let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
///
/// Plot::new()
///     .line(&x, &y)
///     .style(LineStyle::Dashed)
///     .end_series()
///     .save("dashed_line.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// ![Line styles](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/line_styles.png)
#[derive(Debug, Clone, PartialEq)]
pub enum LineStyle {
    /// Solid continuous line (default)
    Solid,
    /// Dashed line with equal dash and gap lengths
    Dashed,
    /// Dotted line with small dots
    Dotted,
    /// Dash-dot pattern (long dash, short gap, dot, short gap)
    DashDot,
    /// Dash-dot-dot pattern (long dash, short gap, dot, short gap, dot, short gap)
    DashDotDot,
    /// Custom pattern defined by dash array
    Custom(Vec<f32>),
}

impl LineStyle {
    /// Convert to dash array for tiny-skia stroke
    /// Returns None for solid lines, Some(dash_array) for patterned lines
    pub fn to_dash_array(&self) -> Option<Vec<f32>> {
        match self {
            LineStyle::Solid => None,
            LineStyle::Dashed => Some(vec![5.0, 5.0]),
            LineStyle::Dotted => Some(vec![1.0, 2.0]),
            LineStyle::DashDot => Some(vec![8.0, 3.0, 1.0, 3.0]),
            LineStyle::DashDotDot => Some(vec![8.0, 3.0, 1.0, 3.0, 1.0, 3.0]),
            LineStyle::Custom(pattern) => {
                if pattern.is_empty() {
                    None
                } else {
                    Some(pattern.clone())
                }
            }
        }
    }

    /// Create a custom line style from dash pattern
    ///
    /// Pattern should alternate between dash length and gap length.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// // Custom dash-dot-dot pattern: long dash, short gap, dot, gap, dot, gap
    /// let custom_style = LineStyle::Custom(vec![10.0, 3.0, 2.0, 3.0, 2.0, 3.0]);
    ///
    /// Plot::new()
    ///     .line(&[1.0, 2.0, 3.0], &[1.0, 4.0, 9.0])
    ///     .style(custom_style)
    ///     .end_series()
    ///     .save("custom_line.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn custom<I>(pattern: I) -> Self
    where
        I: IntoIterator<Item = f32>,
    {
        let pattern: Vec<f32> = pattern
            .into_iter()
            .map(|x| x.abs()) // Ensure positive values
            .filter(|&x| x > 0.0) // Remove zero values
            .collect();

        LineStyle::Custom(pattern)
    }

    /// Get a descriptive name for the line style
    pub fn name(&self) -> &'static str {
        match self {
            LineStyle::Solid => "solid",
            LineStyle::Dashed => "dashed",
            LineStyle::Dotted => "dotted",
            LineStyle::DashDot => "dash-dot",
            LineStyle::DashDotDot => "dash-dot-dot",
            LineStyle::Custom(_) => "custom",
        }
    }

    /// Check if this is a solid line
    pub fn is_solid(&self) -> bool {
        matches!(self, LineStyle::Solid)
    }

    /// Check if this is a patterned (non-solid) line
    pub fn is_patterned(&self) -> bool {
        !self.is_solid()
    }

    /// Get pattern length (total length of one complete pattern cycle)
    pub fn pattern_length(&self) -> f32 {
        match self.to_dash_array() {
            None => 0.0, // Solid line has no pattern
            Some(pattern) => pattern.iter().sum(),
        }
    }

    /// Scale the pattern by a factor (useful for different line widths)
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::render::LineStyle;
    ///
    /// let dashed = LineStyle::Dashed;
    /// let wide_dashed = dashed.scaled(2.0);
    ///
    /// // Doubled pattern: [10.0, 10.0] instead of [5.0, 5.0]
    /// assert_eq!(wide_dashed.to_dash_array(), Some(vec![10.0, 10.0]));
    /// ```
    pub fn scaled(&self, factor: f32) -> Self {
        if factor <= 0.0 {
            return self.clone();
        }

        match self {
            LineStyle::Custom(pattern) => {
                LineStyle::Custom(pattern.iter().map(|&x| x * factor).collect())
            }
            _ => {
                // For predefined patterns, create scaled custom version
                if let Some(pattern) = self.to_dash_array() {
                    LineStyle::Custom(pattern.iter().map(|&x| x * factor).collect())
                } else {
                    self.clone() // Solid line unchanged
                }
            }
        }
    }
}

impl Default for LineStyle {
    /// Default line style is solid
    fn default() -> Self {
        LineStyle::Solid
    }
}

impl std::fmt::Display for LineStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LineStyle::Solid => write!(f, "solid"),
            LineStyle::Dashed => write!(f, "dashed"),
            LineStyle::Dotted => write!(f, "dotted"),
            LineStyle::DashDot => write!(f, "dash-dot"),
            LineStyle::DashDotDot => write!(f, "dash-dot-dot"),
            LineStyle::Custom(pattern) => {
                write!(f, "custom(")?;
                for (i, &value) in pattern.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:.1}", value)?;
                }
                write!(f, ")")
            }
        }
    }
}

/// Marker style for scatter plots and data points
///
/// # Example
///
/// ```rust,no_run
/// use ruviz::prelude::*;
/// use ruviz::render::MarkerStyle;
///
/// let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let y = vec![1.0, 4.0, 2.0, 5.0, 3.0];
///
/// Plot::new()
///     .scatter(&x, &y)
///     .marker(MarkerStyle::Star)
///     .marker_size(12.0)
///     .end_series()
///     .save("star_markers.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// ![Marker styles](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/marker_styles.png)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkerStyle {
    /// Filled circle (default)
    Circle,
    /// Filled square
    Square,
    /// Filled triangle pointing up
    Triangle,
    /// Filled triangle pointing down
    TriangleDown,
    /// Filled diamond
    Diamond,
    /// Plus sign (+)
    Plus,
    /// X mark
    Cross,
    /// Filled star
    Star,
    /// Hollow circle
    CircleOpen,
    /// Hollow square
    SquareOpen,
    /// Hollow triangle
    TriangleOpen,
    /// Hollow diamond
    DiamondOpen,
}

impl MarkerStyle {
    /// Get a descriptive name for the marker style
    pub fn name(&self) -> &'static str {
        match self {
            MarkerStyle::Circle => "circle",
            MarkerStyle::Square => "square",
            MarkerStyle::Triangle => "triangle",
            MarkerStyle::TriangleDown => "triangle-down",
            MarkerStyle::Diamond => "diamond",
            MarkerStyle::Plus => "plus",
            MarkerStyle::Cross => "cross",
            MarkerStyle::Star => "star",
            MarkerStyle::CircleOpen => "circle-open",
            MarkerStyle::SquareOpen => "square-open",
            MarkerStyle::TriangleOpen => "triangle-open",
            MarkerStyle::DiamondOpen => "diamond-open",
        }
    }

    /// Check if this is a filled marker
    pub fn is_filled(&self) -> bool {
        matches!(
            self,
            MarkerStyle::Circle
                | MarkerStyle::Square
                | MarkerStyle::Triangle
                | MarkerStyle::TriangleDown
                | MarkerStyle::Diamond
                | MarkerStyle::Star
        )
    }

    /// Check if this is a hollow/open marker
    pub fn is_hollow(&self) -> bool {
        matches!(
            self,
            MarkerStyle::CircleOpen
                | MarkerStyle::SquareOpen
                | MarkerStyle::TriangleOpen
                | MarkerStyle::DiamondOpen
        )
    }

    /// Check if this is a line-based marker (plus, cross)
    pub fn is_line_based(&self) -> bool {
        matches!(self, MarkerStyle::Plus | MarkerStyle::Cross)
    }
}

impl Default for MarkerStyle {
    /// Default marker style is filled circle
    fn default() -> Self {
        MarkerStyle::Circle
    }
}

impl std::fmt::Display for MarkerStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_style_dash_arrays() {
        assert_eq!(LineStyle::Solid.to_dash_array(), None);
        assert_eq!(LineStyle::Dashed.to_dash_array(), Some(vec![5.0, 5.0]));
        assert_eq!(LineStyle::Dotted.to_dash_array(), Some(vec![1.0, 2.0]));
        assert_eq!(
            LineStyle::DashDot.to_dash_array(),
            Some(vec![8.0, 3.0, 1.0, 3.0])
        );
    }

    #[test]
    fn test_custom_line_style() {
        let custom = LineStyle::custom(vec![4.0, 2.0, 1.0, 2.0]);
        assert_eq!(custom.to_dash_array(), Some(vec![4.0, 2.0, 1.0, 2.0]));

        // Test filtering of zero values and abs() of negative values
        // -2.0 becomes 2.0 (abs), 0.0 is filtered out
        let custom_filtered = LineStyle::custom(vec![4.0, 0.0, -2.0, 3.0]);
        assert_eq!(custom_filtered.to_dash_array(), Some(vec![4.0, 2.0, 3.0]));

        // Test empty pattern
        let empty = LineStyle::custom(Vec::<f32>::new());
        assert_eq!(empty.to_dash_array(), None);
    }

    #[test]
    fn test_line_style_properties() {
        assert!(LineStyle::Solid.is_solid());
        assert!(!LineStyle::Dashed.is_solid());
        assert!(LineStyle::Dashed.is_patterned());
        assert!(!LineStyle::Solid.is_patterned());
    }

    #[test]
    fn test_pattern_length() {
        assert_eq!(LineStyle::Solid.pattern_length(), 0.0);
        assert_eq!(LineStyle::Dashed.pattern_length(), 10.0); // 5.0 + 5.0
        assert_eq!(LineStyle::DashDot.pattern_length(), 15.0); // 8.0 + 3.0 + 1.0 + 3.0
    }

    #[test]
    fn test_line_style_scaling() {
        let scaled_dashed = LineStyle::Dashed.scaled(2.0);
        assert_eq!(scaled_dashed.to_dash_array(), Some(vec![10.0, 10.0]));

        let scaled_solid = LineStyle::Solid.scaled(2.0);
        assert_eq!(scaled_solid, LineStyle::Solid); // Solid unchanged

        let custom = LineStyle::custom(vec![2.0, 1.0]);
        let scaled_custom = custom.scaled(3.0);
        assert_eq!(scaled_custom.to_dash_array(), Some(vec![6.0, 3.0]));
    }

    #[test]
    fn test_line_style_names() {
        assert_eq!(LineStyle::Solid.name(), "solid");
        assert_eq!(LineStyle::DashDot.name(), "dash-dot");
        assert_eq!(LineStyle::custom(vec![1.0, 2.0]).name(), "custom");
    }

    #[test]
    fn test_marker_style_properties() {
        assert!(MarkerStyle::Circle.is_filled());
        assert!(!MarkerStyle::CircleOpen.is_filled());
        assert!(MarkerStyle::CircleOpen.is_hollow());
        assert!(!MarkerStyle::Circle.is_hollow());
        assert!(MarkerStyle::Plus.is_line_based());
        assert!(!MarkerStyle::Circle.is_line_based());
    }

    #[test]
    fn test_marker_style_names() {
        assert_eq!(MarkerStyle::Circle.name(), "circle");
        assert_eq!(MarkerStyle::TriangleDown.name(), "triangle-down");
        assert_eq!(MarkerStyle::CircleOpen.name(), "circle-open");
    }

    #[test]
    fn test_defaults() {
        assert_eq!(LineStyle::default(), LineStyle::Solid);
        assert_eq!(MarkerStyle::default(), MarkerStyle::Circle);
    }

    #[test]
    fn test_display() {
        assert_eq!(LineStyle::Solid.to_string(), "solid");
        assert_eq!(
            LineStyle::custom(vec![1.0, 2.5]).to_string(),
            "custom(1.0, 2.5)"
        );
        assert_eq!(MarkerStyle::Circle.to_string(), "circle");
    }
}
