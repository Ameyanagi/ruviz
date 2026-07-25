#![cfg(feature = "3d")]

#[allow(dead_code)]
#[path = "../examples/generate_3d_gallery.rs"]
mod three_d_gallery;
#[path = "common/visual.rs"]
mod visual;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine;
use ruviz::prelude::*;

struct WorkingDirectory {
    original: PathBuf,
}

impl WorkingDirectory {
    fn enter(path: &Path) -> std::io::Result<Self> {
        let original = std::env::current_dir()?;
        std::env::set_current_dir(path)?;
        Ok(Self { original })
    }
}

impl Drop for WorkingDirectory {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.original)
            .expect("failed to restore 3D visual test working directory");
    }
}

fn fixture_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/golden/3d")
}

fn png_names(directory: &Path) -> std::io::Result<BTreeSet<String>> {
    fs::read_dir(directory)?
        .filter_map(|entry| match entry {
            Ok(entry)
                if entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "png") =>
            {
                Some(Ok(entry.file_name().to_string_lossy().into_owned()))
            }
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

#[test]
fn committed_3d_fixture_manifest_is_exact() {
    let expected: BTreeSet<String> = three_d_gallery::THREE_D_GALLERY_FIXTURES
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    let actual = png_names(&fixture_directory()).expect("read committed 3D fixtures");
    assert_eq!(actual, expected);
}

#[test]
fn committed_gallery_assets_match_the_exact_golden_images() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let gallery = repository.join("docs/assets/gallery/rust/3d");
    for fixture in three_d_gallery::THREE_D_GALLERY_FIXTURES {
        assert_eq!(
            fs::read(fixture_directory().join(fixture)).expect("read 3D golden"),
            fs::read(gallery.join(fixture)).expect("read 3D gallery asset"),
            "{fixture} must be byte-identical in gallery and golden storage"
        );
    }
}

#[test]
fn hybrid_svg_has_one_depth_layer_and_vector_axis_text() {
    let svg = surface(
        &[-1.0, 0.0, 1.0],
        &[-1.0, 0.0, 1.0],
        &[[0.0, 0.5, 0.0], [-0.5, 0.0, 0.5], [0.0, -0.5, 0.0]],
    )
    .title("hybrid-fixed-camera")
    .xlabel("axis-x-fixed")
    .ylabel("axis-y-fixed")
    .zlabel("axis-z-fixed")
    .camera(
        Camera3D::default()
            .azimuth_deg(-48.0)
            .elevation_deg(31.0)
            .orthographic(),
    )
    .size(4.0, 3.0)
    .dpi(80)
    .render_to_svg()
    .expect("hybrid SVG");

    assert_eq!(svg.matches("<image ").count(), 1);
    assert!(svg.matches("<text ").count() >= 4);
    assert!(svg.contains("<line "));
    assert!(svg.contains("hybrid-fixed-camera"));
    assert!(svg.contains("axis-x-fixed"));
    assert!(svg.contains("axis-y-fixed"));
    assert!(svg.contains("axis-z-fixed"));

    let encoded = svg
        .split_once("data:image/png;base64,")
        .expect("embedded PNG data URI")
        .1
        .split('"')
        .next()
        .expect("data URI terminator");
    let png = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .expect("decode embedded PNG");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    let layer = image::load_from_memory(&png)
        .expect("decode embedded raster layer")
        .to_rgba8();
    assert_eq!(layer.dimensions(), (320, 240));
    assert!(
        layer.pixels().any(|pixel| pixel[3] != 0),
        "embedded depth-tested layer must contain visible geometry"
    );
}

#[test]
#[ignore] // Exact pixels are checked with the pinned visual-regression toolchain.
fn deterministic_fixed_camera_3d_goldens_match_exactly()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let generated_root = tempfile::tempdir()?;
    let actual_directory = generated_root.path().join("3d");
    {
        let _working_directory = WorkingDirectory::enter(generated_root.path())?;
        three_d_gallery::generate_three_d_gallery(&actual_directory)?;
    }

    let committed_directory = fixture_directory();
    if std::env::var_os("UPDATE_3D_GOLDENS").is_some() {
        fs::create_dir_all(&committed_directory)?;
        for fixture in three_d_gallery::THREE_D_GALLERY_FIXTURES {
            fs::copy(
                actual_directory.join(fixture),
                committed_directory.join(fixture),
            )?;
        }
    }

    let artifact_directory = repository.join("generated/tests/render/3d");
    for fixture in three_d_gallery::THREE_D_GALLERY_FIXTURES {
        visual::assert_exact_pixels(
            &actual_directory.join(fixture),
            &committed_directory.join(fixture),
            &artifact_directory,
        )?;
    }
    Ok(())
}
