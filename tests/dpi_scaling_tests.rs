//! DPI Scaling Tests
//!
//! Tests that verify plots render consistently at different DPI settings.
//! Uses SSIM (Structural Similarity) to compare downscaled high-DPI images
//! with low-DPI images - they should be nearly identical.

mod common;

use ruviz::prelude::*;
use std::path::PathBuf;

/// Simple image comparison using mean squared error per channel
/// Returns similarity score from 0.0 to 1.0
fn compare_images_simple(img1: &image::RgbaImage, img2: &image::RgbaImage) -> f64 {
    if img1.dimensions() != img2.dimensions() {
        return 0.0;
    }

    let (width, height) = img1.dimensions();
    let total_pixels = (width * height) as f64;
    let mut total_diff = 0.0;

    for (p1, p2) in img1.pixels().zip(img2.pixels()) {
        let dr = (p1[0] as f64 - p2[0] as f64) / 255.0;
        let dg = (p1[1] as f64 - p2[1] as f64) / 255.0;
        let db = (p1[2] as f64 - p2[2] as f64) / 255.0;
        let da = (p1[3] as f64 - p2[3] as f64) / 255.0;

        total_diff += (dr * dr + dg * dg + db * db + da * da) / 4.0;
    }

    let mse = total_diff / total_pixels;
    // Convert MSE to similarity score (1 - normalized MSE)
    1.0 - mse.sqrt()
}

/// Resize image using nearest neighbor (simple but fast)
fn resize_image(img: &image::RgbaImage, new_width: u32, new_height: u32) -> image::RgbaImage {
    let (old_width, old_height) = img.dimensions();
    let mut result = image::RgbaImage::new(new_width, new_height);

    for y in 0..new_height {
        for x in 0..new_width {
            // Map to source coordinates
            let src_x = (x as f64 * old_width as f64 / new_width as f64) as u32;
            let src_y = (y as f64 * old_height as f64 / new_height as f64) as u32;

            let src_x = src_x.min(old_width - 1);
            let src_y = src_y.min(old_height - 1);

            result.put_pixel(x, y, *img.get_pixel(src_x, src_y));
        }
    }

    result
}

/// Render plot at specified DPI and return the image
fn render_at_dpi(x: &[f64], y: &[f64], dpi: u32) -> image::RgbaImage {
    let x_vec: Vec<f64> = x.to_vec();
    let y_vec: Vec<f64> = y.to_vec();

    let plot = Plot::new()
        .size(6.4, 4.8) // 6.4 x 4.8 inches
        .dpi(dpi)
        .line(&x_vec, &y_vec)
        .title("DPI Test")
        .xlabel("X")
        .ylabel("Y");

    let image = plot.render().expect("Render should succeed");

    // Convert ruviz Image to image::RgbaImage
    image::RgbaImage::from_raw(image.width, image.height, image.pixels)
        .expect("Image conversion should succeed")
}

#[test]
fn test_dpi_scaling_line_plot() {
    let x: Vec<f64> = (0..20).map(|i| i as f64 * 0.5).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.powi(2)).collect();

    // Render at 100 DPI (reference)
    let img_100dpi = render_at_dpi(&x, &y, 100);
    let (w100, h100) = img_100dpi.dimensions();

    // Render at 200 DPI
    let img_200dpi = render_at_dpi(&x, &y, 200);

    // Downscale 200 DPI to 100 DPI size
    let img_200dpi_downscaled = resize_image(&img_200dpi, w100, h100);

    // Compare
    let similarity = compare_images_simple(&img_100dpi, &img_200dpi_downscaled);

    println!("100 DPI size: {}x{}", w100, h100);
    println!(
        "200 DPI original size: {}x{}",
        img_200dpi.width(),
        img_200dpi.height()
    );
    println!("Similarity score: {:.4}", similarity);

    // Should be at least 90% similar (allowing for anti-aliasing differences)
    assert!(
        similarity > 0.90,
        "DPI scaling should preserve appearance: similarity = {:.4}",
        similarity
    );
}

#[test]
fn test_dpi_scaling_scatter_plot() {
    let x: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let y: Vec<f64> = vec![2.3, 4.1, 5.9, 8.2, 10.1, 12.4, 14.0, 16.3, 18.2, 20.0];

    // Render at 100 DPI (reference)
    let img_100dpi = {
        let plot = Plot::new()
            .size(6.4, 4.8)
            .dpi(100)
            .scatter(&x, &y)
            .title("Scatter DPI Test")
            .xlabel("X")
            .ylabel("Y");

        let image = plot.render().expect("Render should succeed");
        image::RgbaImage::from_raw(image.width, image.height, image.pixels)
            .expect("Image conversion should succeed")
    };

    let (w100, h100) = img_100dpi.dimensions();

    // Render at 200 DPI
    let img_200dpi = {
        let plot = Plot::new()
            .size(6.4, 4.8)
            .dpi(200)
            .scatter(&x, &y)
            .title("Scatter DPI Test")
            .xlabel("X")
            .ylabel("Y");

        let image = plot.render().expect("Render should succeed");
        image::RgbaImage::from_raw(image.width, image.height, image.pixels)
            .expect("Image conversion should succeed")
    };

    // Downscale 200 DPI to 100 DPI size
    let img_200dpi_downscaled = resize_image(&img_200dpi, w100, h100);

    // Compare
    let similarity = compare_images_simple(&img_100dpi, &img_200dpi_downscaled);

    println!("Scatter plot similarity score: {:.4}", similarity);

    assert!(
        similarity > 0.90,
        "DPI scaling should preserve scatter plot appearance: similarity = {:.4}",
        similarity
    );
}

#[test]
fn test_dpi_dimensions_scale_correctly() {
    // Test that DPI affects image dimensions correctly
    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];

    // 6.4 x 4.8 inches at 100 DPI = 640 x 480 pixels
    let img_100dpi = render_at_dpi(&x, &y, 100);
    let (w100, h100) = img_100dpi.dimensions();

    // 6.4 x 4.8 inches at 200 DPI = 1280 x 960 pixels
    let img_200dpi = render_at_dpi(&x, &y, 200);
    let (w200, h200) = img_200dpi.dimensions();

    println!("100 DPI: {}x{}", w100, h100);
    println!("200 DPI: {}x{}", w200, h200);

    assert_eq!(w100, 640, "100 DPI width should be 640");
    assert_eq!(h100, 480, "100 DPI height should be 480");
    assert_eq!(w200, 1280, "200 DPI width should be 1280");
    assert_eq!(h200, 960, "200 DPI height should be 960");
}

#[test]
fn test_dpi_output_files() {
    let output_100dpi = common::test_output_path("dpi_test_100.png");
    let output_200dpi = common::test_output_path("dpi_test_200.png");

    let x: Vec<f64> = (0..20).map(|i| i as f64 * 0.5).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.powi(2)).collect();

    // Save at 100 DPI
    Plot::new()
        .size(6.4, 4.8)
        .dpi(100)
        .line(&x, &y)
        .title("100 DPI Test")
        .xlabel("X")
        .ylabel("Y")
        .save(&output_100dpi)
        .expect("Save at 100 DPI should succeed");

    // Save at 200 DPI
    Plot::new()
        .size(6.4, 4.8)
        .dpi(200)
        .line(&x, &y)
        .title("200 DPI Test")
        .xlabel("X")
        .ylabel("Y")
        .save(&output_200dpi)
        .expect("Save at 200 DPI should succeed");

    assert!(output_100dpi.exists(), "100 DPI output should exist");
    assert!(output_200dpi.exists(), "200 DPI output should exist");

    // Verify file sizes (200 DPI should be larger)
    let size_100 = std::fs::metadata(&output_100dpi).unwrap().len();
    let size_200 = std::fs::metadata(&output_200dpi).unwrap().len();

    println!("100 DPI file size: {} bytes", size_100);
    println!("200 DPI file size: {} bytes", size_200);

    assert!(
        size_200 > size_100,
        "200 DPI file should be larger than 100 DPI"
    );
}
