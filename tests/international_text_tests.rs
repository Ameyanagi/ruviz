//! Integration tests for international text rendering
//!
//! These tests verify that the text rendering system properly handles
//! various Unicode scripts including CJK, Arabic, Hebrew, and emoji.

use ruviz::prelude::*;
use ruviz::render::{TextRenderer, FontConfig, FontFamily, Color};
use tiny_skia::Pixmap;
use std::path::Path;

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
        10.0, 50.0,
        &config,
        Color::BLACK,
    );

    assert!(result.is_ok(), "ASCII text rendering should succeed");
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
        10.0, 50.0,
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
    let result = renderer.render_text(
        &mut pixmap,
        "中文测试",
        10.0, 50.0,
        &config,
        Color::BLACK,
    );
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
        10.0, 50.0,
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
        10.0, 50.0,
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
    let result = renderer.render_text(
        &mut pixmap,
        "📊📈🎨",
        10.0, 50.0,
        &config,
        Color::BLACK,
    );
    // Emoji may not render if fonts are not available, but should not error
    assert!(result.is_ok(), "Emoji rendering should not error");
}

/// Test text measurement for international text
#[test]
fn test_international_text_measurement() {
    let renderer = TextRenderer::new();
    let config = FontConfig::new(FontFamily::SansSerif, 24.0);

    // ASCII should have positive width
    let (ascii_width, ascii_height) = renderer.measure_text("Hello", &config)
        .expect("ASCII measurement should succeed");
    assert!(ascii_width > 0.0, "ASCII text should have positive width");
    assert!(ascii_height > 0.0, "ASCII text should have positive height");

    // CJK should also have positive width
    let (cjk_width, cjk_height) = renderer.measure_text("日本語", &config)
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
        50.0, 200.0,
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
    let output_dir = Path::new("test_output");
    std::fs::create_dir_all(output_dir).ok();

    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![2.0, 4.0, 1.0, 3.0, 5.0];

    // Test with Japanese text
    let result = Plot::new()
        .title("日本語フォントテスト".to_string())
        .xlabel("横軸 (X)".to_string())
        .ylabel("縦軸 (Y)".to_string())
        .line(&x_data, &y_data)
        .save("test_output/japanese_test.png");

    assert!(result.is_ok(), "Japanese plot generation should succeed");

    // Verify file was created
    assert!(Path::new("test_output/japanese_test.png").exists(),
            "Japanese output file should exist");
}
