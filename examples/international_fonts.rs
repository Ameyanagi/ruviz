//! Compare publication typography in plain and Typst modes.
//!
//! Install the relevant Noto fonts or pass font-file paths as arguments:
//! `cargo run --example international_fonts --features typst-math -- font.otf`
//! Outputs are written to `generated/examples/international_fonts/`.
use ruviz::prelude::*;

fn main() -> PlotResult<()> {
    for path in std::env::args().skip(1) {
        ruviz::render::register_font_bytes(std::fs::read(path)?)?;
    }
    let x: Vec<f64> = (0..120).map(|i| i as f64 / 119.0).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|x| 0.12 + 0.15 * (x * 7.0).sin().powi(2))
        .collect();
    let config = ruviz::core::PlotConfig::builder()
        .typography(|_| ruviz::core::TypographyConfig::publication().base_size(12.0))
        .build();
    let mut plot: Plot = Plot::with_config(config)
        .line(&x, &y)
        .label("Signal / 信号")
        .legend(LegendPosition::UpperRight)
        .title("散乱強度 / Intensity (Å⁻²)")
        .xlabel("Wave vector q (Å⁻¹)")
        .ylabel("Intensity (Å⁻²)")
        .language("ja")
        .size_px(1100, 800)
        .xlim(0.0, 1.0)
        .ylim(0.0, 1.0)
        .into();
    for (row, (language, text)) in [
        ("ja", "日本語 — 散乱強度 / Signal"),
        ("zh-CN", "简体中文 — 散射强度 / Signal"),
        ("zh-TW", "繁體中文 — 散射強度 / Signal"),
        ("ko", "한국어 — 산란 강도 / Signal"),
        ("ar", "العربية — شدة الإشارة / Signal 123"),
        ("he", "עברית — עוצמת האות / Signal 123"),
    ]
    .into_iter()
    .enumerate()
    {
        plot = plot.annotate(Annotation::text_styled(
            0.5,
            0.91 - row as f64 * 0.103,
            text,
            TextStyle::new()
                .font_size(15.0)
                .text_options(TextOptions::new().language(language)),
        ));
    }
    let output = std::path::Path::new("generated/examples/international_fonts");
    std::fs::create_dir_all(output)?;
    plot.clone().save(output.join("plain.png"))?;
    plot.clone().save(output.join("plain.svg"))?;
    #[cfg(feature = "typst-math")]
    {
        let typst = plot.typst(true);
        typst.clone().save(output.join("typst.png"))?;
        typst.save(output.join("typst.svg"))?;
    }
    println!("Typography comparison saved to {}", output.display());
    Ok(())
}
