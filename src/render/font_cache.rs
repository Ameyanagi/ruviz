//! Disk cache for the system font database.
//!
//! Discovering system fonts is the single largest cost of the first render in
//! a process: `fontdb`'s directory walk takes ~60-70ms on a typical macOS
//! install, while re-loading the same font files from a cached path list takes
//! ~9ms — and re-parsing goes through exactly the same `fontdb` loader, so the
//! resulting database is built by the same code either way. Only the
//! *discovery* of which files exist is cached, never their parsed contents.
//!
//! The cache is invalidated by any change to the cached font files (path,
//! mtime, size), to the directories that contain them, or to the platform
//! font roots that `fontdb` scans, in which case a full scan runs and the
//! cache is rewritten. A cache miss can therefore only ever cost one extra
//! scan; it can not resolve fonts differently from an uncached run. File
//! order is preserved from the original scan so duplicate-family
//! tie-breaking matches the uncached database.
//!
//! Set `RUVIZ_FONT_CACHE=0` to disable the cache entirely.

#![cfg(not(target_arch = "wasm32"))]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use cosmic_text::fontdb::{Database, Source};

const CACHE_HEADER: &str = "ruviz-font-cache\tv1";

/// Build the system font database, through the cache when possible.
///
/// Equivalent to `fontdb::Database::load_system_fonts()` in every observable
/// way; the cache only skips the directory walk.
pub(crate) fn system_font_database() -> Database {
    if std::env::var_os("RUVIZ_FONT_CACHE").is_some_and(|v| v == "0") {
        return scanned_database();
    }
    let Some(cache_path) = cache_file_path() else {
        return scanned_database();
    };

    if let Some(database) = load_from_cache(&cache_path) {
        return database;
    }

    let database = scanned_database();
    // Best effort: a failed write just means the next process scans again.
    let _ = write_cache(&cache_path, &database);
    database
}

fn scanned_database() -> Database {
    let mut database = Database::new();
    database.load_system_fonts();
    database
}

fn cache_file_path() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("RUVIZ_FONT_CACHE") {
        // Any value other than the "0" kill switch names a directory.
        return Some(PathBuf::from(dir).join("font-cache-v1.tsv"));
    }
    let base = if cfg!(target_os = "macos") {
        PathBuf::from(std::env::var_os("HOME")?).join("Library/Caches")
    } else if cfg!(windows) {
        PathBuf::from(std::env::var_os("LOCALAPPDATA")?)
    } else {
        match std::env::var_os("XDG_CACHE_HOME") {
            Some(xdg) if !xdg.is_empty() => PathBuf::from(xdg),
            _ => PathBuf::from(std::env::var_os("HOME")?).join(".cache"),
        }
    };
    Some(base.join("ruviz").join("font-cache-v1.tsv"))
}

/// The mtime of a path, as nanoseconds since the epoch, or `-` when absent.
///
/// Absence is a valid, cacheable state: a font root that did not exist at
/// scan time must still not exist for the cache to hold.
fn mtime_stamp(path: &Path) -> String {
    match fs::metadata(path).and_then(|m| m.modified()) {
        Ok(time) => match time.duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => d.as_nanos().to_string(),
            Err(_) => "-".to_string(),
        },
        Err(_) => "-".to_string(),
    }
}

fn file_stamp(path: &Path) -> Option<(String, u64)> {
    let meta = fs::metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos()
        .to_string();
    Some((mtime, meta.len()))
}

/// The directories `fontdb` 0.21 scans on this platform (pinned in
/// `Cargo.lock`). Kept in sync manually; drift can only make the cache
/// mismatch and rescan, never resolve differently.
fn platform_font_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if cfg!(target_os = "macos") {
        roots.push(PathBuf::from("/Library/Fonts"));
        roots.push(PathBuf::from("/System/Library/Fonts"));
        roots.push(PathBuf::from("/System/Library/AssetsV2"));
        roots.push(PathBuf::from("/Network/Library/Fonts"));
        if let Some(home) = std::env::var_os("HOME") {
            roots.push(PathBuf::from(home).join("Library/Fonts"));
        }
    } else if cfg!(windows) {
        if let Some(system_root) = std::env::var_os("SYSTEMROOT") {
            roots.push(PathBuf::from(system_root).join("Fonts"));
        } else {
            roots.push(PathBuf::from("C:\\Windows\\Fonts"));
        }
        if let Some(profile) = std::env::var_os("USERPROFILE") {
            let profile = PathBuf::from(profile);
            roots.push(profile.join("AppData\\Local\\Microsoft\\Windows\\Fonts"));
            roots.push(profile.join("AppData\\Roaming\\Microsoft\\Windows\\Fonts"));
        }
    } else {
        roots.push(PathBuf::from("/usr/share/fonts"));
        roots.push(PathBuf::from("/usr/local/share/fonts"));
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            roots.push(home.join(".fonts"));
            match std::env::var_os("XDG_DATA_HOME") {
                Some(xdg) if !xdg.is_empty() => roots.push(PathBuf::from(xdg).join("fonts")),
                _ => roots.push(home.join(".local/share/fonts")),
            }
        }
    }
    roots
}

/// Every directory whose mtime guards this cache: the platform roots, the
/// downloadable-font asset dirs beneath them, and the parents of every
/// cached font file.
fn guard_directories(files: &[PathBuf]) -> Vec<PathBuf> {
    let mut dirs = BTreeSet::new();
    for root in platform_font_roots() {
        if cfg!(target_os = "macos")
            && root.ends_with("AssetsV2")
            && let Ok(entries) = fs::read_dir(&root)
        {
            for entry in entries.flatten() {
                if entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("com_apple_MobileAsset_Font")
                {
                    dirs.insert(entry.path());
                }
            }
        }
        dirs.insert(root);
    }
    for file in files {
        if let Some(parent) = file.parent() {
            dirs.insert(parent.to_path_buf());
        }
    }
    dirs.into_iter().collect()
}

fn field_is_cacheable(text: &str) -> bool {
    !text.contains('\t') && !text.contains('\n') && !text.contains('\r')
}

fn write_cache(cache_path: &Path, database: &Database) -> std::io::Result<()> {
    // Preserve the scan's insertion order, deduplicated: collections
    // contribute several faces from one file.
    let mut files: Vec<PathBuf> = Vec::new();
    let mut seen: BTreeSet<PathBuf> = BTreeSet::new();
    for face in database.faces() {
        match &face.source {
            Source::File(path) => {
                if seen.insert(path.clone()) {
                    files.push(path.clone());
                }
            }
            // A non-file source can not be revalidated from disk; do not cache.
            _ => return Ok(()),
        }
    }

    let mut out = String::new();
    out.push_str(CACHE_HEADER);
    out.push('\n');
    out.push_str(&format!("faces\t{}\n", database.len()));
    for dir in guard_directories(&files) {
        let Some(text) = dir.to_str() else {
            return Ok(());
        };
        if !field_is_cacheable(text) {
            return Ok(());
        }
        out.push_str(&format!("dir\t{}\t{}\n", mtime_stamp(&dir), text));
    }
    for file in &files {
        let Some(text) = file.to_str() else {
            return Ok(());
        };
        if !field_is_cacheable(text) {
            return Ok(());
        }
        let Some((mtime, size)) = file_stamp(file) else {
            return Ok(());
        };
        out.push_str(&format!("file\t{mtime}\t{size}\t{text}\n"));
    }

    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = cache_path.with_extension("tmp");
    fs::write(&tmp, out)?;
    fs::rename(&tmp, cache_path)
}

fn load_from_cache(cache_path: &Path) -> Option<Database> {
    let content = fs::read_to_string(cache_path).ok()?;
    let mut lines = content.lines();
    if lines.next()? != CACHE_HEADER {
        return None;
    }
    let expected_faces: usize = lines.next()?.strip_prefix("faces\t")?.parse().ok()?;

    let mut files: Vec<PathBuf> = Vec::new();
    for line in lines {
        let mut fields = line.split('\t');
        match fields.next()? {
            "dir" => {
                let recorded = fields.next()?;
                let path = Path::new(fields.next()?);
                if mtime_stamp(path) != recorded {
                    return None;
                }
            }
            "file" => {
                let recorded_mtime = fields.next()?;
                let recorded_size: u64 = fields.next()?.parse().ok()?;
                let path = PathBuf::from(fields.next()?);
                let (mtime, size) = file_stamp(&path)?;
                if mtime != recorded_mtime || size != recorded_size {
                    return None;
                }
                files.push(path);
            }
            _ => return None,
        }
    }

    let mut database = Database::new();
    for file in &files {
        database.load_font_file(file).ok()?;
    }
    if database.len() != expected_faces {
        return None;
    }
    Some(database)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cached database must be indistinguishable from a fresh scan:
    /// same faces, same order, same sources.
    #[test]
    fn cache_round_trip_reproduces_the_scanned_database() {
        let scanned = scanned_database();
        let dir =
            std::env::temp_dir().join(format!("ruviz-font-cache-test-{}", std::process::id()));
        let cache_path = dir.join("font-cache-v1.tsv");
        write_cache(&cache_path, &scanned).expect("cache write");

        let rebuilt = load_from_cache(&cache_path).expect("cache load");
        let fingerprint = |db: &Database| {
            db.faces()
                .map(|f| {
                    (
                        match &f.source {
                            Source::File(p) => p.clone(),
                            _ => PathBuf::new(),
                        },
                        f.index,
                        f.post_script_name.clone(),
                        f.weight,
                        f.style,
                        f.stretch,
                        f.monospaced,
                        f.families.clone(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(fingerprint(&scanned), fingerprint(&rebuilt));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn stale_file_stamp_invalidates_the_cache() {
        let scanned = scanned_database();
        let dir =
            std::env::temp_dir().join(format!("ruviz-font-stale-test-{}", std::process::id()));
        let cache_path = dir.join("font-cache-v1.tsv");
        write_cache(&cache_path, &scanned).expect("cache write");

        let content = fs::read_to_string(&cache_path).expect("read cache");
        let tampered: String = content
            .lines()
            .map(|line| {
                if let Some(rest) = line.strip_prefix("file\t") {
                    format!("file\t0{rest}\n")
                } else {
                    format!("{line}\n")
                }
            })
            .collect();
        fs::write(&cache_path, tampered).expect("tamper cache");

        assert!(load_from_cache(&cache_path).is_none());
        let _ = fs::remove_dir_all(&dir);
    }
}
