//! Integration tests for international text rendering
//!
//! These tests verify that the text rendering system properly handles
//! various Unicode scripts including CJK, Arabic, Hebrew, and emoji.

use ruviz::prelude::*;
use ruviz::render::{Color, FontConfig, FontFamily, TextRenderer};
use std::path::Path;
use tiny_skia::Pixmap;

const DARK_BACKGROUND: [u8; 4] = [16, 16, 24, 255];

#[derive(Debug, PartialEq, Eq)]
struct InkBitmap {
    width: u32,
    height: u32,
    pixels: Vec<bool>,
}

fn pixel_rgba(pixmap: &Pixmap, x: u32, y: u32) -> [u8; 4] {
    let pixel = pixmap.pixels()[(y * pixmap.width() + x) as usize];
    [pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()]
}

fn is_ink(pixel: [u8; 4]) -> bool {
    pixel[..3]
        .iter()
        .zip(DARK_BACKGROUND[..3].iter())
        .any(|(&channel, &background)| channel.abs_diff(background) > 2)
}

fn ink_bounds(pixmap: &Pixmap) -> Option<(u32, u32, u32, u32)> {
    let mut bounds = (pixmap.width(), pixmap.height(), 0, 0);
    let mut found = false;

    for y in 0..pixmap.height() {
        for x in 0..pixmap.width() {
            if is_ink(pixel_rgba(pixmap, x, y)) {
                found = true;
                bounds.0 = bounds.0.min(x);
                bounds.1 = bounds.1.min(y);
                bounds.2 = bounds.2.max(x);
                bounds.3 = bounds.3.max(y);
            }
        }
    }

    found.then_some(bounds)
}

fn cropped_pixels(pixmap: &Pixmap, bounds: (u32, u32, u32, u32)) -> Vec<[u8; 4]> {
    let mut pixels = Vec::new();
    for y in bounds.1..=bounds.3 {
        for x in bounds.0..=bounds.2 {
            pixels.push(pixel_rgba(pixmap, x, y));
        }
    }
    pixels
}

fn ink_column_runs(pixmap: &Pixmap, bounds: (u32, u32, u32, u32)) -> Vec<(u32, u32)> {
    let occupied = |x| (bounds.1..=bounds.3).any(|y| is_ink(pixel_rgba(pixmap, x, y)));
    let mut runs = Vec::new();
    let mut start = None;

    for x in bounds.0..=bounds.2 {
        match (start, occupied(x)) {
            (None, true) => start = Some(x),
            (Some(run_start), false) => {
                runs.push((run_start, x - 1));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(run_start) = start {
        runs.push((run_start, bounds.2));
    }
    runs
}

fn normalized_ink_bitmap(pixmap: &Pixmap, x_bounds: (u32, u32)) -> InkBitmap {
    let ink_pixels = (0..pixmap.height()).flat_map(|y| {
        (x_bounds.0..=x_bounds.1)
            .filter(move |&x| is_ink(pixel_rgba(pixmap, x, y)))
            .map(move |x| (x, y))
    });
    let (mut min_y, mut max_y) = (pixmap.height(), 0);
    for (_, y) in ink_pixels {
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    assert!(min_y <= max_y, "glyph region contained no ink");

    let width = x_bounds.1 - x_bounds.0 + 1;
    let height = max_y - min_y + 1;
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for y in min_y..=max_y {
        for x in x_bounds.0..=x_bounds.1 {
            pixels.push(is_ink(pixel_rgba(pixmap, x, y)));
        }
    }

    InkBitmap {
        width,
        height,
        pixels,
    }
}

fn luminance(pixel: [u8; 4]) -> u32 {
    2126 * pixel[0] as u32 + 7152 * pixel[1] as u32 + 722 * pixel[2] as u32
}

/// Test basic ASCII text rendering
#[test]
fn test_ascii_text_rendering() {
    let renderer = TextRenderer::new();
    let mut pixmap = Pixmap::new(400, 100).expect("Failed to create pixmap");
    pixmap.fill(tiny_skia::Color::WHITE);

    let config = FontConfig::new(FontFamily::SansSerif, 24.0);
    let result = renderer.render_text(
        &mut pixmap,
        "Hello, World!",
        10.0,
        50.0,
        &config,
        Color::BLACK,
    );

    assert!(result.is_ok(), "ASCII text rendering should succeed");
}

/// Lock scientific Unicode glyph coverage and premultiplied AA on dark backgrounds.
#[test]
fn scientific_unicode_preserves_light_on_dark_edges_and_rotation() {
    let font_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/ruviz-web/assets/NotoSans-Regular.ttf");
    let font_bytes = std::fs::read(&font_path).unwrap_or_else(|error| {
        panic!(
            "failed to read tracked deterministic test font {}: {error}",
            font_path.display()
        )
    });
    ruviz::render::register_font_bytes(font_bytes)
        .expect("failed to register tracked Noto Sans test font");

    let renderer = TextRenderer::new();
    let config = FontConfig::new(FontFamily::Name("Noto Sans".to_string()), 48.0);
    let text = "Å χ ² μ";
    let ink = Color::new(235, 235, 245);
    let background = tiny_skia::Color::from_rgba8(16, 16, 24, 255);
    let mut normal = Pixmap::new(512, 192).expect("failed to create normal text pixmap");
    let mut rotated = Pixmap::new(256, 512).expect("failed to create rotated text pixmap");
    normal.fill(background);
    rotated.fill(background);

    renderer
        .render_text(&mut normal, text, 32.0, 48.0, &config, ink)
        .expect("normal scientific Unicode rendering failed");
    renderer
        .render_text_rotated(&mut rotated, text, 128.0, 256.0, &config, ink)
        .expect("rotated scientific Unicode rendering failed");

    let normal_bounds = ink_bounds(&normal).expect("Noto Sans rendered no normal glyph pixels");
    let rotated_bounds = ink_bounds(&rotated).expect("Noto Sans rendered no rotated glyph pixels");
    let normal_size = (
        normal_bounds.2 - normal_bounds.0 + 1,
        normal_bounds.3 - normal_bounds.1 + 1,
    );
    let rotated_size = (
        rotated_bounds.2 - rotated_bounds.0 + 1,
        rotated_bounds.3 - rotated_bounds.1 + 1,
    );
    assert_eq!(
        rotated_size,
        (normal_size.1, normal_size.0),
        "90-degree rendering must swap the cropped ink dimensions"
    );

    // Exact RGBA parity catches damaged premultiplication at antialiased edges.
    let normal_pixels = cropped_pixels(&normal, normal_bounds);
    let rotated_pixels = cropped_pixels(&rotated, rotated_bounds);
    for y in 0..normal_size.1 {
        for x in 0..normal_size.0 {
            let normal_index = (y * normal_size.0 + x) as usize;
            let rotated_x = y;
            let rotated_y = normal_size.0 - 1 - x;
            let rotated_index = (rotated_y * rotated_size.0 + rotated_x) as usize;
            assert_eq!(rotated_pixels[rotated_index], normal_pixels[normal_index]);
        }
    }

    let background_luminance = luminance(DARK_BACKGROUND);
    let ink_luminance = luminance([ink.r, ink.g, ink.b, ink.a]);
    assert!(
        normal_pixels
            .iter()
            .any(|&pixel| luminance(pixel) >= ink_luminance - 10_000 * 10),
        "glyphs must contain bright interior pixels near the requested ink color"
    );
    assert!(
        normal_pixels.iter().any(|&pixel| {
            let value = luminance(pixel);
            value > background_luminance + 10_000 * 2 && value < ink_luminance - 10_000 * 2
        }),
        "glyphs must retain intermediate-luminance antialiased edge pixels"
    );

    // Spaces isolate the four tokens; distinct normalized masks reject tofu boxes.
    let runs = ink_column_runs(&normal, normal_bounds);
    assert_eq!(runs.len(), 4, "expected one ink run for each Unicode token");
    let glyphs: Vec<_> = runs
        .into_iter()
        .map(|run| normalized_ink_bitmap(&normal, run))
        .collect();
    for (index, glyph) in glyphs.iter().enumerate() {
        assert!(
            glyph.pixels.iter().filter(|&&pixel| pixel).count() >= 8,
            "Unicode token {index} rendered too few ink pixels"
        );
    }
    for left in 0..glyphs.len() {
        for right in left + 1..glyphs.len() {
            assert_ne!(
                glyphs[left], glyphs[right],
                "Unicode tokens {left} and {right} rendered as the same glyph"
            );
        }
    }
}

/// Test CJK (Chinese/Japanese/Korean) text rendering
#[test]
fn test_cjk_text_rendering() {
    let renderer = TextRenderer::new();
    let mut pixmap = Pixmap::new(600, 100).expect("Failed to create pixmap");
    pixmap.fill(tiny_skia::Color::WHITE);

    let config = FontConfig::new(FontFamily::SansSerif, 24.0);

    // Japanese
    let result = renderer.render_text(
        &mut pixmap,
        "日本語テスト",
        10.0,
        50.0,
        &config,
        Color::BLACK,
    );
    assert!(result.is_ok(), "Japanese text rendering should succeed");
}

/// Test Chinese text rendering
#[test]
fn test_chinese_text_rendering() {
    let renderer = TextRenderer::new();
    let mut pixmap = Pixmap::new(600, 100).expect("Failed to create pixmap");
    pixmap.fill(tiny_skia::Color::WHITE);

    let config = FontConfig::new(FontFamily::SansSerif, 24.0);

    // Simplified Chinese
    let result = renderer.render_text(&mut pixmap, "中文测试", 10.0, 50.0, &config, Color::BLACK);
    assert!(result.is_ok(), "Chinese text rendering should succeed");
}

/// Test Korean text rendering
#[test]
fn test_korean_text_rendering() {
    let renderer = TextRenderer::new();
    let mut pixmap = Pixmap::new(600, 100).expect("Failed to create pixmap");
    pixmap.fill(tiny_skia::Color::WHITE);

    let config = FontConfig::new(FontFamily::SansSerif, 24.0);

    // Korean
    let result = renderer.render_text(
        &mut pixmap,
        "한국어 테스트",
        10.0,
        50.0,
        &config,
        Color::BLACK,
    );
    assert!(result.is_ok(), "Korean text rendering should succeed");
}

/// Test mixed script rendering (ASCII + CJK)
#[test]
fn test_mixed_script_rendering() {
    let renderer = TextRenderer::new();
    let mut pixmap = Pixmap::new(800, 100).expect("Failed to create pixmap");
    pixmap.fill(tiny_skia::Color::WHITE);

    let config = FontConfig::new(FontFamily::SansSerif, 24.0);

    // Mixed Latin and Japanese
    let result = renderer.render_text(
        &mut pixmap,
        "Hello 日本語 World",
        10.0,
        50.0,
        &config,
        Color::BLACK,
    );
    assert!(result.is_ok(), "Mixed script rendering should succeed");
}

/// Test emoji rendering
#[test]
fn test_emoji_rendering() {
    let renderer = TextRenderer::new();
    let mut pixmap = Pixmap::new(400, 100).expect("Failed to create pixmap");
    pixmap.fill(tiny_skia::Color::WHITE);

    let config = FontConfig::new(FontFamily::SansSerif, 24.0);

    // Emoji characters
    let result = renderer.render_text(&mut pixmap, "📊📈🎨", 10.0, 50.0, &config, Color::BLACK);
    // Emoji may not render if fonts are not available, but should not error
    assert!(result.is_ok(), "Emoji rendering should not error");
}

/// Test text measurement for international text
#[test]
fn test_international_text_measurement() {
    let renderer = TextRenderer::new();
    let config = FontConfig::new(FontFamily::SansSerif, 24.0);

    // ASCII should have positive width
    let (ascii_width, ascii_height) = renderer
        .measure_text("Hello", &config)
        .expect("ASCII measurement should succeed");
    assert!(ascii_width > 0.0, "ASCII text should have positive width");
    assert!(ascii_height > 0.0, "ASCII text should have positive height");

    // CJK should also have positive width
    let (cjk_width, cjk_height) = renderer
        .measure_text("日本語", &config)
        .expect("CJK measurement should succeed");
    assert!(cjk_width > 0.0, "CJK text should have positive width");
    assert!(cjk_height > 0.0, "CJK text should have positive height");
}

/// Test rotated international text
#[test]
fn test_rotated_international_text() {
    let renderer = TextRenderer::new();
    let mut pixmap = Pixmap::new(200, 400).expect("Failed to create pixmap");
    pixmap.fill(tiny_skia::Color::WHITE);

    let config = FontConfig::new(FontFamily::SansSerif, 18.0);

    // Rotated Japanese text (like Y-axis label)
    let result = renderer.render_text_rotated(
        &mut pixmap,
        "縦書きテスト",
        50.0,
        200.0,
        &config,
        Color::BLACK,
    );
    assert!(result.is_ok(), "Rotated CJK text should succeed");
}

/// Test that font system singleton is properly shared
#[test]
fn test_font_system_singleton() {
    use ruviz::render::text::{get_font_system, get_swash_cache};

    // Get references multiple times - should return same instance
    let fs1 = get_font_system();
    let fs2 = get_font_system();
    let sc1 = get_swash_cache();
    let sc2 = get_swash_cache();

    // Verify they point to the same memory
    assert!(std::ptr::eq(fs1, fs2), "FontSystem should be singleton");
    assert!(std::ptr::eq(sc1, sc2), "SwashCache should be singleton");
}

/// Generate a visual test image with international text
#[test]
fn test_international_plot_generation() {
    // Create output directory
    let output_dir = Path::new("generated/tests/render");
    std::fs::create_dir_all(output_dir).ok();

    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![2.0, 4.0, 1.0, 3.0, 5.0];

    // Test with Japanese text
    let result = Plot::new()
        .title("日本語フォントテスト".to_string())
        .xlabel("横軸 (X)".to_string())
        .ylabel("縦軸 (Y)".to_string())
        .line(&x_data, &y_data)
        .save("generated/tests/render/japanese_test.png");

    assert!(result.is_ok(), "Japanese plot generation should succeed");

    // Verify file was created
    assert!(
        Path::new("generated/tests/render/japanese_test.png").exists(),
        "Japanese output file should exist"
    );
}
