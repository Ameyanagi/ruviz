//! Export functionality
//!
//! Provides export capabilities for various formats:
//! - PNG: Raster export via `Plot::save()`
//! - SVG: Vector export via `Plot::to_svg()` or `Plot::render_to_svg()`
//! - PDF: Vector export via `Plot::save_pdf()` (requires `pdf` feature)
//!
//! The PDF export uses an SVG → PDF pipeline for high-quality vector output.

use crate::core::{PlottingError, Result};
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub mod svg;

#[cfg(feature = "pdf")]
pub mod pdf;

#[cfg(feature = "pdf")]
pub mod svg_to_pdf;

pub use svg::SvgRenderer;

#[cfg(feature = "pdf")]
pub use pdf::PdfRenderer;

#[cfg(feature = "pdf")]
pub use svg_to_pdf::{page_sizes, svg_to_pdf, svg_to_pdf_file};

fn atomic_temp_path(path: &Path) -> PathBuf {
    static TEMP_PATH_NONCE: AtomicU64 = AtomicU64::new(0);
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("ruviz-output");
    let nonce = TEMP_PATH_NONCE.fetch_add(1, Ordering::Relaxed);
    parent.join(format!(
        ".{}.{}.{}.tmp",
        file_name,
        std::process::id(),
        nonce
    ))
}

fn cleanup_temp_file(path: &Path) {
    let _ = fs::remove_file(path);
}

fn rename_temp_into_place(temp_path: &Path, path: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        match fs::rename(temp_path, path) {
            Ok(()) => Ok(()),
            Err(err)
                if path.exists()
                    && matches!(
                        err.kind(),
                        std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::PermissionDenied
                    ) =>
            {
                fs::remove_file(path)?;
                fs::rename(temp_path, path)
            }
            Err(err) => Err(err),
        }
    }

    #[cfg(not(windows))]
    {
        fs::rename(temp_path, path)
    }
}

pub(crate) fn write_bytes_atomic<P: AsRef<Path>>(path: P, bytes: &[u8]) -> Result<()> {
    let path = path.as_ref();
    let temp_path = atomic_temp_path(path);

    let write_result = (|| -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        rename_temp_into_place(&temp_path, path)?;
        Ok(())
    })();

    if let Err(err) = write_result {
        cleanup_temp_file(&temp_path);
        return Err(PlottingError::IoError(err));
    }

    Ok(())
}

pub(crate) fn write_with_atomic_writer<P, F>(path: P, writer: F) -> Result<()>
where
    P: AsRef<Path>,
    F: FnOnce(&mut BufWriter<File>) -> Result<()>,
{
    let path = path.as_ref();
    let temp_path = atomic_temp_path(path);

    let write_result = (|| -> Result<()> {
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)
            .map_err(PlottingError::IoError)?;
        let mut writer_handle = BufWriter::new(file);

        writer(&mut writer_handle)?;
        writer_handle.flush().map_err(PlottingError::IoError)?;
        let file = writer_handle
            .into_inner()
            .map_err(|err| PlottingError::IoError(err.into_error()))?;
        file.sync_all().map_err(PlottingError::IoError)?;
        drop(file);
        rename_temp_into_place(&temp_path, path).map_err(PlottingError::IoError)?;
        Ok(())
    })();

    if let Err(err) = write_result {
        cleanup_temp_file(&temp_path);
        return Err(err);
    }

    Ok(())
}
