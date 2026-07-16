#[allow(dead_code)]
#[path = "../examples/doc_subplots.rs"]
mod doc_subplots;
#[allow(dead_code)]
#[path = "../examples/generate_golden_images.rs"]
mod golden_generator;
#[path = "common/visual.rs"]
mod visual;

use std::{fs, path::Path};

#[test]
fn canonical_subplot_gallery_copy_matches_rustdoc_asset() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let rustdoc = repository.join("docs/assets/rustdoc/subplots.png");
    let gallery = repository.join("docs/assets/gallery/rust/publication/subplots.png");
    assert_eq!(
        fs::read(&rustdoc).expect("read canonical rustdoc subplot"),
        fs::read(&gallery).expect("read gallery subplot copy"),
        "gallery subplot must be byte-identical to its canonical rustdoc source"
    );
}

#[test]
#[ignore] // Uses the pinned visual-regression toolchain in CI.
fn canonical_subplot_builder_matches_golden_fixture() -> Result<(), Box<dyn std::error::Error>> {
    golden_generator::register_golden_font()?;
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = repository.join("tests/fixtures/golden/gallery/gallery_subplots.png");
    let output_directory = tempfile::tempdir()?;
    let actual = output_directory.path().join("gallery_subplots.png");
    doc_subplots::build_subplots_figure(Some(golden_generator::GOLDEN_FONT_FAMILY))?
        .save(&actual)?;

    if std::env::var_os("UPDATE_GALLERY_SUBPLOT_GOLDEN").is_some() {
        fs::create_dir_all(fixture.parent().expect("fixture has a parent directory"))?;
        fs::copy(&actual, &fixture)?;
    }

    visual::assert_exact_pixels(
        &actual,
        &fixture,
        &repository.join("generated/tests/render"),
    )
}
