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

#[cfg(windows)]
use std::{iter::once, os::windows::ffi::OsStrExt};

#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_REPLACE_EXISTING, MoveFileExW};

const TEMP_FILE_CREATE_RETRIES: usize = 8;

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

fn resolve_atomic_destination(path: &Path) -> std::io::Result<PathBuf> {
    #[cfg(unix)]
    {
        let mut current = path.to_path_buf();
        let mut hops = 0usize;

        loop {
            match fs::symlink_metadata(&current) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    if hops >= 16 {
                        return Err(std::io::Error::other(
                            "too many symlink levels while resolving export destination",
                        ));
                    }

                    let target = fs::read_link(&current)?;
                    current = if target.is_absolute() {
                        target
                    } else {
                        current
                            .parent()
                            .unwrap_or_else(|| Path::new("."))
                            .join(target)
                    };
                    hops += 1;
                }
                Ok(_) => return Ok(current),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(current),
                Err(err) => return Err(err),
            }
        }
    }

    #[cfg(not(unix))]
    {
        Ok(path.to_path_buf())
    }
}

struct AtomicWriteFailure {
    error: PlottingError,
    cleanup_temp: bool,
}

impl AtomicWriteFailure {
    fn cleanup(error: PlottingError) -> Self {
        Self {
            error,
            cleanup_temp: true,
        }
    }

    fn preserve_temp(error: PlottingError) -> Self {
        Self {
            error,
            cleanup_temp: false,
        }
    }
}

fn create_atomic_temp_file(path: &Path) -> std::io::Result<(PathBuf, File)> {
    let mut last_err = None;

    for _ in 0..TEMP_FILE_CREATE_RETRIES {
        let temp_path = atomic_temp_path(path);
        match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)
        {
            Ok(file) => return Ok((temp_path, file)),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                // Stale temp file from a previous crash with the same PID.
                // No live process can share our PID, so removing it is safe.
                cleanup_temp_file(&temp_path);
                last_err = Some(err);
            }
            Err(err) => return Err(err),
        }
    }

    Err(last_err.unwrap_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "failed to allocate a unique temporary export file",
        )
    }))
}

fn rename_temp_into_place(
    temp_path: &Path,
    path: &Path,
) -> std::result::Result<(), AtomicWriteFailure> {
    #[cfg(windows)]
    {
        let temp_wide: Vec<u16> = temp_path.as_os_str().encode_wide().chain(once(0)).collect();
        let path_wide: Vec<u16> = path.as_os_str().encode_wide().chain(once(0)).collect();

        // MOVEFILE_REPLACE_EXISTING preserves the destination if the replacement fails,
        // avoiding the delete-then-rename data-loss window from the previous fallback.
        let replace_result = unsafe {
            MoveFileExW(
                temp_wide.as_ptr(),
                path_wide.as_ptr(),
                MOVEFILE_REPLACE_EXISTING,
            )
        };

        if replace_result != 0 {
            Ok(())
        } else {
            let err = std::io::Error::last_os_error();
            Err(AtomicWriteFailure::preserve_temp(PlottingError::IoError(
                std::io::Error::new(
                    err.kind(),
                    format!(
                        "failed to replace {} with {}; the temporary file has been preserved for recovery",
                        path.display(),
                        temp_path.display()
                    ),
                ),
            )))
        }
    }

    #[cfg(not(windows))]
    {
        fs::rename(temp_path, path)
            .map_err(|err| AtomicWriteFailure::cleanup(PlottingError::IoError(err)))
    }
}

pub(crate) fn write_bytes_atomic<P: AsRef<Path>>(path: P, bytes: &[u8]) -> Result<()> {
    let path = path.as_ref();
    let destination_path = resolve_atomic_destination(path).map_err(PlottingError::IoError)?;
    let (temp_path, mut file) =
        create_atomic_temp_file(&destination_path).map_err(PlottingError::IoError)?;

    let write_result = (|| -> std::result::Result<(), AtomicWriteFailure> {
        file.write_all(bytes)
            .map_err(|err| AtomicWriteFailure::cleanup(PlottingError::IoError(err)))?;
        file.sync_all()
            .map_err(|err| AtomicWriteFailure::cleanup(PlottingError::IoError(err)))?;
        drop(file);
        rename_temp_into_place(&temp_path, &destination_path)?;
        Ok(())
    })();

    if let Err(failure) = write_result {
        if failure.cleanup_temp {
            cleanup_temp_file(&temp_path);
        }
        return Err(failure.error);
    }

    Ok(())
}

pub(crate) fn write_with_atomic_writer<P, F>(path: P, writer: F) -> Result<()>
where
    P: AsRef<Path>,
    F: FnOnce(&mut BufWriter<File>) -> Result<()>,
{
    let path = path.as_ref();
    let destination_path = resolve_atomic_destination(path).map_err(PlottingError::IoError)?;
    let (temp_path, file) =
        create_atomic_temp_file(&destination_path).map_err(PlottingError::IoError)?;

    let write_result = (|| -> std::result::Result<(), AtomicWriteFailure> {
        let mut writer_handle = BufWriter::new(file);

        writer(&mut writer_handle).map_err(AtomicWriteFailure::cleanup)?;
        writer_handle
            .flush()
            .map_err(|err| AtomicWriteFailure::cleanup(PlottingError::IoError(err)))?;
        let file = writer_handle
            .into_inner()
            .map_err(|err| AtomicWriteFailure::cleanup(PlottingError::IoError(err.into_error())))?;
        file.sync_all()
            .map_err(|err| AtomicWriteFailure::cleanup(PlottingError::IoError(err)))?;
        drop(file);
        rename_temp_into_place(&temp_path, &destination_path)?;
        Ok(())
    })();

    if let Err(failure) = write_result {
        if failure.cleanup_temp {
            cleanup_temp_file(&temp_path);
        }
        return Err(failure.error);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn atomic_write_preserves_symlink_and_updates_target() {
        use std::os::unix::fs::symlink;

        let tempdir = tempfile::tempdir().expect("tempdir");
        let runs_dir = tempdir.path().join("runs");
        fs::create_dir(&runs_dir).expect("runs dir");

        let target_path = runs_dir.join("42.png");
        fs::write(&target_path, b"old-bytes").expect("seed target");

        let link_path = tempdir.path().join("latest.png");
        symlink(Path::new("runs/42.png"), &link_path).expect("create symlink");

        write_bytes_atomic(&link_path, b"new-bytes").expect("atomic write through symlink");

        assert!(
            fs::symlink_metadata(&link_path)
                .expect("symlink metadata")
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            fs::read_link(&link_path).expect("read symlink"),
            PathBuf::from("runs/42.png")
        );
        assert_eq!(fs::read(&target_path).expect("read target"), b"new-bytes");
    }
}
