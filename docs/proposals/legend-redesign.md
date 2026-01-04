# Legend Redesign Proposal

## Problem Statement

The current ruviz legend implementation has critical flaws that make it unsuitable for publication-quality plotting:

### Visual Issues
1. **Wrong symbol type**: Always draws filled squares, even for line plots
2. **Lost style information**: No line style (solid/dashed/dotted) differentiation
3. **No marker representation**: Scatter plots show squares instead of actual markers

### Spacing Issues
4. **Hardcoded dimensions**: Fixed 120px width regardless of content
5. **No DPI scaling**: Spacing doesn't adapt to resolution
6. **Tight/uneven spacing**: Items feel cramped

### Position Issues
7. **Fixed inside placement**: Always overlaps plot data
8. **No 'best' algorithm**: Doesn't find empty space automatically
9. **No outside placement**: Can't put legend outside plot area
10. **No anchor control**: Can't fine-tune position

---

## Proposed Solution

### 1. Spacing System (matplotlib-compatible)

All spacing parameters in **font-size units** for automatic scaling:

```
┌─────────────────────────────────────────────────────────┐
│←borderpad→                              ←borderpad→│
│           ┌─────────────────────────────┐           │
│           │ TITLE (optional)            │           │
│           ├─────────────────────────────┤           │
│           │←─handle_length─→│←htpad→│text│           │
│           │      ════════   │       │    │           │
│           ├─────────────────────────────┤ ↕label    │
│           │      ── ── ──   │       │    │  spacing │
│           ├─────────────────────────────┤           │
│           │         ●       │       │    │           │
│           └─────────────────────────────┘           │
│←borderpad→                              ←borderpad→│
└─────────────────────────────────────────────────────────┘
           ↑                              ↑
     border_axes_pad                 border_axes_pad
      (from plot edge)
```

| Parameter | Default | Description |
|-----------|---------|-------------|
| `handle_length` | 2.0 | Length of line segment (font-size units) |
| `handle_height` | 0.7 | Height of handle area |
| `handle_text_pad` | 0.8 | Gap between handle and label |
| `label_spacing` | 0.5 | Vertical space between entries |
| `border_pad` | 0.4 | Internal padding |
| `border_axes_pad` | 0.5 | Gap from plot axes |
| `column_spacing` | 2.0 | Gap between columns (if multi-column) |

**Example calculation** (font_size = 10pt):
- handle_length = 2.0 × 10 = **20pt** (enough to show dash pattern)
- handle_text_pad = 0.8 × 10 = **8pt**
- Item width = 20 + 8 + text_width

---

### 2. Position System

#### Standard Positions (inside plot area)

```
┌─────────────────────────────────────┐
│ upper    upper     upper            │
│ left     center    right            │
│                                     │
│ center   center    center           │
│ left               right            │
│                                     │
│ lower    lower     lower            │
│ left     center    right            │
└─────────────────────────────────────┘
```

**Position codes** (matplotlib-compatible):
| Code | String | Description |
|------|--------|-------------|
| 0 | `"best"` | Auto-find minimum overlap |
| 1 | `"upper_right"` | Top-right corner |
| 2 | `"upper_left"` | Top-left corner |
| 3 | `"lower_left"` | Bottom-left corner |
| 4 | `"lower_right"` | Bottom-right corner |
| 5 | `"right"` | Center-right |
| 6 | `"center_left"` | Center-left |
| 7 | `"center_right"` | Center-right |
| 8 | `"lower_center"` | Bottom-center |
| 9 | `"upper_center"` | Top-center |
| 10 | `"center"` | Dead center |

#### Outside Positions (don't overlap data)

```
                outside_upper
        ┌───────────────────────────┐
        │                           │
outside │      PLOT AREA            │ outside
 _left  │                           │  _right
        │                           │
        └───────────────────────────┘
                outside_lower
```

| String | Description |
|--------|-------------|
| `"outside_right"` | Right of plot, top-aligned |
| `"outside_left"` | Left of plot, top-aligned |
| `"outside_upper"` | Above plot, right-aligned |
| `"outside_lower"` | Below plot, right-aligned |

#### Custom Position with Anchor

```rust
legend.position(LegendPosition::Custom {
    x: 1.02,      // In axes coordinates (0-1 = inside, >1 = outside)
    y: 1.0,
    anchor: Anchor::NorthWest,  // Which corner of legend box
})
```

**Anchor points:**
```
NW ─── N ─── NE
│             │
W      C      E
│             │
SW ─── S ─── SE
```

---

### 3. 'Best' Position Algorithm

When `loc = "best"`, find position with minimum data overlap:

```
Algorithm: find_best_position(legend_size, series_data)

1. Compute legend bounding box at each of 9 standard positions
2. For each position:
   a. Calculate overlap area with each series' bounding box
   b. Add penalty for positions near data-dense regions
   c. Score = total_overlap + edge_penalty
3. Select position with lowest score
4. If all positions have significant overlap:
   - Fall back to outside_right
   - Or use position with least critical overlap

Optimization: Skip 'best' calculation for >100k points (too slow)
             Default to upper_right instead
```

**Overlap calculation:**
```rust
fn calculate_overlap(legend_bbox: Rect, series_bboxes: &[Rect]) -> f32 {
    series_bboxes.iter()
        .map(|bbox| intersection_area(legend_bbox, *bbox))
        .sum()
}
```

---

### 4. Handle Rendering by Series Type

#### Line Series
Draw actual line segment with correct style:

```
Current (wrong):     Proposed (correct):
■ sin(x)            ─────  sin(x)      (solid)
■ cos(x)            ── ──  cos(x)      (dashed)
■ tan(x)            ·····  tan(x)      (dotted)
```

```rust
fn draw_line_handle(x, y, length, color, style, width) {
    match style {
        Solid => draw_line(x, y, x + length, y, color, width),
        Dashed => draw_dashed_line(x, y, x + length, y, color, width, [6, 4]),
        Dotted => draw_dashed_line(x, y, x + length, y, color, width, [2, 2]),
        DashDot => draw_dashed_line(x, y, x + length, y, color, width, [6, 2, 2, 2]),
    }
}
```

#### Scatter Series
Draw the actual marker:

```
Current (wrong):     Proposed (correct):
■ data              ●  data           (circle)
■ outliers          ◆  outliers       (diamond)
■ peaks             ▲  peaks          (triangle)
```

```rust
fn draw_scatter_handle(x, y, marker, size, color) {
    let cx = x + handle_length / 2.0;  // Center of handle area
    draw_marker(cx, y, marker, size, color);
}
```

#### Bar/Histogram Series
Filled rectangle (current behavior is correct for these):

```
■ Category A        ■  Category A     (filled rect)
■ Category B        ■  Category B     (different color)
```

#### Line + Marker (combined)
For plots with both line and markers:

```
──●──  data points   (line with marker at center)
```

---

### 5. Frame/Box Styling

```rust
pub struct LegendFrame {
    pub visible: bool,           // frameon
    pub background: Color,       // facecolor (with alpha)
    pub border_color: Color,     // edgecolor
    pub border_width: f32,       // linewidth
    pub corner_radius: f32,      // fancybox (0 = sharp, >0 = rounded)
    pub shadow: bool,            // shadow
    pub shadow_offset: (f32, f32),
}

impl Default for LegendFrame {
    fn default() -> Self {
        Self {
            visible: true,
            background: Color::new_rgba(255, 255, 255, 204),  // 80% opaque
            border_color: Color::new_rgba(0, 0, 0, 128),      // 50% black
            border_width: 0.8,
            corner_radius: 0.0,  // Sharp corners by default
            shadow: false,
            shadow_offset: (2.0, -2.0),
        }
    }
}
```

---

### 6. Complete API Design

```rust
// Builder pattern for legend configuration
Plot::new()
    .line(&x, &y1).label("sin(x)")
    .line(&x, &y2).label("cos(x)").style(LineStyle::Dashed)
    .legend(|l| l
        .position(LegendPosition::Best)      // or UpperRight, OutsideRight, etc.
        .columns(1)                          // Single column
        .title("Functions")                  // Optional title
        .frame(|f| f
            .background(Color::WHITE.with_alpha(0.9))
            .border_color(Color::GRAY)
            .corner_radius(3.0)
        )
        .spacing(|s| s
            .handle_length(2.5)              // In font-size units
            .label_spacing(0.6)
        )
    )
    .save("plot.png")?;

// Simple usage (sensible defaults)
Plot::new()
    .line(&x, &y).label("data")
    .legend_position(LegendPosition::UpperRight)
    .save("plot.png")?;
```

---

### 7. Data Structure Changes

```rust
// OLD: Lost information
legend_items: Vec<(String, Color)>

// NEW: Full series information preserved
pub struct LegendItem {
    pub label: String,
    pub color: Color,
    pub series_type: LegendSeriesType,
}

pub enum LegendSeriesType {
    Line {
        style: LineStyle,
        width: f32,
    },
    Scatter {
        marker: MarkerStyle,
        size: f32,
    },
    LineMarker {
        line_style: LineStyle,
        line_width: f32,
        marker: MarkerStyle,
        marker_size: f32,
    },
    Bar,
    Area {
        edge_color: Option<Color>,
    },
    ErrorBar,
}
```

---

### 8. Implementation Priority

| Phase | Task | Effort |
|-------|------|--------|
| **P0** | Correct handle rendering (line/scatter/bar) | Medium |
| **P0** | Font-size-based spacing system | Low |
| **P1** | All 10 standard positions | Low |
| **P1** | Outside positions (4 variants) | Medium |
| **P2** | 'Best' position algorithm | High |
| **P2** | Custom position with anchor | Medium |
| **P3** | Multi-column layout | Medium |
| **P3** | Shadow/fancy styling | Low |

---

## References

- [matplotlib.pyplot.legend](https://matplotlib.org/stable/api/_as_gen/matplotlib.pyplot.legend.html)
- [matplotlib Legend API](https://matplotlib.org/stable/api/legend_api.html)
- [matplotlib Legend Guide](https://matplotlib.org/stable/users/explain/axes/legend_guide.html)

---

## Visual Comparison

### Before (Current)
```
┌────────────┐
│ ■ sin(x)   │  ← Square for everything
│ ■ cos(x)   │  ← No style differentiation
└────────────┘
  Fixed position, overlaps data
```

### After (Proposed)
```
┌──────────────┐
│ ────  sin(x) │  ← Solid line
│ ── ── cos(x) │  ← Dashed line
│   ●   data   │  ← Scatter marker
└──────────────┘
  'Best' position, avoids data
```

---

## Questions for Discussion

1. Should 'best' fall back to outside_right or upper_right when all inside positions overlap?
2. Default frame style: transparent or semi-opaque white?
3. Should we support horizontal legend layout (items in a row)?
4. Title positioning: inside box or above box?
