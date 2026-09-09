use crate::{
    core::{PlottingError, Result, TextAlign},
    render::{
        Color, FontFamily, FontWeight,
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
    fn escaped_line(line: &str) -> String {
        line.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    if !text.contains('\n') {
        return format!("#text(\"{}\")", escaped_line(text));
    }

    text.split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .map(|line| format!("#text(\"{}\")", escaped_line(line)))
        .collect::<Vec<_>>()
        .join("\\\n")
}

pub(crate) fn with_explicit_line_breaks(snippet: &str) -> String {
    if snippet.trim().is_empty() || !snippet.contains('\n') {
        return snippet.to_string();
    }

    let mut explicit = String::with_capacity(snippet.len() + snippet.matches('\n').count());
    for segment in snippet.split_inclusive('\n') {
        let has_newline = segment.ends_with('\n');
        let line = segment.strip_suffix('\n').unwrap_or(segment);
        let line = line.strip_suffix('\r').unwrap_or(line);
        explicit.push_str(line);
        if has_newline {
            let trailing_backslashes = line.chars().rev().take_while(|ch| *ch == '\\').count();
            if trailing_backslashes % 2 == 0 {
                explicit.push('\\');
            }
            explicit.push('\n');
        }
    }
    explicit
}

pub(crate) fn with_font_weight(snippet: &str, weight: FontWeight) -> String {
    if snippet.trim().is_empty() {
        return snippet.to_string();
    }
    format!("#set text(weight: {})\n{snippet}", weight.numeric())
}

pub(crate) fn with_horizontal_alignment(snippet: &str, align: TextAlign) -> String {
    if snippet.trim().is_empty() || align == TextAlign::Left {
        return snippet.to_string();
    }
    let typst_align = match align {
        TextAlign::Left => unreachable!("left alignment returns before wrapping"),
        TextAlign::Center => "center",
        TextAlign::Right => "right",
    };
    format!("#align({typst_align})[{snippet}]")
}

#[cfg(test)]
mod weight_tests {
    use super::*;

    #[test]
    fn font_weight_wrapper_preserves_empty_text_fast_path() {
        assert_eq!(with_font_weight("", FontWeight::Bold), "");
        assert_eq!(with_font_weight(" \n\t", FontWeight::Bold), " \n\t");
    }

    #[test]
    fn horizontal_alignment_wrapper_preserves_empty_and_left_aligned_text() {
        assert_eq!(
            with_horizontal_alignment(" \n\t", TextAlign::Center),
            " \n\t"
        );
        assert_eq!(with_horizontal_alignment("value", TextAlign::Left), "value");
        assert_eq!(
            with_horizontal_alignment("wide\nshort", TextAlign::Right),
            "#align(right)[wide\nshort]"
        );
    }

    #[test]
    fn explicit_line_breaks_convert_authored_newlines_without_escaping_markup() {
        assert_eq!(with_explicit_line_breaks("wide\nshort"), "wide\\\nshort");
        assert_eq!(
            with_explicit_line_breaks("#emph[wide]\n$ x + 1 $"),
            "#emph[wide]\\\n$ x + 1 $"
        );
        assert_eq!(
            with_explicit_line_breaks("already\\\nexplicit"),
            "already\\\nexplicit"
        );
        assert_eq!(with_explicit_line_breaks(" \n\t"), " \n\t");
    }

    #[test]
    fn literal_text_snippet_uses_explicit_breaks_between_escaped_lines() {
        assert_eq!(
            literal_text_snippet("a\\\"\n\tb"),
            "#text(\"a\\\\\\\"\")\\\n#text(\"\\tb\")"
        );
    }
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

#[cfg(not(feature = "typst-math"))]
#[allow(clippy::too_many_arguments)]
pub fn render_raster_with_options(
    snippet: &str,
    size_pt: f32,
    color: Color,
    rotation_deg: f32,
    font_family: &FontFamily,
    _options: &crate::render::TextOptions,
    operation: &str,
) -> Result<TypstRasterOutput> {
    render_raster_with_font_family(
        snippet,
        size_pt,
        color,
        rotation_deg,
        font_family,
        operation,
    )
}

#[cfg(not(feature = "typst-math"))]
#[allow(clippy::too_many_arguments)]
pub fn render_svg_with_options(
    snippet: &str,
    size_pt: f32,
    color: Color,
    rotation_deg: f32,
    font_family: &FontFamily,
    _options: &crate::render::TextOptions,
    operation: &str,
) -> Result<TypstSvgOutput> {
    render_svg_with_font_family(
        snippet,
        size_pt,
        color,
        rotation_deg,
        font_family,
        operation,
    )
}

#[cfg(not(feature = "typst-math"))]
#[allow(clippy::too_many_arguments)]
pub fn measure_text_with_options(
    snippet: &str,
    size_pt: f32,
    color: Color,
    rotation_deg: f32,
    backend: TypstBackendKind,
    font_family: &FontFamily,
    _options: &crate::render::TextOptions,
    operation: &str,
) -> Result<(f32, f32)> {
    measure_text_with_font_family(
        snippet,
        size_pt,
        color,
        rotation_deg,
        backend,
        font_family,
        operation,
    )
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
        options: crate::render::TextOptions,
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
        cursive_family: String,
        fantasy_family: String,
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

        let generation = font_registry::generation()?;
        let contexts = FONTS.get_or_init(|| Mutex::new(None));
        let mut context = lock_cache_resource(contexts, "Typst font context")?;
        if let Some(existing) = context.as_ref()
            && existing.generation >= generation
        {
            return Ok(existing.clone());
        }

        let rebuilt = Arc::new(build_font_context(font_registry::snapshot()?)?);
        *context = Some(rebuilt.clone());
        Ok(rebuilt)
    }

    fn build_font_context(snapshot: font_registry::RegistrySnapshot) -> Result<FontContext> {
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

        // Use the concrete generic families chosen by the plain renderer. This
        // also keeps platform-specific names and registration precedence aligned.
        let system = crate::render::get_font_system().lock().map_err(|_| {
            PlottingError::RenderError(
                "Text rendering aborted because FontSystem lock is poisoned".into(),
            )
        })?;
        let selected = |family: FontFamily, candidates: &[&str], fragment: &str, fallback: &str| {
            let cosmic_family = family.to_cosmic_family();
            let shared = system.db().family_name(&cosmic_family);
            canonical_family_name(&book, shared)
                .map(str::to_string)
                .unwrap_or_else(|| select_family(&book, candidates, fragment, fallback))
        };
        let sans_family = selected(
            FontFamily::SansSerif,
            crate::render::font_policy::SANS,
            "sans",
            "New Computer Modern Sans",
        );
        let serif_family = selected(
            FontFamily::Serif,
            crate::render::font_policy::SERIF,
            "serif",
            &sans_family,
        );
        let mono_family = selected(
            FontFamily::Monospace,
            crate::render::font_policy::MONO,
            "mono",
            &sans_family,
        );
        let cursive_family = selected(
            FontFamily::Cursive,
            crate::render::font_policy::CURSIVE,
            "sans",
            &sans_family,
        );
        let fantasy_family = selected(
            FontFamily::Fantasy,
            crate::render::font_policy::FANTASY,
            "sans",
            &sans_family,
        );
        Ok(FontContext {
            generation: snapshot.generation,
            book: LazyHash::new(book),
            fonts,
            sans_family,
            serif_family,
            mono_family,
            cursive_family,
            fantasy_family,
        })
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
        match crate::render::font_policy::inferred_generic(name) {
            FontFamily::Serif => GenericFontFamily::Serif,
            FontFamily::Monospace => GenericFontFamily::Monospace,
            _ => GenericFontFamily::SansSerif,
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
            FontFamily::SansSerif => font_ctx.sans_family.clone(),
            FontFamily::Cursive => font_ctx.cursive_family.clone(),
            FontFamily::Fantasy => font_ctx.fantasy_family.clone(),
            FontFamily::Name(name) => canonical_family_name(&font_ctx.book, name)
                .map(str::to_string)
                .unwrap_or_else(|| {
                    fallback_family(font_ctx, inferred_generic_family_for_name(name)).to_string()
                }),
        }
    }

    fn resolve_typst_font_family_with_options(
        font_ctx: &FontContext,
        family: &FontFamily,
        options: &crate::render::TextOptions,
    ) -> String {
        if let FontFamily::Name(name) = family
            && canonical_family_name(&font_ctx.book, name).is_none()
            && let Some(fallback) = options
                .fallback_families()
                .iter()
                .map(String::as_str)
                .chain(options.language_fallbacks().iter().copied())
                .find_map(|family| canonical_family_name(&font_ctx.book, family))
        {
            return fallback.to_string();
        }
        resolve_typst_font_family(font_ctx, family)
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
            options: crate::render::TextOptions::default(),
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
        // Auto-sized, zero-margin labels must include the actual glyph bounds.
        // Font ascenders can exclude accents such as the ring in Å, and inline
        // math otherwise allows tall content to protrude into paragraph leading.
        if rotation_deg.abs() > f32::EPSILON {
            format!(
                "#set page(width: auto, height: auto, margin: 0pt, fill: none)\n#set text(font: \"{font_family}\", size: {size_pt}pt, fill: rgb({r}, {g}, {b}, {a}), top-edge: \"bounds\", bottom-edge: \"bounds\")\n#rotate({rotation_deg}deg, reflow: true)[{snippet}]",
                r = color.r,
                g = color.g,
                b = color.b,
                a = color.a,
            )
        } else {
            format!(
                "#set page(width: auto, height: auto, margin: 0pt, fill: none)\n#set text(font: \"{font_family}\", size: {size_pt}pt, fill: rgb({r}, {g}, {b}, {a}), top-edge: \"bounds\", bottom-edge: \"bounds\")\n{snippet}",
                r = color.r,
                g = color.g,
                b = color.b,
                a = color.a,
            )
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn build_document_source_with_options(
        font_ctx: &FontContext,
        snippet: &str,
        size_pt: f32,
        color: Color,
        rotation_deg: f32,
        font_family: &str,
        options: &crate::render::TextOptions,
    ) -> Result<String> {
        use std::fmt::Write;
        options.validate()?;
        let mut settings = String::new();
        if !options.fallback_families().is_empty() || !options.language_fallbacks().is_empty() {
            settings.push_str("#set text(font: (");
            for family in std::iter::once(font_family)
                .chain(options.fallback_families().iter().map(String::as_str))
                .chain(options.language_fallbacks().iter().copied())
            {
                let canonical = canonical_family_name(&font_ctx.book, family).unwrap_or(family);
                let _ = write!(settings, "\"{}\",", escape_typst_string(canonical));
            }
            settings.push_str("))\n");
        }
        if let Some((lang, script, region)) = options.language_components() {
            let _ = writeln!(settings, "#set text(lang: \"{lang}\")");
            if let Some(script) = script {
                let _ = writeln!(settings, "#set text(script: \"{script}\")");
            }
            // Typst accepts ISO alpha-2 regions; numeric UN regions are kept in
            // SVG language tags and the plain renderer's locale.
            if let Some(region) = region.filter(|region| region.len() == 2) {
                let _ = writeln!(
                    settings,
                    "#set text(region: \"{}\")",
                    region.to_ascii_uppercase()
                );
            }
        }
        if let Some(direction) = options.resolved_direction() {
            let dir = if direction == crate::render::TextDirection::RightToLeft {
                "rtl"
            } else {
                "ltr"
            };
            let _ = writeln!(settings, "#set text(dir: {dir})");
        }
        if let Some(family) = options.math_font_family() {
            let canonical = canonical_family_name(&font_ctx.book, family).ok_or_else(|| {
                PlottingError::TypstError(format!(
                    "Math font `{family}` is unavailable; install or register an OpenType math font"
                ))
            })?;
            let is_math = font_ctx
                .book
                .select_family(&canonical.to_lowercase())
                .any(|index| {
                    font_ctx
                        .fonts
                        .get(index)
                        .and_then(ContextFontSlot::get)
                        .is_some_and(|font| font.ttf().tables().math.is_some())
                });
            if !is_math {
                return Err(PlottingError::TypstError(format!(
                    "Font `{family}` has no OpenType MATH table"
                )));
            }
            let _ = writeln!(
                settings,
                "#show math.equation: set text(font: \"{}\")",
                escape_typst_string(canonical)
            );
        }
        if settings.is_empty() {
            return Ok(build_document_source(
                snippet,
                size_pt,
                color,
                rotation_deg,
                font_family,
            ));
        }
        settings.push_str(snippet);
        Ok(build_document_source(
            &settings,
            size_pt,
            color,
            rotation_deg,
            font_family,
        ))
    }

    fn validate_frame_glyphs(frame: &typst::layout::Frame) -> Result<()> {
        use typst::layout::FrameItem;
        for (_, item) in frame.items() {
            match item {
                FrameItem::Group(group) => validate_frame_glyphs(&group.frame)?,
                FrameItem::Text(text) => {
                    for glyph in text
                        .glyphs
                        .iter()
                        .filter(|glyph| glyph.id == 0 || text.font.info().is_last_resort())
                    {
                        if let Some(error) = crate::render::text_options::missing_glyph_error(
                            text.text.get(glyph.range()).unwrap_or(&text.text),
                        ) {
                            return Err(error);
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    #[cfg(test)]
    fn compile_single_page(
        font_ctx: &FontContext,
        snippet: &str,
        size_pt: f32,
        color: Color,
        rotation_deg: f32,
        font_family: &str,
        operation: &str,
    ) -> Result<Page> {
        compile_single_page_with_options(
            font_ctx,
            snippet,
            size_pt,
            color,
            rotation_deg,
            font_family,
            &crate::render::TextOptions::default(),
            operation,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn compile_single_page_with_options(
        font_ctx: &FontContext,
        snippet: &str,
        size_pt: f32,
        color: Color,
        rotation_deg: f32,
        font_family: &str,
        options: &crate::render::TextOptions,
        operation: &str,
    ) -> Result<Page> {
        let source_text = build_document_source_with_options(
            font_ctx,
            snippet,
            size_pt,
            color,
            rotation_deg,
            font_family,
            options,
        )?;
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

        if options.requires_all_glyphs() {
            for page in &document.pages {
                validate_frame_glyphs(&page.frame)?;
            }
        }

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
        render_raster_with_options(
            snippet,
            size_pt,
            color,
            rotation_deg,
            font_family,
            &crate::render::TextOptions::default(),
            operation,
        )
    }

    pub fn render_raster_with_options(
        snippet: &str,
        size_pt: f32,
        color: Color,
        rotation_deg: f32,
        font_family: &FontFamily,
        options: &crate::render::TextOptions,
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

        options.validate()?;
        let font_ctx = font_context()?;
        let resolved_font_family =
            resolve_typst_font_family_with_options(&font_ctx, font_family, options);
        let mut key = make_key_with_font_family(
            snippet,
            size_pt,
            color,
            rotation_deg,
            TypstBackendKind::Raster,
            &resolved_font_family,
            font_ctx.generation,
        );
        key.options = options.clone();

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

        let page = compile_single_page_with_options(
            &font_ctx,
            snippet,
            size_pt,
            color,
            rotation_deg,
            &resolved_font_family,
            options,
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
        render_svg_with_options(
            snippet,
            size_pt,
            color,
            rotation_deg,
            font_family,
            &crate::render::TextOptions::default(),
            operation,
        )
    }

    pub fn render_svg_with_options(
        snippet: &str,
        size_pt: f32,
        color: Color,
        rotation_deg: f32,
        font_family: &FontFamily,
        options: &crate::render::TextOptions,
        operation: &str,
    ) -> Result<TypstSvgOutput> {
        if snippet.trim().is_empty() {
            return Ok(TypstSvgOutput {
                svg: String::new(),
                width: 0.0,
                height: 0.0,
            });
        }

        options.validate()?;
        let font_ctx = font_context()?;
        let resolved_font_family =
            resolve_typst_font_family_with_options(&font_ctx, font_family, options);
        let mut key = make_key_with_font_family(
            snippet,
            size_pt,
            color,
            rotation_deg,
            TypstBackendKind::Svg,
            &resolved_font_family,
            font_ctx.generation,
        );
        key.options = options.clone();
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

        let page = compile_single_page_with_options(
            &font_ctx,
            snippet,
            size_pt,
            color,
            rotation_deg,
            &resolved_font_family,
            options,
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
        measure_text_with_options(
            snippet,
            size_pt,
            color,
            rotation_deg,
            backend,
            font_family,
            &crate::render::TextOptions::default(),
            operation,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn measure_text_with_options(
        snippet: &str,
        size_pt: f32,
        color: Color,
        rotation_deg: f32,
        backend: TypstBackendKind,
        font_family: &FontFamily,
        options: &crate::render::TextOptions,
        operation: &str,
    ) -> Result<(f32, f32)> {
        match backend {
            TypstBackendKind::Raster => {
                let rendered = render_raster_with_options(
                    snippet,
                    size_pt,
                    color,
                    rotation_deg,
                    font_family,
                    options,
                    operation,
                )?;
                Ok((rendered.width, rendered.height))
            }
            TypstBackendKind::Svg => {
                let rendered = render_svg_with_options(
                    snippet,
                    size_pt,
                    color,
                    rotation_deg,
                    font_family,
                    options,
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
        fn mode_switch_keeps_primary_font_and_text_width_for_styles() {
            let ctx = font_context().unwrap();
            let text = "Ångström Signal 123";
            let renderer = crate::render::TextRenderer::new();
            for family in [
                FontFamily::SansSerif,
                FontFamily::Serif,
                FontFamily::Monospace,
                FontFamily::from("Missing Serif"),
            ] {
                for weight in [
                    crate::render::FontWeight::Normal,
                    crate::render::FontWeight::Bold,
                ] {
                    let config =
                        crate::render::FontConfig::new(family.clone(), 20.0).weight(weight);
                    let (plain_width, _) = renderer.measure_text(text, &config).unwrap();
                    let resolved = resolve_typst_font_family(&ctx, &family);
                    let snippet = crate::render::typst_text::with_font_weight(text, weight);
                    let page = compile_single_page(
                        &ctx,
                        &snippet,
                        20.0,
                        Color::BLACK,
                        0.0,
                        &resolved,
                        "mode parity",
                    )
                    .unwrap();
                    let typst_width = page.frame.size().x.to_pt() as f32;
                    assert!(
                        (plain_width - typst_width).abs() < plain_width * 0.04,
                        "{family:?} {weight:?}: plain={plain_width}, Typst={typst_width}"
                    );
                    let system = crate::render::get_font_system().lock().unwrap();
                    let plain_family = crate::render::font_policy::resolve(
                        system.db(),
                        &family,
                        &crate::render::TextOptions::new(),
                    );
                    assert_eq!(
                        resolved, plain_family,
                        "{family:?} must not change fonts on a mode switch"
                    );
                }
            }
        }

        #[test]
        fn international_source_preserves_region_font_order_and_direction() {
            let ctx = font_context().unwrap();
            let options = crate::render::TextOptions::new()
                .font_fallbacks(["Example \"quoted\" font", "Second font"])
                .language("zh-TW")
                .direction(crate::render::TextDirection::RightToLeft);
            let source = build_document_source_with_options(
                &ctx,
                "Å",
                12.0,
                Color::BLACK,
                0.0,
                "Primary",
                &options,
            )
            .unwrap();
            assert!(
                source.contains("\"Primary\",\"Example \\\"quoted\\\" font\",\"Second font\","),
                "{source}"
            );
            assert!(source.contains("lang: \"zh\""));
            assert!(source.contains("region: \"TW\""));
            assert!(source.contains("dir: rtl"));
            assert!(source.contains("Noto Sans CJK TC"));
        }

        #[test]
        fn explicit_math_font_is_validated_and_part_of_cache_identity() {
            let ctx = font_context().unwrap();
            let options =
                crate::render::TextOptions::new().math_font("Ruviz nonexistent math font");
            let error = render_raster_with_options(
                "$q (Å^(-1))$",
                16.0,
                Color::BLACK,
                0.0,
                &FontFamily::SansSerif,
                &options,
                "test",
            )
            .unwrap_err();
            assert!(error.to_string().contains("unavailable"), "{error}");
            let Some(bytes) = font_registry::renamed_test_font(b"NMth") else {
                return;
            };
            let family = font_registry::validate(bytes.clone()).unwrap().faces[0]
                .family
                .clone();
            crate::render::register_font_bytes(bytes).unwrap();
            let nonmath = crate::render::TextOptions::new().math_font(family);
            let error = render_svg_with_options(
                "$q$",
                16.0,
                Color::BLACK,
                0.0,
                &FontFamily::SansSerif,
                &nonmath,
                "test",
            )
            .unwrap_err();
            assert!(error.to_string().contains("MATH table"), "{error}");
            let family = ctx
                .fonts
                .iter()
                .filter_map(ContextFontSlot::get)
                .find(|font| font.info().family == "New Computer Modern Math")
                .expect("bundled math font")
                .info()
                .family
                .clone();
            let valid = crate::render::TextOptions::new()
                .math_font(&family)
                .require_all_glyphs(true);
            let rendered = render_svg_with_options(
                "$q (Å^(-1))$",
                16.0,
                Color::BLACK,
                0.0,
                &FontFamily::SansSerif,
                &valid,
                "test",
            )
            .unwrap();
            assert!(rendered.width > 0.0 && rendered.height > 0.0);
            let mut key = make_key("$q$", 16.0, Color::BLACK, 0.0, TypstBackendKind::Svg);
            let default_key = key.clone();
            key.options = valid;
            assert_ne!(key, default_key);
        }

        #[test]
        fn missing_primary_uses_explicit_fallback_before_generic_substitution() {
            let Some(bytes) = font_registry::renamed_test_font(b"TFbk") else {
                return;
            };
            let font = font_registry::validate(bytes.clone()).unwrap();
            let family = font.faces[0].family.clone();
            crate::render::register_font_bytes(bytes).unwrap();
            let options = crate::render::TextOptions::new()
                .font_fallbacks([family.clone()])
                .require_all_glyphs(true);
            let actual = render_svg_with_options(
                "Ångström",
                16.0,
                Color::BLACK,
                0.0,
                &FontFamily::from("Unavailable primary"),
                &options,
                "test",
            )
            .unwrap();
            let expected = render_svg_with_font_family(
                "Ångström",
                16.0,
                Color::BLACK,
                0.0,
                &FontFamily::from(family),
                "test",
            )
            .unwrap();
            assert_eq!(actual.width, expected.width);
            assert_eq!(actual.height, expected.height);
            assert_eq!(actual.svg, expected.svg);
        }

        #[test]
        fn strict_typst_coverage_cannot_reuse_a_non_strict_cached_result() {
            let text = "Missing \u{10ffff}";
            let options = crate::render::TextOptions::new().require_all_glyphs(true);
            render_svg(text, 12.0, Color::BLACK, 0.0, "test").unwrap();
            let error = render_svg_with_options(
                text,
                12.0,
                Color::BLACK,
                0.0,
                &FontFamily::SansSerif,
                &options,
                "test",
            )
            .unwrap_err();
            assert!(error.to_string().contains("U+10FFFF"), "{error}");
        }

        #[test]
        fn angstrom_labels_keep_their_full_ink_inside_the_page() {
            use typst::layout::{Abs, Point};

            let font_ctx = font_context().unwrap();
            for snippet in [
                "$Å$",
                "$q (Å^(-1))$",
                "$\"Re\" chi(q) (Å^(-2))$",
                "$q (angstrom^(-1))$",
                "Å",
            ] {
                for rotation in [0.0, 90.0, -90.0] {
                    let page = compile_single_page(
                        &font_ctx,
                        snippet,
                        32.0,
                        Color::BLACK,
                        rotation,
                        "New Computer Modern",
                        "angstrom regression",
                    )
                    .unwrap();
                    let raster = typst_render::render(&page, 4.0);

                    // Render the same positioned glyphs on a larger canvas. Ink
                    // outside the original page would be lost in PNG and SVG.
                    let mut padded = page.clone();
                    let padding = Abs::pt(8.0);
                    padded.frame.translate(Point::new(padding, padding));
                    padded.frame.size_mut().x += 2.0 * padding;
                    padded.frame.size_mut().y += 2.0 * padding;
                    let reference = typst_render::render(&padded, 4.0);
                    let ink = |pixels: &[u8]| -> u64 {
                        pixels.chunks_exact(4).map(|pixel| pixel[3] as u64).sum()
                    };
                    let expected = ink(reference.data());
                    let actual = ink(raster.data());
                    // Allow 0.2% for rasterization rounding after translation.
                    assert!(
                        actual.abs_diff(expected) <= expected / 500,
                        "Typst clipped {snippet:?} at {rotation} degrees: \
                         rendered ink {actual}, unclipped ink {expected}"
                    );
                }
            }
        }

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
            })
            .unwrap();
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
            })
            .unwrap();
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
        fn all_generic_families_follow_the_plain_renderer() {
            let ctx = font_context().unwrap();
            let system = crate::render::get_font_system().lock().unwrap();
            for family in [
                FontFamily::SansSerif,
                FontFamily::Serif,
                FontFamily::Monospace,
                FontFamily::Cursive,
                FontFamily::Fantasy,
            ] {
                let cosmic_family = family.to_cosmic_family();
                let plain = system.db().family_name(&cosmic_family);
                assert_eq!(resolve_typst_font_family(&ctx, &family), plain);
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
    measure_text, measure_text_with_font_family, measure_text_with_options, render_raster,
    render_raster_with_font_family, render_raster_with_options, render_svg,
    render_svg_with_font_family, render_svg_with_options,
};
