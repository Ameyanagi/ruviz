# Matplotlib/Seaborn Styling Research

This document captures the styling conventions and defaults from matplotlib and seaborn that inform ruviz's design.

## Overview

The goal is to achieve visual parity with matplotlib/seaborn output while maintaining a clean Rust API. This research covers default values, color handling, and common styling patterns.

## Core Styling Constants

### Line Widths (matplotlib rcParams)

```python
# matplotlib defaults (from matplotlib.rcParams)
lines.linewidth: 1.5        # Main data lines
patch.linewidth: 0.8        # Edges on filled shapes (bars, boxes)
axes.linewidth: 0.8         # Axis spines
grid.linewidth: 0.5         # Grid lines
legend.framealpha: 0.8      # Legend frame transparency
```

**ruviz implementation:**
- `StyleResolver::line_width()` defaults to 1.5
- `StyleResolver::edge_width()` defaults to 0.8 (for patch-like shapes)
- Grid lines use 0.5pt width

### Fill Alpha Values

```python
# Common seaborn defaults
histogram.alpha: 0.7        # Overlapping histograms
kde.fill_alpha: 0.25        # Filled KDE curves
violin.alpha: 0.7           # Violin fills
boxplot.alpha: 0.7          # Box fills
```

**ruviz implementation:**
- `StyleResolver::fill_alpha()` defaults to 0.7 for most fill types
- KDE fill uses lower alpha (0.25) for overlay visibility

### Font Sizes

```python
# matplotlib figure defaults
font.size: 10               # Base font size
axes.titlesize: 12          # Axis title
axes.labelsize: 10          # Axis labels
legend.fontsize: 10         # Legend text
xtick.labelsize: 8          # Tick labels
ytick.labelsize: 8
```

## Edge Color Derivation

Matplotlib/seaborn use several strategies for edge colors:

### 1. Darker Fill Color (most common)

For filled shapes like histogram bars and box plots:
```python
edge_color = fill_color.darken(0.3)  # 30% darker
```

**ruviz implementation:**
- `Color::darken(factor)` using HSL color space
- `StyleResolver::edge_color()` applies 30% darkening by default

### 2. Explicit Black Edges

Some elements use pure black edges:
- Box plot whiskers and caps
- Violin plot inner lines (quartiles, median)

### 3. Theme-Based Edges

Dark themes may use lighter edges:
```python
if is_dark_theme:
    edge_color = fill_color.lighten(0.2)
```

## SpineConfig (Axis Borders)

Seaborn's `despine()` function is a key styling feature:

```python
import seaborn as sns

# Default despine - removes top and right spines
sns.despine()

# Full despine - removes all spines
sns.despine(left=True, bottom=True)

# Offset spines - moves spines away from data
sns.despine(offset=10)  # 10 points away from data
```

**ruviz implementation:**
- `SpineConfig::despine()` - hides top/right spines
- `SpineConfig::minimal()` - alias for despine
- `SpineConfig::none()` - hides all spines
- `offset` field for spine positioning

## Plot-Specific Styling

### Histogram

```python
# matplotlib.pyplot.hist defaults
histtype: 'bar'             # Standard filled bars
align: 'mid'                # Bars centered on bin edges
rwidth: 0.8                 # Relative bar width (80% of bin)
alpha: 0.7                  # Fill transparency
edgecolor: 'black'          # Or derived from fill
linewidth: 0.8              # Edge line width
```

**ruviz HistogramConfig:**
- `bar_width: 0.8` (relative width)
- `fill_alpha: 0.7`
- `edge_width: 0.8`
- `edge_color: None` (auto-derived)

### Box Plot

```python
# matplotlib boxplot defaults
patch.linewidth: 0.8        # Box edges
flierprops.markersize: 6    # Outlier size
whiskerprops.linestyle: '-' # Solid whiskers
medianprops.linewidth: 1.5  # Median line
showfliers: True            # Show outliers
showmeans: False            # Don't show means
```

**ruviz BoxPlotConfig:**
- Box fill with derived edge color
- Median line at 1.5pt width
- Whiskers using solid lines
- Outliers as small circles

### Violin Plot

```python
# seaborn violin defaults
inner: 'box'                # Box plot inside
cut: 2                      # Extend density 2 bandwidths
scale: 'width'              # Scale violins by width
bw_method: 'scott'          # Bandwidth selection
alpha: 0.7                  # Fill transparency
linewidth: 1.0              # Outline width
```

**ruviz ViolinConfig:**
- `show_box: true` for inner box
- `scale: ViolinScale::Width`
- `bandwidth: BandwidthMethod::Scott`
- `fill_alpha: 0.7`

### KDE Plot

```python
# seaborn kdeplot defaults
bw_method: 'scott'          # Bandwidth selection
fill: False                 # Just line by default
linewidth: 1.5              # Line width
common_norm: True           # Normalize across groups
```

**ruviz KdeConfig:**
- `bandwidth: None` (uses Scott's rule)
- `fill: false` (line-only default)
- Line width from theme (1.5)

### Heatmap

```python
# seaborn heatmap defaults
cmap: 'rocket'              # Color map
annot: False                # No annotations
fmt: '.2g'                  # Annotation format
linewidths: 0               # No cell borders
linecolor: 'white'          # Border color if shown
square: False               # Don't force square
cbar: True                  # Show colorbar
```

**ruviz HeatmapConfig:**
- Colormap support via theme
- Optional cell grid lines
- Optional value annotations

### Contour Plot

```python
# matplotlib contour defaults
levels: 10                  # Number of contour levels
linewidths: 1.5             # Contour line width
alpha: 1.0                  # Full opacity
cmap: None                  # Use default colormap
filled: False               # Line contours by default
```

### Radar/Spider Chart

```python
# No direct matplotlib support, common conventions:
fill_alpha: 0.25            # Light fill
linewidth: 2.0              # Thicker perimeter
marker: 'o'                 # Dots at vertices
grid_alpha: 0.3             # Light grid
```

## Color Palettes

### Default Palette (matplotlib tab10)

```python
colors = [
    '#1f77b4',  # Blue
    '#ff7f0e',  # Orange
    '#2ca02c',  # Green
    '#d62728',  # Red
    '#9467bd',  # Purple
    '#8c564b',  # Brown
    '#e377c2',  # Pink
    '#7f7f7f',  # Gray
    '#bcbd22',  # Olive
    '#17becf',  # Cyan
]
```

### Seaborn Palettes

```python
# Common seaborn palettes
deep = ['#4C72B0', '#DD8452', '#55A868', '#C44E52', '#8172B3', '#937860']
muted = ['#4878D0', '#EE854A', '#6ACC64', '#D65F5F', '#956CB4', '#8C613C']
pastel = ['#A1C9F4', '#FFB482', '#8DE5A1', '#FF9F9B', '#D0BBFF', '#DEBB9B']
dark = ['#001C7F', '#B1400D', '#12711C', '#8C0800', '#591E71', '#592F0D']
```

## Grid Styling

```python
# matplotlib grid defaults
grid.color: '#b0b0b0'       # Light gray
grid.alpha: 0.5             # 50% transparent
grid.linestyle: '-'         # Solid
grid.linewidth: 0.5         # Thin

# seaborn style modifications
whitegrid:
    axes.grid: True
    axes.facecolor: 'white'
    grid.color: '.8'        # 80% gray

darkgrid:
    axes.facecolor: '#EAEAF2'
    grid.color: 'white'
```

## Theme Variations

### Light Theme (matplotlib default)

```python
figure.facecolor: 'white'
axes.facecolor: 'white'
axes.edgecolor: 'black'
text.color: 'black'
xtick.color: 'black'
ytick.color: 'black'
```

### Dark Theme

```python
figure.facecolor: '#1C1C1C'
axes.facecolor: '#2D2D2D'
axes.edgecolor: '#CCCCCC'
text.color: '#CCCCCC'
xtick.color: '#CCCCCC'
ytick.color: '#CCCCCC'
```

### Seaborn Styles

- **white**: No grid, minimal spines
- **whitegrid**: White background with gray grid
- **darkgrid**: Gray background with white grid
- **ticks**: Ticks on all spines, no grid

## Implementation Summary

### StyleResolver Usage

```rust
use ruviz::core::StyleResolver;

let resolver = StyleResolver::new(theme);

// Get theme-aware line width
let line_width = resolver.line_width(config.line_width);

// Get fill alpha
let alpha = resolver.fill_alpha(config.alpha);

// Derive edge color from fill
let edge = resolver.edge_color(fill_color, config.edge_color);
```

### SpineConfig Usage

```rust
use ruviz::core::SpineConfig;

// Seaborn-style despine
let spines = SpineConfig::despine();

// Minimal (no top/right)
let minimal = SpineConfig::minimal();

// Custom
let custom = SpineConfig {
    left: true,
    bottom: true,
    right: false,
    top: false,
    offset: 5.0,  // Move spines 5pt from data
};
```

### StyledShape Usage

```rust
use ruviz::plots::StyledShape;

// Implement for custom shapes
impl StyledShape for MyBar {
    fn fill_color(&self) -> Color { self.fill }
    fn edge_color(&self) -> Option<Color> { self.edge }
    fn edge_width(&self) -> f32 { 0.8 }
    fn alpha(&self) -> f32 { 0.7 }
}

// Auto-derive edge color
let edge = bar.resolved_edge_color();
```

## References

- [matplotlib rcParams](https://matplotlib.org/stable/api/matplotlib_configuration_api.html)
- [seaborn aesthetics](https://seaborn.pydata.org/tutorial/aesthetics.html)
- [seaborn color palettes](https://seaborn.pydata.org/tutorial/color_palettes.html)
- [matplotlib colormaps](https://matplotlib.org/stable/tutorials/colors/colormaps.html)
