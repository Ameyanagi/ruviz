//! Documentation example: fast mode vs exact rendering
//!
//! Generates the side-by-side comparison images the performance guide shows
//! next to the fast-mode limitations:
//!
//! - docs/assets/rustdoc/fast_mode_scatter_exact.png
//! - docs/assets/rustdoc/fast_mode_scatter_fast.png
//! - docs/assets/rustdoc/fast_mode_line_exact.png
//! - docs/assets/rustdoc/fast_mode_line_fast.png
//!
//! The data is a seeded deterministic pseudo-random walk so regeneration is
//! reproducible.

use ruviz::prelude::*;

/// Small deterministic generator (SplitMix64) so this example does not need
/// a rand dependency and regenerates identically.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_f64(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Standard-normal-ish sample via the sum of twelve uniforms.
    fn next_gauss(&mut self) -> f64 {
        (0..12).map(|_| self.next_f64()).sum::<f64>() - 6.0
    }
}

fn main() -> PlotResult<()> {
    let mut rng = SplitMix64(20260713);
    let n = 1_000_000;
    let x: Vec<f64> = (0..n).map(|_| rng.next_gauss()).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&value| (value + 1.2 * rng.next_gauss()) * 0.5)
        .collect();

    Plot::new()
        .size_px(640, 320)
        .title("Exact: every marker composited")
        .scatter(&x, &y)
        .save("docs/assets/rustdoc/fast_mode_scatter_exact.png")?;

    let fast_scatter: Plot = Plot::new()
        .size_px(640, 320)
        .title("Fast: density with the marker footprint")
        .scatter(&x, &y)
        .into();
    fast_scatter
        .fast(true)
        .save("docs/assets/rustdoc/fast_mode_scatter_fast.png")?;

    let line_n = 200_000;
    let lx: Vec<f64> = (0..line_n).map(|i| i as f64).collect();
    let ly: Vec<f64> = lx
        .iter()
        .map(|&i| (i * 0.001).sin() + rng.next_gauss() * 0.1)
        .collect();

    Plot::new()
        .size_px(640, 320)
        .title("Exact: full stroke and markers")
        .line(&lx, &ly)
        .marker(MarkerStyle::Circle)
        .save("docs/assets/rustdoc/fast_mode_line_exact.png")?;

    let fast_line: Plot = Plot::new()
        .size_px(640, 320)
        .title("Fast: reduced stroke, exact markers")
        .line(&lx, &ly)
        .marker(MarkerStyle::Circle)
        .into();
    fast_line
        .fast(true)
        .save("docs/assets/rustdoc/fast_mode_line_fast.png")?;

    println!("✓ Generated docs/assets/rustdoc/fast_mode_*.png");
    Ok(())
}
