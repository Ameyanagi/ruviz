use crate::{
    core::{PlottingError, Result},
    render::{
        Color, FontFamily,
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
pub fn render_raster_with_font_family(
    _snippet: &str,
    _size_pt: f32,
    _color: Color,
    _rotation_deg: f32,
    _font_family: &FontFamily,
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
pub fn render_svg_with_font_family(
    _snippet: &str,
    _size_pt: f32,
    _color: Color,
    _rotation_deg: f32,
    _font_family: &FontFamily,
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

#[cfg(not(feature = "typst-math"))]
pub fn measure_text_with_font_family(
    _snippet: &str,
    _size_pt: f32,
    _color: Color,
    _rotation_deg: f32,
    _backend: TypstBackendKind,
    _font_family: &FontFamily,
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
        render::{Color, FontFamily, font_registry},
    };
    use std::{
        collections::HashMap,
        path::PathBuf,
        sync::{Arc, Mutex, MutexGuard, OnceLock},
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
    use typst_kit::fonts::{FontSearcher, FontSlot as SearcherFontSlot};

    const MAX_CACHE_ENTRIES: usize = 256;
    const MAX_CACHE_BYTES: usize = 64 * 1024 * 1024;
    const MAX_RASTER_DIMENSION: u32 = 8_192;
    const MAX_RASTER_BYTES: usize = 128 * 1024 * 1024;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct CacheKey {
        font_generation: u64,
        snippet: String,
        size_bits: u32,
        color: (u8, u8, u8, u8),
        rotation_bits: u32,
        backend: TypstBackendKind,
        font_family: String,
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
        font_generation: u64,
        entries: HashMap<CacheKey, CachedValue>,
        total_bytes: usize,
    }

    #[derive(Debug)]
    enum ContextFontSlot {
        Searcher(SearcherFontSlot),
        Registered(Font),
    }

    impl ContextFontSlot {
        fn get(&self) -> Option<Font> {
            match self {
                Self::Searcher(slot) => slot.get(),
                Self::Registered(font) => Some(font.clone()),
            }
        }
    }

    #[derive(Debug)]
    struct FontContext {
        generation: u64,
        book: LazyHash<FontBook>,
        fonts: Vec<ContextFontSlot>,
        sans_family: String,
        serif_family: String,
        mono_family: String,
    }

    #[derive(Debug)]
    struct TypstWorld<'a> {
        library: &'static LazyHash<Library>,
        book: &'a LazyHash<FontBook>,
        fonts: &'a [ContextFontSlot],
        main: FileId,
        source: Source,
    }

    impl World for TypstWorld<'_> {
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
            self.fonts.get(index).and_then(ContextFontSlot::get)
        }

        fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
            None
        }
    }

    fn library() -> &'static LazyHash<Library> {
        static LIB: OnceLock<LazyHash<Library>> = OnceLock::new();
        LIB.get_or_init(|| LazyHash::new(Library::default()))
    }

    fn font_context() -> Result<Arc<FontContext>> {
        static FONTS: OnceLock<Mutex<Option<Arc<FontContext>>>> = OnceLock::new();

        let snapshot = font_registry::snapshot()?;
        let contexts = FONTS.get_or_init(|| Mutex::new(None));
        let mut context = lock_cache_resource(contexts, "Typst font context")?;
        if let Some(existing) = context.as_ref()
            && existing.generation >= snapshot.generation
        {
            return Ok(existing.clone());
        }

        let rebuilt = Arc::new(build_font_context(snapshot));
        *context = Some(rebuilt.clone());
        Ok(rebuilt)
    }

    fn build_font_context(snapshot: font_registry::RegistrySnapshot) -> FontContext {
        let mut searcher = FontSearcher::new();
        #[cfg(target_arch = "wasm32")]
        searcher.include_system_fonts(false);
        let found = searcher.search();

        let mut book = FontBook::new();
        let mut fonts = Vec::new();
        for registered in snapshot.fonts.iter() {
            let bytes = Bytes::new(registered.bytes.clone());
            for face in registered.faces.iter() {
                if let Some(font) = Font::new(bytes.clone(), face.index) {
                    let mut info = font.info().clone();
                    info.family.clone_from(&face.family);
                    book.push(info);
                    fonts.push(ContextFontSlot::Registered(font));
                }
            }
        }
        for (index, slot) in found.fonts.into_iter().enumerate() {
            if let Some(info) = found.book.info(index) {
                book.push(info.clone());
                fonts.push(ContextFontSlot::Searcher(slot));
            }
        }

        let sans_family = select_family(
            &book,
            &[
                "noto sans",
                "dejavu sans",
                "liberation sans",
                "arial",
                "helvetica",
                "new computer modern sans",
                "latin modern sans",
            ],
            "sans",
            "New Computer Modern Sans",
        );
        let serif_family = select_family(
            &book,
            &[
                "new computer modern",
                "latin modern roman",
                "times new roman",
                "noto serif",
                "dejavu serif",
                "liberation serif",
                "georgia",
            ],
            "serif",
            "New Computer Modern",
        );
        let mono_family = select_family(
            &book,
            &[
                "new computer modern mono",
                "latin modern mono",
                "noto sans mono",
                "dejavu sans mono",
                "liberation mono",
                "courier new",
                "monaco",
            ],
            "mono",
            "New Computer Modern Mono",
        );
        FontContext {
            generation: snapshot.generation,
            book: LazyHash::new(book),
            fonts,
            sans_family,
            serif_family,
            mono_family,
        }
    }

    fn canonical_family_name<'a>(book: &'a FontBook, requested: &str) -> Option<&'a str> {
        let requested = requested.to_lowercase();
        book.select_family(&requested)
            .next()
            .and_then(|index| book.info(index))
            .map(|info| info.family.as_str())
    }

    fn select_family(
        book: &FontBook,
        preferred: &[&str],
        fallback_fragment: &str,
        final_fallback: &str,
    ) -> String {
        let fallback_fragment = fallback_fragment.to_ascii_lowercase();
        for candidate in preferred {
            if let Some(family) = canonical_family_name(book, candidate) {
                return family.to_string();
            }
        }

        let mut first_family: Option<String> = None;
        for (family, _) in book.families() {
            if first_family.is_none() {
                first_family = Some(family.to_string());
            }
            if family.to_ascii_lowercase().contains(&fallback_fragment) {
                return family.to_string();
            }
        }

        first_family.unwrap_or_else(|| final_fallback.to_string())
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum GenericFontFamily {
        SansSerif,
        Serif,
        Monospace,
    }

    fn inferred_generic_family_for_name(name: &str) -> GenericFontFamily {
        let lowered = name.to_ascii_lowercase();
        if lowered.contains("mono")
            || lowered.contains("courier")
            || lowered.contains("consolas")
            || lowered.contains("menlo")
        {
            GenericFontFamily::Monospace
        } else if lowered.contains("sans") {
            GenericFontFamily::SansSerif
        } else if lowered.contains("serif")
            || lowered.contains("times")
            || lowered.contains("georgia")
            || lowered.contains("cambria")
            || lowered.contains("garamond")
        {
            GenericFontFamily::Serif
        } else {
            GenericFontFamily::SansSerif
        }
    }

    fn fallback_family(font_ctx: &FontContext, generic: GenericFontFamily) -> &str {
        match generic {
            GenericFontFamily::SansSerif => &font_ctx.sans_family,
            GenericFontFamily::Serif => &font_ctx.serif_family,
            GenericFontFamily::Monospace => &font_ctx.mono_family,
        }
    }

    fn resolve_typst_font_family(font_ctx: &FontContext, family: &FontFamily) -> String {
        match family {
            FontFamily::Serif => font_ctx.serif_family.clone(),
            FontFamily::Monospace => font_ctx.mono_family.clone(),
            FontFamily::SansSerif | FontFamily::Cursive | FontFamily::Fantasy => {
                font_ctx.sans_family.clone()
            }
            FontFamily::Name(name) => canonical_family_name(&font_ctx.book, name)
                .map(str::to_string)
                .unwrap_or_else(|| {
                    fallback_family(font_ctx, inferred_generic_family_for_name(name)).to_string()
                }),
        }
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
        make_key_with_font_family(snippet, size_pt, color, rotation_deg, backend, "", 0)
    }

    fn make_key_with_font_family(
        snippet: &str,
        size_pt: f32,
        color: Color,
        rotation_deg: f32,
        backend: TypstBackendKind,
        font_family: &str,
        font_generation: u64,
    ) -> CacheKey {
        CacheKey {
            font_generation,
            snippet: snippet.to_string(),
            size_bits: size_pt.to_bits(),
            color: (color.r, color.g, color.b, color.a),
            rotation_bits: rotation_deg.to_bits(),
            backend,
            font_family: font_family.to_string(),
        }
    }

    fn synchronize_cache_generation(cache: &mut CacheState, generation: u64) -> bool {
        if generation < cache.font_generation {
            return false;
        }
        if generation > cache.font_generation {
            cache.entries.clear();
            cache.total_bytes = 0;
            cache.font_generation = generation;
        }
        true
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
        font_ctx: &FontContext,
        snippet: &str,
        size_pt: f32,
        color: Color,
        rotation_deg: f32,
        font_family: &str,
        operation: &str,
    ) -> Result<Page> {
        let source_text = build_document_source(snippet, size_pt, color, rotation_deg, font_family);
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
        render_raster_with_font_family(
            snippet,
            size_pt,
            color,
            rotation_deg,
            &FontFamily::SansSerif,
            operation,
        )
    }

    pub fn render_raster_with_font_family(
        snippet: &str,
        size_pt: f32,
        color: Color,
        rotation_deg: f32,
        font_family: &FontFamily,
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

        let font_ctx = font_context()?;
        let resolved_font_family = resolve_typst_font_family(&font_ctx, font_family);
        let key = make_key_with_font_family(
            snippet,
            size_pt,
            color,
            rotation_deg,
            TypstBackendKind::Raster,
            &resolved_font_family,
            font_ctx.generation,
        );

        {
            let mut cache = lock_cache()?;
            if synchronize_cache_generation(&mut cache, font_ctx.generation)
                && let Some(CachedValue::Raster {
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

        let page = compile_single_page(
            &font_ctx,
            snippet,
            size_pt,
            color,
            rotation_deg,
            &resolved_font_family,
            operation,
        )?;
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
        if synchronize_cache_generation(&mut cache, font_ctx.generation) {
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
        render_svg_with_font_family(
            snippet,
            size_pt,
            color,
            rotation_deg,
            &FontFamily::SansSerif,
            operation,
        )
    }

    pub fn render_svg_with_font_family(
        snippet: &str,
        size_pt: f32,
        color: Color,
        rotation_deg: f32,
        font_family: &FontFamily,
        operation: &str,
    ) -> Result<TypstSvgOutput> {
        if snippet.trim().is_empty() {
            return Ok(TypstSvgOutput {
                svg: String::new(),
                width: 0.0,
                height: 0.0,
            });
        }

        let font_ctx = font_context()?;
        let resolved_font_family = resolve_typst_font_family(&font_ctx, font_family);
        let key = make_key_with_font_family(
            snippet,
            size_pt,
            color,
            rotation_deg,
            TypstBackendKind::Svg,
            &resolved_font_family,
            font_ctx.generation,
        );
        {
            let mut cache = lock_cache()?;
            if synchronize_cache_generation(&mut cache, font_ctx.generation)
                && let Some(CachedValue::Svg { svg, width, height }) = cache.entries.get(&key)
            {
                return Ok(TypstSvgOutput {
                    svg: svg.clone(),
                    width: *width,
                    height: *height,
                });
            }
        }

        let page = compile_single_page(
            &font_ctx,
            snippet,
            size_pt,
            color,
            rotation_deg,
            &resolved_font_family,
            operation,
        )?;
        let raw_svg = typst_svg::svg(&page);
        let size = page.frame.size();
        let width = size.x.to_pt() as f32;
        let height = size.y.to_pt() as f32;

        let mut cache = lock_cache()?;
        if synchronize_cache_generation(&mut cache, font_ctx.generation) {
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
        measure_text_with_font_family(
            snippet,
            size_pt,
            color,
            rotation_deg,
            backend,
            &FontFamily::SansSerif,
            operation,
        )
    }

    pub fn measure_text_with_font_family(
        snippet: &str,
        size_pt: f32,
        color: Color,
        rotation_deg: f32,
        backend: TypstBackendKind,
        font_family: &FontFamily,
        operation: &str,
    ) -> Result<(f32, f32)> {
        match backend {
            TypstBackendKind::Raster => {
                let rendered = render_raster_with_font_family(
                    snippet,
                    size_pt,
                    color,
                    rotation_deg,
                    font_family,
                    operation,
                )?;
                Ok((rendered.width, rendered.height))
            }
            TypstBackendKind::Svg => {
                let rendered = render_svg_with_font_family(
                    snippet,
                    size_pt,
                    color,
                    rotation_deg,
                    font_family,
                    operation,
                )?;
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
        fn registered_font_in_initial_snapshot_feeds_typst_raster_and_svg() {
            let Some(bytes) = font_registry::renamed_test_font(b"PBef") else {
                return;
            };
            let font =
                font_registry::validate(bytes).expect("renamed deterministic font should validate");
            let font_ctx = build_font_context(font_registry::RegistrySnapshot {
                generation: 41,
                fonts: vec![font].into(),
            });
            let family = FontFamily::Name("PBef Sans".to_string());
            let resolved = resolve_typst_font_family(&font_ctx, &family);
            assert_eq!(resolved, "PBef Sans");

            let page = compile_single_page(
                &font_ctx,
                "Initial registry font",
                14.0,
                Color::BLACK,
                0.0,
                &resolved,
                "initial registry test",
            )
            .unwrap();
            assert!(!typst_svg::svg(&page).is_empty());
            assert!(!typst_render::render(&page, 1.0).data().is_empty());
        }

        #[test]
        fn registered_typst_face_uses_registry_canonical_family() {
            let Some(bytes) = font_registry::distinct_typographic_family_test_font() else {
                return;
            };
            let font = font_registry::validate(bytes).unwrap();
            let canonical = font.faces[0].family.clone();
            let typst_family = Font::new(Bytes::new(font.bytes.clone()), font.faces[0].index)
                .unwrap()
                .info()
                .family
                .clone();
            assert_ne!(typst_family, canonical);

            let font_ctx = build_font_context(font_registry::RegistrySnapshot {
                generation: 42,
                fonts: vec![font].into(),
            });
            let selected = font_ctx
                .book
                .select_family(&canonical.to_lowercase())
                .next()
                .expect("canonical registry family should select the registered face");

            assert_eq!(font_ctx.book.info(selected).unwrap().family, canonical);
            assert!(matches!(
                font_ctx.fonts[selected],
                ContextFontSlot::Registered(_)
            ));
            assert_eq!(
                resolve_typst_font_family(
                    &font_ctx,
                    &FontFamily::Name(font_ctx.book.info(selected).unwrap().family.clone())
                ),
                canonical
            );
        }

        #[test]
        fn late_registration_rebuilds_typst_context_and_advances_cache_generation() {
            let Some(bytes) = font_registry::renamed_test_font(b"PTyp") else {
                return;
            };
            let requested = FontFamily::Name("PTyp Sans".to_string());
            let before = font_context().unwrap();
            assert_ne!(resolve_typst_font_family(&before, &requested), "PTyp Sans");

            // Populate both backend caches at the old generation.
            render_svg_with_font_family(
                "Late font",
                15.0,
                Color::BLACK,
                0.0,
                &requested,
                "late SVG baseline",
            )
            .unwrap();
            render_raster_with_font_family(
                "Late font",
                15.0,
                Color::BLACK,
                0.0,
                &requested,
                "late raster baseline",
            )
            .unwrap();

            crate::render::register_font_bytes(bytes).unwrap();
            let after = font_context().unwrap();
            assert!(after.generation > before.generation);
            assert_eq!(resolve_typst_font_family(&after, &requested), "PTyp Sans");

            let svg = render_svg_with_font_family(
                "Late font",
                15.0,
                Color::BLACK,
                0.0,
                &requested,
                "late SVG registered",
            )
            .unwrap();
            let raster = render_raster_with_font_family(
                "Late font",
                15.0,
                Color::BLACK,
                0.0,
                &requested,
                "late raster registered",
            )
            .unwrap();
            assert!(!svg.svg.is_empty());
            assert!(raster.pixmap.data().iter().any(|alpha| *alpha != 0));

            let cache = lock_cache().unwrap();
            assert!(cache.font_generation >= after.generation);
            assert!(
                cache
                    .entries
                    .keys()
                    .all(|key| key.font_generation == cache.font_generation)
            );
        }

        #[test]
        fn cache_generation_clears_old_entries_and_rejects_regression() {
            let mut cache = CacheState::default();
            assert!(synchronize_cache_generation(&mut cache, 7));
            let key = make_key_with_font_family(
                "generation",
                12.0,
                Color::BLACK,
                0.0,
                TypstBackendKind::Svg,
                "P12",
                7,
            );
            insert_cached_value(
                &mut cache,
                key,
                CachedValue::Svg {
                    svg: "old".to_string(),
                    width: 1.0,
                    height: 1.0,
                },
            );
            assert!(synchronize_cache_generation(&mut cache, 8));
            assert!(cache.entries.is_empty());
            assert_eq!(cache.total_bytes, 0);
            assert!(!synchronize_cache_generation(&mut cache, 7));
            assert_eq!(cache.font_generation, 8);
        }

        fn swap_ascii_case(value: &str) -> String {
            value
                .chars()
                .map(|character| {
                    if character.is_ascii_lowercase() {
                        character.to_ascii_uppercase()
                    } else {
                        character.to_ascii_lowercase()
                    }
                })
                .collect()
        }

        #[test]
        fn named_family_resolution_is_case_insensitive_and_canonical() {
            let font_ctx = font_context().unwrap();
            let canonical = font_ctx
                .book
                .families()
                .next()
                .map(|(family, _)| family.to_string())
                .expect("Typst font search should find at least one family");
            let requested = swap_ascii_case(&canonical);

            let resolved = resolve_typst_font_family(&font_ctx, &FontFamily::Name(requested));

            assert_eq!(resolved, canonical);
        }

        #[test]
        fn supported_generic_families_resolve_to_selected_canonical_families() {
            let font_ctx = font_context().unwrap();

            assert_eq!(
                resolve_typst_font_family(&font_ctx, &FontFamily::Serif),
                font_ctx.serif_family
            );
            assert_eq!(
                resolve_typst_font_family(&font_ctx, &FontFamily::SansSerif),
                font_ctx.sans_family
            );
            assert_eq!(
                resolve_typst_font_family(&font_ctx, &FontFamily::Monospace),
                font_ctx.mono_family
            );
        }

        #[test]
        fn unsupported_typst_generic_families_use_sans_serif_fallback() {
            let font_ctx = font_context().unwrap();

            for family in [FontFamily::Cursive, FontFamily::Fantasy] {
                assert_eq!(
                    resolve_typst_font_family(&font_ctx, &family),
                    font_ctx.sans_family
                );
            }
        }

        #[test]
        fn typst_svg_render_applies_distinct_available_families() {
            let snippet = "#text(\"iiiiiiiiWWWW\")";
            let serif = render_svg_with_font_family(
                snippet,
                18.0,
                Color::BLACK,
                0.0,
                &FontFamily::Name("Libertinus Serif".to_string()),
                "serif test render",
            )
            .expect("embedded serif Typst render should succeed");
            let mono = render_svg_with_font_family(
                snippet,
                18.0,
                Color::BLACK,
                0.0,
                &FontFamily::Name("DejaVu Sans Mono".to_string()),
                "monospace test render",
            )
            .expect("embedded monospace Typst render should succeed");

            assert_ne!(serif.svg, mono.svg);
        }

        #[test]
        fn canonical_family_spelling_produces_the_same_cache_key_and_source() {
            let font_ctx = font_context().unwrap();
            let canonical = font_ctx.sans_family.clone();
            let differently_cased = swap_ascii_case(&canonical);
            let resolved_canonical =
                resolve_typst_font_family(&font_ctx, &FontFamily::Name(canonical.clone()));
            let resolved_differently_cased =
                resolve_typst_font_family(&font_ctx, &FontFamily::Name(differently_cased));

            let canonical_key = make_key_with_font_family(
                "#text(\"a\")",
                12.0,
                Color::BLACK,
                0.0,
                TypstBackendKind::Svg,
                &resolved_canonical,
                font_ctx.generation,
            );
            let differently_cased_key = make_key_with_font_family(
                "#text(\"a\")",
                12.0,
                Color::BLACK,
                0.0,
                TypstBackendKind::Svg,
                &resolved_differently_cased,
                font_ctx.generation,
            );
            let other_family_key = make_key_with_font_family(
                "#text(\"a\")",
                12.0,
                Color::BLACK,
                0.0,
                TypstBackendKind::Svg,
                "Definitely Different Family",
                font_ctx.generation,
            );
            let source = build_document_source(
                "#text(\"a\")",
                12.0,
                Color::BLACK,
                0.0,
                &resolved_differently_cased,
            );

            assert_eq!(resolved_canonical, canonical);
            assert_eq!(resolved_differently_cased, canonical);
            assert_eq!(canonical_key, differently_cased_key);
            assert_ne!(canonical_key, other_family_key);
            assert!(source.contains(&format!("font: \"{}\"", escape_typst_string(&canonical))));
        }

        #[test]
        fn document_source_escapes_named_family_for_typst_string() {
            let source = build_document_source(
                "Label",
                12.0,
                Color::BLACK,
                0.0,
                r#"Family "Quoted" \ Path"#,
            );

            assert!(source.contains(r#"font: "Family \"Quoted\" \\ Path""#));
        }

        #[test]
        fn missing_named_font_falls_back_by_family_hint() {
            let font_ctx = font_context().unwrap();

            let serif = resolve_typst_font_family(
                &font_ctx,
                &FontFamily::Name("Definitely Missing Serif".to_string()),
            );
            let sans = resolve_typst_font_family(
                &font_ctx,
                &FontFamily::Name("Definitely Missing Sans-Serif".to_string()),
            );
            let mono = resolve_typst_font_family(
                &font_ctx,
                &FontFamily::Name("Definitely Missing Mono".to_string()),
            );

            assert_eq!(serif, font_ctx.serif_family);
            assert_eq!(sans, font_ctx.sans_family);
            assert_eq!(mono, font_ctx.mono_family);
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
pub use imp::{
    measure_text, measure_text_with_font_family, render_raster, render_raster_with_font_family,
    render_svg, render_svg_with_font_family,
};
