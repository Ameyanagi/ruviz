//! Pixel-level checks that [`Theme::seaborn`] reproduces seaborn's default look.
//!
//! The reference numbers come from `seaborn.set_theme()` rcParams:
//! `axes.facecolor = #EAEAF2`, `figure.facecolor = white`, `grid.color = white`,
//! `xtick.bottom = ytick.left = False`, `text.color = .15`, and the first entry
//! of the `deep` palette, `#4C72B0`.

use ruviz::prelude::*;

const PANEL: [u8; 3] = [0xEA, 0xEA, 0xF2];
const WHITE: [u8; 3] = [0xFF, 0xFF, 0xFF];
const DEEP_FIRST: [u8; 3] = [0x4C, 0x72, 0xB0];

struct Pixels {
    width: u32,
    height: u32,
    rgb: Vec<[u8; 3]>,
}

impl Pixels {
    fn render(plot: Plot) -> Self {
        let image = plot.render().expect("plot should render");
        let rgb = image
            .pixels
            .chunks_exact(4)
            .map(|px| [px[0], px[1], px[2]])
            .collect();
        Self {
            width: image.width,
            height: image.height,
            rgb,
        }
    }

    fn at(&self, x: u32, y: u32) -> [u8; 3] {
        self.rgb[(y * self.width + x) as usize]
    }

    fn count(&self, color: [u8; 3]) -> usize {
        self.rgb.iter().filter(|px| **px == color).count()
    }

    /// Bounding box of every pixel painted with `color`, as `(left, top, right, bottom)`.
    fn bounds_of(&self, color: [u8; 3]) -> (u32, u32, u32, u32) {
        let (mut left, mut top, mut right, mut bottom) = (u32::MAX, u32::MAX, 0, 0);
        for y in 0..self.height {
            for x in 0..self.width {
                if self.at(x, y) == color {
                    left = left.min(x);
                    top = top.min(y);
                    right = right.max(x);
                    bottom = bottom.max(y);
                }
            }
        }
        assert_ne!(left, u32::MAX, "no pixel painted {color:?}");
        (left, top, right, bottom)
    }

    /// Pixels painted `color` within `(left, top, right, bottom)`, ends included.
    fn count_in(&self, color: [u8; 3], (left, top, right, bottom): (u32, u32, u32, u32)) -> usize {
        (left..=right)
            .flat_map(|x| (top..=bottom).map(move |y| (x, y)))
            .filter(|&(x, y)| self.at(x, y) == color)
            .count()
    }

    /// The panel box shrunk by a pixel, dropping the antialiased ring where the
    /// fill meets the figure background.
    fn panel_interior(&self) -> (u32, u32, u32, u32) {
        let (left, top, right, bottom) = self.bounds_of(PANEL);
        (left + 1, top + 1, right - 1, bottom - 1)
    }

    /// Whether a panel pixel carries grid ink. Seaborn strokes the grid white
    /// over the `#EAEAF2` fill, so anything lighter than the fill belongs to a
    /// grid line - including the fringe of a line that lands between pixel
    /// centres. Nothing else inside the panel is lighter than the fill: the
    /// curve and any text are darker.
    fn is_grid_ink(px: [u8; 3]) -> bool {
        px[0] > PANEL[0] && px[1] > PANEL[1] && px[2] > PANEL[2]
    }

    fn grid_runs(pixels: impl Iterator<Item = [u8; 3]>) -> usize {
        let mut runs = 0;
        let mut inside = false;
        for px in pixels {
            let ink = Self::is_grid_ink(px);
            runs += usize::from(ink && !inside);
            inside = ink;
        }
        runs
    }

    /// Grid lines crossing the panel vertically, counted as runs of grid ink
    /// along whichever row meets the most of them. Counting runs keeps the
    /// measurement independent of where the layout puts the panel and its
    /// ticks, which shifts with the platform's font metrics.
    fn vertical_grid_lines(&self) -> usize {
        let (left, top, right, bottom) = self.panel_interior();
        (top..=bottom)
            .map(|y| Self::grid_runs((left..=right).map(|x| self.at(x, y))))
            .max()
            .unwrap_or(0)
    }

    /// The same count for horizontal grid lines, scanning columns.
    fn horizontal_grid_lines(&self) -> usize {
        let (left, top, right, bottom) = self.panel_interior();
        (left..=right)
            .map(|x| Self::grid_runs((top..=bottom).map(|y| self.at(x, y))))
            .max()
            .unwrap_or(0)
    }
}

fn sine() -> (Vec<f64>, Vec<f64>) {
    let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let y = x.iter().map(|v| v.sin()).collect();
    (x, y)
}

fn seaborn_line_sized(width: u32, height: u32) -> Plot {
    let (x, y) = sine();
    Plot::new()
        .theme(Theme::seaborn())
        .size_px(width, height)
        .title("Line")
        .xlabel("x")
        .ylabel("y")
        .line(&x, &y)
        .into()
}

fn seaborn_line() -> Plot {
    seaborn_line_sized(800, 600)
}

#[test]
fn seaborn_theme_carries_the_seaborn_rcparams() {
    let theme = Theme::seaborn();
    assert_eq!(
        theme.panel_background,
        Some(Color::from_hex("#EAEAF2").unwrap())
    );
    assert_eq!(theme.background, Color::WHITE);
    assert_eq!(theme.grid_color, Color::WHITE);
    assert_eq!(theme.foreground, Color::from_hex("#262626").unwrap());
    assert!(!theme.frame);
    assert!(!theme.tick_marks);
    assert_eq!(theme.patch_edge_color, Some(Color::WHITE));
    assert_eq!(theme.color_palette.len(), 10);
    assert_eq!(theme.get_color(0), Color::from_hex("#4C72B0").unwrap());
    assert_eq!(theme.get_color(9), Color::from_hex("#64B5CD").unwrap());
}

#[test]
fn seaborn_theme_fills_the_panel_and_leaves_the_figure_white() {
    let pixels = Pixels::render(seaborn_line());

    assert_eq!(pixels.at(0, 0), WHITE, "figure corner should stay white");
    let (left, top, right, bottom) = pixels.bounds_of(PANEL);
    assert!(
        left > 0 && top > 0 && right < pixels.width - 1 && bottom < pixels.height - 1,
        "the panel should be inset from the figure edge, got {left},{top},{right},{bottom}"
    );

    // The panel is filled, not merely outlined: nearly every pixel inside the
    // box is the fill colour, the remainder being grid lines and the curve.
    // Sampling one computed point instead would be a coin flip, since a grid
    // line lands wherever the platform's font metrics push the layout.
    let area = ((right - left + 1) as usize) * ((bottom - top + 1) as usize);
    let filled = pixels.count_in(PANEL, (left, top, right, bottom));
    assert!(
        filled * 10 > area * 8,
        "the panel should be filled with #EAEAF2, got {filled} of {area} pixels"
    );
}

#[test]
fn seaborn_theme_draws_white_grid_lines_on_the_panel() {
    // Two sizes, one of them odd, so the panel lands on a different sub-pixel
    // offset in each. Platform font metrics move the layout the same way.
    for (width, height) in [(800, 600), (801, 603)] {
        let pixels = Pixels::render(seaborn_line_sized(width, height));

        // Count grid lines as runs of ink rather than sampling computed offsets:
        // a line that falls between pixel centres is antialiased across two
        // columns and neither is pure white, so how many *pure white* columns a
        // panel shows is an accident of the layout.
        let vertical = pixels.vertical_grid_lines();
        let horizontal = pixels.horizontal_grid_lines();
        assert!(
            vertical >= 3,
            "expected vertical grid lines at {width}x{height}, found {vertical}"
        );
        assert!(
            horizontal >= 3,
            "expected horizontal grid lines at {width}x{height}, found {horizontal}"
        );

        // The grid is white, not a lighter tint of the panel: wherever a line
        // does land on a pixel centre it paints pure #FFFFFF.
        let white = pixels.count_in(WHITE, pixels.panel_interior());
        assert!(white > 0, "grid lines at {width}x{height} should be white");
    }
}

#[test]
fn seaborn_theme_draws_no_frame_and_no_tick_marks() {
    let pixels = Pixels::render(seaborn_line());
    let (left, top, right, bottom) = pixels.bounds_of(PANEL);
    let foreground = Color::from_hex("#262626").unwrap();
    let fg = [foreground.r, foreground.g, foreground.b];

    // No spine: the panel edge rows and columns are panel fill, never foreground.
    for x in left..=right {
        assert_ne!(pixels.at(x, top), fg, "top spine drawn at column {x}");
        assert_ne!(pixels.at(x, bottom), fg, "bottom spine drawn at column {x}");
    }
    for y in top..=bottom {
        assert_ne!(pixels.at(left, y), fg, "left spine drawn at row {y}");
        assert_ne!(pixels.at(right, y), fg, "right spine drawn at row {y}");
    }

    // No tick marks: the strip immediately outside the panel carries no ink,
    // only the antialiased fringe of the panel fill. Tick *labels*, which sit
    // further out, are still drawn - see the next test.
    let is_ink = |px: [u8; 3]| u32::from(px[0]) + u32::from(px[1]) + u32::from(px[2]) < 500;
    for y in bottom + 1..=bottom + 4 {
        for x in left..=right {
            assert!(!is_ink(pixels.at(x, y)), "tick mark below panel at {x},{y}");
        }
    }
    for x in left.saturating_sub(4)..left {
        for y in top..=bottom {
            assert!(
                !is_ink(pixels.at(x, y)),
                "tick mark left of panel at {x},{y}"
            );
        }
    }
}

#[test]
fn seaborn_theme_keeps_tick_labels_while_dropping_tick_marks() {
    let pixels = Pixels::render(seaborn_line());
    let foreground = Color::from_hex("#262626").unwrap();
    let fg = [foreground.r, foreground.g, foreground.b];
    let (_, _, _, bottom) = pixels.bounds_of(PANEL);

    let label_ink = (bottom + 5..pixels.height)
        .flat_map(|y| (0..pixels.width).map(move |x| (x, y)))
        .filter(|&(x, y)| pixels.at(x, y) == fg)
        .count();
    assert!(
        label_ink > 0,
        "tick labels should still be drawn below the panel"
    );
}

#[test]
fn seaborn_theme_uses_the_deep_palette_for_the_first_series() {
    let pixels = Pixels::render(seaborn_line());
    assert!(
        pixels.count(DEEP_FIRST) > 500,
        "expected the line drawn in deep[0] #4C72B0, found {} pixels",
        pixels.count(DEEP_FIRST)
    );
}

#[test]
fn seaborn_theme_gives_bars_a_white_edge_instead_of_a_darkened_one() {
    let plot: Plot = Plot::new()
        .theme(Theme::seaborn())
        .size_px(800, 600)
        .bar(&["A", "B", "C", "D"], &[3.0, 7.0, 5.0, 9.0])
        .into();
    let pixels = Pixels::render(plot);

    let darkened = Color::from_hex("#4C72B0").unwrap().darken(0.3);
    assert_eq!(
        pixels.count([darkened.r, darkened.g, darkened.b]),
        0,
        "seaborn bars must not carry the darkened default edge"
    );
    assert!(
        pixels.count(DEEP_FIRST) > 1000,
        "bars should be filled with deep[0]"
    );
}

#[test]
fn seaborn_theme_svg_matches_the_raster_panel_and_frame() {
    let svg = seaborn_line()
        .render_to_svg()
        .expect("seaborn plot should export SVG");

    assert!(
        svg.contains(r#"fill="rgb(234,234,242)""#),
        "SVG should paint the panel with #EAEAF2"
    );
    assert!(
        svg.contains(r#"stroke="rgb(255,255,255)""#),
        "SVG should stroke the grid white"
    );
    assert!(
        !svg.contains(r#"stroke="rgb(38,38,38)""#),
        "SVG should carry neither spines nor tick marks"
    );
}

#[test]
fn other_themes_keep_their_frame_ticks_and_bare_panel() {
    for theme in [
        Theme::light(),
        Theme::dark(),
        Theme::publication(),
        Theme::minimal(),
        Theme::presentation(),
        Theme::ieee(),
        Theme::nature(),
        Theme::paul_tol(),
        Theme::colorblind_friendly(),
    ] {
        assert_eq!(theme.panel_background, None);
        assert!(theme.frame);
        assert!(theme.tick_marks);
        assert_eq!(theme.patch_edge_color, None);
    }
}
