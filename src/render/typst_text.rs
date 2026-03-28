use crate::{
    core::{PlottingError, Result},
    render::{
        Color,
        text_anchor::{TextAnchorKind, anchor_to_top_left},
    },
};
use tiny_skia::{IntSize, Pixmap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypstBackendKind {
    Raster,
    Svg,
}

#[derive(Debug)]
pub struct TypstRasterOutput {
    pub pixmap: Pixmap,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct TypstSvgOutput {
    pub svg: String,
    pub width: f32,
    pub height: f32,
}

pub fn literal_text_snippet(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("#text(\"{}\")", escaped)
}

/// Text anchor semantics used when positioning rendered Typst output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypstTextAnchor {
    /// Horizontal text anchored at the top-left corner.
    TopLeft,
    /// Horizontal text anchored at the top-center point.
    TopCenter,
    /// Text anchored at geometric center (used for rotated/centered content).
    Center,
}

/// Convert anchor coordinates into top-left draw coordinates.
pub fn anchored_top_left(
    anchor_x: f32,
    anchor_y: f32,
    rendered_width: f32,
    rendered_height: f32,
    anchor: TypstTextAnchor,
) -> (f32, f32) {
    let shared_anchor = match anchor {
        TypstTextAnchor::TopLeft => TextAnchorKind::TopLeft,
        TypstTextAnchor::TopCenter => TextAnchorKind::TopCenter,
        TypstTextAnchor::Center => TextAnchorKind::Center,
    };
    anchor_to_top_left(
        anchor_x,
        anchor_y,
        rendered_width,
        rendered_height,
        shared_anchor,
    )
}

#[cfg(not(feature = "typst-math"))]
pub fn render_raster(
    _snippet: &str,
    _size_pt: f32,
    _color: Color,
    _rotation_deg: f32,
    operation: &str,
) -> Result<TypstRasterOutput> {
    Err(PlottingError::FeatureNotEnabled {
        feature: "typst-math".to_string(),
        operation: operation.to_string(),
    })
}

#[cfg(not(feature = "typst-math"))]
pub fn render_svg(
    _snippet: &str,
    _size_pt: f32,
    _color: Color,
    _rotation_deg: f32,
    operation: &str,
) -> Result<TypstSvgOutput> {
    Err(PlottingError::FeatureNotEnabled {
        feature: "typst-math".to_string(),
        operation: operation.to_string(),
    })
}

#[cfg(not(feature = "typst-math"))]
pub fn measure_text(
    _snippet: &str,
    _size_pt: f32,
    _color: Color,
    _rotation_deg: f32,
    _backend: TypstBackendKind,
    operation: &str,
) -> Result<(f32, f32)> {
    Err(PlottingError::FeatureNotEnabled {
        feature: "typst-math".to_string(),
        operation: operation.to_string(),
    })
}

#[cfg(feature = "typst-math")]
mod imp {
    use super::{TypstBackendKind, TypstRasterOutput, TypstSvgOutput};
    use crate::{
        core::{PlottingError, Result},
        render::Color,
    };
    use std::{
        collections::HashMap,
        path::PathBuf,
        sync::{Mutex, MutexGuard, OnceLock},
    };
    use tiny_skia::{IntSize, Pixmap};
    use typst::{
        Library, World, compile,
        diag::FileError,
        foundations::{Bytes, Datetime},
        layout::{Page, PagedDocument},
        syntax::{FileId, Source, VirtualPath},
        text::{Font, FontBook},
        utils::LazyHash,
    };
    use typst_kit::fonts::{FontSearcher, FontSlot};

    const MAX_CACHE_ENTRIES: usize = 256;
    const MAX_CACHE_BYTES: usize = 64 * 1024 * 1024;
    const MAX_RASTER_DIMENSION: u32 = 8_192;
    const MAX_RASTER_BYTES: usize = 128 * 1024 * 1024;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct CacheKey {
        snippet: String,
        size_bits: u32,
        color: (u8, u8, u8, u8),
        rotation_bits: u32,
        backend: TypstBackendKind,
    }

    #[derive(Debug, Clone)]
    enum CachedValue {
        Raster {
            pixels: Vec<u8>,
            pixel_width: u32,
            pixel_height: u32,
            logical_width: f32,
            logical_height: f32,
        },
        Svg {
            svg: String,
            width: f32,
            height: f32,
        },
    }

    #[derive(Debug, Default)]
    struct CacheState {
        entries: HashMap<CacheKey, CachedValue>,
        total_bytes: usize,
    }

    #[derive(Debug)]
    struct FontContext {
        book: LazyHash<FontBook>,
        fonts: Vec<FontSlot>,
        sans_family: String,
    }

    #[derive(Debug)]
    struct TypstWorld {
        library: &'static LazyHash<Library>,
        book: &'static LazyHash<FontBook>,
        fonts: &'static [FontSlot],
        main: FileId,
        source: Source,
    }

    impl World for TypstWorld {
        fn library(&self) -> &LazyHash<Library> {
            self.library
        }

        fn book(&self) -> &LazyHash<FontBook> {
            self.book
        }

        fn main(&self) -> FileId {
            self.main
        }

        fn source(&self, id: FileId) -> typst::diag::FileResult<Source> {
            if id == self.main {
                Ok(self.source.clone())
            } else {
                Err(FileError::NotFound(PathBuf::from("<memory>")))
            }
        }

        fn file(&self, _id: FileId) -> typst::diag::FileResult<Bytes> {
            Err(FileError::NotFound(PathBuf::from("<memory>")))
        }

        fn font(&self, index: usize) -> Option<Font> {
            self.fonts.get(index).and_then(FontSlot::get)
        }

        fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
            None
        }
    }

    fn library() -> &'static LazyHash<Library> {
        static LIB: OnceLock<LazyHash<Library>> = OnceLock::new();
        LIB.get_or_init(|| LazyHash::new(Library::default()))
    }

    fn fonts() -> &'static FontContext {
        static FONTS: OnceLock<FontContext> = OnceLock::new();
        FONTS.get_or_init(|| {
            let mut searcher = FontSearcher::new();
            let found = searcher.search();
            let sans_family = select_sans_family(&found.book);
            FontContext {
                book: LazyHash::new(found.book),
                fonts: found.fonts,
                sans_family,
            }
        })
    }

    fn select_sans_family(book: &FontBook) -> String {
        const PREFERRED: &[&str] = &[
            "noto sans",
            "dejavu sans",
            "liberation sans",
            "arial",
            "helvetica",
            "new computer modern sans",
            "latin modern sans",
        ];

        for candidate in PREFERRED {
            if book.contains_family(candidate) {
                return (*candidate).to_string();
            }
        }

        let mut first_family: Option<String> = None;
        for (family, _) in book.families() {
            if first_family.is_none() {
                first_family = Some(family.to_string());
            }
            if family.to_ascii_lowercase().contains("sans") {
                return family.to_string();
            }
        }

        first_family.unwrap_or_else(|| "New Computer Modern Sans".to_string())
    }

    fn escape_typst_string(value: &str) -> String {
        value.replace('\\', "\\\\").replace('"', "\\\"")
    }

    fn cache() -> &'static Mutex<CacheState> {
        static CACHE: OnceLock<Mutex<CacheState>> = OnceLock::new();
        CACHE.get_or_init(|| Mutex::new(CacheState::default()))
    }

    fn lock_cache_resource<'a, T>(
        mutex: &'a Mutex<T>,
        resource_name: &str,
    ) -> Result<MutexGuard<'a, T>> {
        mutex.lock().map_err(|_| {
            PlottingError::TypstError(format!(
                "Typst rendering aborted because {resource_name} lock is poisoned"
            ))
        })
    }

    fn lock_cache() -> Result<MutexGuard<'static, CacheState>> {
        lock_cache_resource(cache(), "Typst cache")
    }

    fn make_key(
        snippet: &str,
        size_pt: f32,
        color: Color,
        rotation_deg: f32,
        backend: TypstBackendKind,
    ) -> CacheKey {
        CacheKey {
            snippet: snippet.to_string(),
            size_bits: size_pt.to_bits(),
            color: (color.r, color.g, color.b, color.a),
            rotation_bits: rotation_deg.to_bits(),
            backend,
        }
    }

    fn cached_value_bytes(value: &CachedValue) -> usize {
        match value {
            CachedValue::Raster { pixels, .. } => pixels.len(),
            CachedValue::Svg { svg, .. } => svg.len(),
        }
    }

    fn maybe_evict(cache: &mut CacheState, incoming_bytes: usize) {
        // Evict in arbitrary HashMap iteration order; for this small local render
        // cache that trade-off is acceptable. A single near-limit item can drain
        // the cache to make room for itself.
        while cache.entries.len() >= MAX_CACHE_ENTRIES
            || (!cache.entries.is_empty()
                && cache.total_bytes.saturating_add(incoming_bytes) > MAX_CACHE_BYTES)
        {
            if let Some(first_key) = cache.entries.keys().next().cloned() {
                if let Some(removed) = cache.entries.remove(&first_key) {
                    cache.total_bytes = cache
                        .total_bytes
                        .saturating_sub(cached_value_bytes(&removed));
                }
            } else {
                break;
            }
        }
    }

    fn remove_cached_value(cache: &mut CacheState, key: &CacheKey) {
        if let Some(previous) = cache.entries.remove(key) {
            cache.total_bytes = cache
                .total_bytes
                .saturating_sub(cached_value_bytes(&previous));
        }
    }

    fn insert_cached_value(cache: &mut CacheState, key: CacheKey, value: CachedValue) {
        let incoming_bytes = cached_value_bytes(&value);
        if incoming_bytes > MAX_CACHE_BYTES {
            remove_cached_value(cache, &key);
            return;
        }

        remove_cached_value(cache, &key);

        maybe_evict(cache, incoming_bytes);
        cache.total_bytes = cache.total_bytes.saturating_add(incoming_bytes);
        cache.entries.insert(key, value);
    }

    fn validate_raster_size(width: u32, height: u32, operation: &str) -> Result<()> {
        if width > MAX_RASTER_DIMENSION || height > MAX_RASTER_DIMENSION {
            return Err(PlottingError::PerformanceLimit {
                limit_type: format!("{operation} raster dimension"),
                actual: width.max(height) as usize,
                maximum: MAX_RASTER_DIMENSION as usize,
            });
        }

        let bytes = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| PlottingError::PerformanceLimit {
                limit_type: format!("{operation} raster bytes"),
                actual: usize::MAX,
                maximum: MAX_RASTER_BYTES,
            })?;

        if bytes > MAX_RASTER_BYTES {
            return Err(PlottingError::PerformanceLimit {
                limit_type: format!("{operation} raster bytes"),
                actual: bytes,
                maximum: MAX_RASTER_BYTES,
            });
        }

        Ok(())
    }

    fn snippet_excerpt(snippet: &str) -> String {
        let compact = snippet.trim().replace('\n', " ");
        let mut chars = compact.chars();
        let excerpt: String = chars.by_ref().take(80).collect();
        if chars.next().is_some() {
            format!("{}...", excerpt)
        } else {
            excerpt
        }
    }

    fn build_document_source(
        snippet: &str,
        size_pt: f32,
        color: Color,
        rotation_deg: f32,
        font_family: &str,
    ) -> String {
        let size_pt = size_pt.max(1.0);
        let font_family = escape_typst_string(font_family);
        if rotation_deg.abs() > f32::EPSILON {
            format!(
                "#set page(width: auto, height: auto, margin: 0pt, fill: none)\n#set text(font: \"{font_family}\", size: {size_pt}pt, fill: rgb({r}, {g}, {b}, {a}), top-edge: \"ascender\", bottom-edge: \"descender\")\n#rotate({rotation_deg}deg, reflow: true)[{snippet}]",
                r = color.r,
                g = color.g,
                b = color.b,
                a = color.a,
            )
        } else {
            format!(
                "#set page(width: auto, height: auto, margin: 0pt, fill: none)\n#set text(font: \"{font_family}\", size: {size_pt}pt, fill: rgb({r}, {g}, {b}, {a}), top-edge: \"ascender\", bottom-edge: \"descender\")\n{snippet}",
                r = color.r,
                g = color.g,
                b = color.b,
                a = color.a,
            )
        }
    }

    fn compile_single_page(
        snippet: &str,
        size_pt: f32,
        color: Color,
        rotation_deg: f32,
        operation: &str,
    ) -> Result<Page> {
        let font_ctx = fonts();
        let source_text =
            build_document_source(snippet, size_pt, color, rotation_deg, &font_ctx.sans_family);
        let main = FileId::new_fake(VirtualPath::new("/main.typ"));
        let source = Source::new(main, source_text);
        let world = TypstWorld {
            library: library(),
            book: &font_ctx.book,
            fonts: &font_ctx.fonts,
            main,
            source,
        };

        let warned = compile::<PagedDocument>(&world);
        let document = warned.output.map_err(|errors| {
            let details = errors
                .iter()
                .map(|diag| diag.message.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            PlottingError::TypstError(format!(
                "{} failed: {}; snippet=`{}`",
                operation,
                details,
                snippet_excerpt(snippet)
            ))
        })?;

        document.pages.first().cloned().ok_or_else(|| {
            PlottingError::TypstError(format!(
                "{} failed: Typst produced no pages; snippet=`{}`",
                operation,
                snippet_excerpt(snippet)
            ))
        })
    }

    pub fn render_raster(
        snippet: &str,
        size_pt: f32,
        color: Color,
        rotation_deg: f32,
        operation: &str,
    ) -> Result<TypstRasterOutput> {
        if snippet.trim().is_empty() {
            let pixmap = Pixmap::new(1, 1).ok_or_else(|| {
                PlottingError::RenderError("Failed to allocate pixmap".to_string())
            })?;
            return Ok(TypstRasterOutput {
                pixmap,
                width: 0.0,
                height: 0.0,
            });
        }

        let key = make_key(
            snippet,
            size_pt,
            color,
            rotation_deg,
            TypstBackendKind::Raster,
        );

        {
            let cache = lock_cache()?;
            if let Some(CachedValue::Raster {
                pixels,
                pixel_width,
                pixel_height,
                logical_width,
                logical_height,
            }) = cache.entries.get(&key)
            {
                let size = IntSize::from_wh(*pixel_width, *pixel_height).ok_or_else(|| {
                    PlottingError::RenderError("Invalid cached typst raster size".to_string())
                })?;
                let pixmap = Pixmap::from_vec(pixels.clone(), size).ok_or_else(|| {
                    PlottingError::RenderError(
                        "Failed to create pixmap from cached Typst raster".to_string(),
                    )
                })?;
                return Ok(TypstRasterOutput {
                    pixmap,
                    width: *logical_width,
                    height: *logical_height,
                });
            }
        }

        let page = compile_single_page(snippet, size_pt, color, rotation_deg, operation)?;
        let size = page.frame.size();
        let logical_width = size.x.to_pt() as f32;
        let logical_height = size.y.to_pt() as f32;
        let expected_width = logical_width.ceil().max(1.0) as u32;
        let expected_height = logical_height.ceil().max(1.0) as u32;
        validate_raster_size(expected_width, expected_height, operation)?;

        let rendered_pixmap = typst_render::render(&page, 1.0);
        let pixel_width = rendered_pixmap.width();
        let pixel_height = rendered_pixmap.height();
        validate_raster_size(pixel_width, pixel_height, operation)?;
        let size = IntSize::from_wh(pixel_width, pixel_height).ok_or_else(|| {
            PlottingError::RenderError("Typst raster output has invalid dimensions".to_string())
        })?;
        let pixmap = Pixmap::from_vec(rendered_pixmap.data().to_vec(), size).ok_or_else(|| {
            PlottingError::RenderError("Failed to convert Typst raster output".to_string())
        })?;
        let pixels = pixmap.data().to_vec();
        let pixel_bytes = pixels.len();
        let mut cache = lock_cache()?;
        if pixel_bytes > MAX_CACHE_BYTES {
            remove_cached_value(&mut cache, &key);
        } else {
            insert_cached_value(
                &mut cache,
                key,
                CachedValue::Raster {
                    pixels,
                    pixel_width,
                    pixel_height,
                    logical_width,
                    logical_height,
                },
            );
        }

        Ok(TypstRasterOutput {
            pixmap,
            width: logical_width,
            height: logical_height,
        })
    }

    pub fn render_svg(
        snippet: &str,
        size_pt: f32,
        color: Color,
        rotation_deg: f32,
        operation: &str,
    ) -> Result<TypstSvgOutput> {
        if snippet.trim().is_empty() {
            return Ok(TypstSvgOutput {
                svg: String::new(),
                width: 0.0,
                height: 0.0,
            });
        }

        let key = make_key(snippet, size_pt, color, rotation_deg, TypstBackendKind::Svg);
        {
            let cache = lock_cache()?;
            if let Some(CachedValue::Svg { svg, width, height }) = cache.entries.get(&key) {
                return Ok(TypstSvgOutput {
                    svg: svg.clone(),
                    width: *width,
                    height: *height,
                });
            }
        }

        let page = compile_single_page(snippet, size_pt, color, rotation_deg, operation)?;
        let raw_svg = typst_svg::svg(&page);
        let size = page.frame.size();
        let width = size.x.to_pt() as f32;
        let height = size.y.to_pt() as f32;

        let mut cache = lock_cache()?;
        if raw_svg.len() > MAX_CACHE_BYTES {
            remove_cached_value(&mut cache, &key);
        } else {
            insert_cached_value(
                &mut cache,
                key,
                CachedValue::Svg {
                    svg: raw_svg.clone(),
                    width,
                    height,
                },
            );
        }

        Ok(TypstSvgOutput {
            svg: raw_svg,
            width,
            height,
        })
    }

    pub fn measure_text(
        snippet: &str,
        size_pt: f32,
        color: Color,
        rotation_deg: f32,
        backend: TypstBackendKind,
        operation: &str,
    ) -> Result<(f32, f32)> {
        match backend {
            TypstBackendKind::Raster => {
                let rendered = render_raster(snippet, size_pt, color, rotation_deg, operation)?;
                Ok((rendered.width, rendered.height))
            }
            TypstBackendKind::Svg => {
                let rendered = render_svg(snippet, size_pt, color, rotation_deg, operation)?;
                Ok((rendered.width, rendered.height))
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn poisoned_typst_cache_lock_returns_error() {
            let mutex = Mutex::new(0_u8);
            let _ = std::panic::catch_unwind(|| {
                let _guard = mutex.lock().unwrap();
                panic!("poison typst cache lock");
            });

            let err = lock_cache_resource(&mutex, "test cache").unwrap_err();
            assert!(matches!(err, PlottingError::TypstError(_)));
            assert!(err.to_string().contains("test cache lock is poisoned"));
        }

        #[test]
        fn cache_insert_tracks_bytes_without_rescanning() {
            let mut cache = CacheState::default();
            let key = make_key(
                "#text(\"a\")",
                12.0,
                Color::BLACK,
                0.0,
                TypstBackendKind::Svg,
            );
            let svg = "<svg>abc</svg>".to_string();
            let bytes = svg.len();

            insert_cached_value(
                &mut cache,
                key.clone(),
                CachedValue::Svg {
                    svg,
                    width: 12.0,
                    height: 8.0,
                },
            );

            assert_eq!(cache.total_bytes, bytes);
            assert!(cache.entries.contains_key(&key));
        }

        #[test]
        fn cache_insert_evicts_stale_entry_when_replacement_is_too_large() {
            let mut cache = CacheState::default();
            let key = make_key(
                "#text(\"grow\")",
                12.0,
                Color::BLACK,
                0.0,
                TypstBackendKind::Svg,
            );
            let small_svg = "<svg>small</svg>".to_string();
            let small_bytes = small_svg.len();

            insert_cached_value(
                &mut cache,
                key.clone(),
                CachedValue::Svg {
                    svg: small_svg,
                    width: 12.0,
                    height: 8.0,
                },
            );

            assert_eq!(cache.total_bytes, small_bytes);
            assert!(cache.entries.contains_key(&key));

            insert_cached_value(
                &mut cache,
                key.clone(),
                CachedValue::Svg {
                    svg: "x".repeat(MAX_CACHE_BYTES + 1),
                    width: 12.0,
                    height: 8.0,
                },
            );

            assert_eq!(cache.total_bytes, 0);
            assert!(!cache.entries.contains_key(&key));
        }

        #[test]
        fn oversized_render_path_evicts_stale_entry_even_without_recaching() {
            let mut cache = CacheState::default();
            let key = make_key(
                "#text(\"grow\")",
                12.0,
                Color::BLACK,
                0.0,
                TypstBackendKind::Svg,
            );

            insert_cached_value(
                &mut cache,
                key.clone(),
                CachedValue::Svg {
                    svg: "<svg>small</svg>".to_string(),
                    width: 12.0,
                    height: 8.0,
                },
            );

            let oversized_svg = "x".repeat(MAX_CACHE_BYTES + 1);
            if oversized_svg.len() > MAX_CACHE_BYTES {
                remove_cached_value(&mut cache, &key);
            } else {
                insert_cached_value(
                    &mut cache,
                    key.clone(),
                    CachedValue::Svg {
                        svg: oversized_svg,
                        width: 12.0,
                        height: 8.0,
                    },
                );
            }

            assert_eq!(cache.total_bytes, 0);
            assert!(!cache.entries.contains_key(&key));
        }
    }
}

#[cfg(feature = "typst-math")]
pub use imp::{measure_text, render_raster, render_svg};
