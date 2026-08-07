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
}

fn sine() -> (Vec<f64>, Vec<f64>) {
    let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let y = x.iter().map(|v| v.sin()).collect();
    (x, y)
}

fn seaborn_line() -> Plot {
    let (x, y) = sine();
    Plot::new()
        .theme(Theme::seaborn())
        .size_px(800, 600)
        .title("Line")
        .xlabel("x")
        .ylabel("y")
        .line(&x, &y)
        .into()
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

    // A point inside the panel and away from any grid line or the sine curve.
    let inside_x = left + (right - left) / 8;
    let inside_y = top + (bottom - top) / 8;
    assert_eq!(pixels.at(inside_x, inside_y), PANEL);
}

#[test]
fn seaborn_theme_draws_white_grid_lines_on_the_panel() {
    let pixels = Pixels::render(seaborn_line());
    let (left, top, right, _) = pixels.bounds_of(PANEL);

    // Columns that are white from the top of the panel downwards are grid lines
    // drawn on the panel; there is no other source of white inside it.
    let grid_columns = (left + 2..right - 1)
        .filter(|&x| (top + 2..top + 40).all(|y| pixels.at(x, y) == WHITE))
        .count();
    assert!(
        grid_columns >= 4,
        "expected white vertical grid lines on the panel, found {grid_columns}"
    );
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
