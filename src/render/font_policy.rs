//! Shared primary-font policy for shaping, Typst, and exported text.

use std::borrow::Cow;

use cosmic_text::fontdb::Database;

use super::{FontFamily, TextOptions};

pub(crate) const SANS: &[&str] = &[
    "Arial",
    "Helvetica",
    "Liberation Sans",
    "Noto Sans",
    "DejaVu Sans",
    "Open Sans",
    "New Computer Modern Sans",
    "Latin Modern Sans",
];
pub(crate) const SERIF: &[&str] = &[
    "Times New Roman",
    "Times",
    "Liberation Serif",
    "Noto Serif",
    "DejaVu Serif",
    "New Computer Modern",
    "Libertinus Serif",
    "Latin Modern Roman",
];
pub(crate) const MONO: &[&str] = &[
    "Consolas",
    "Menlo",
    "Liberation Mono",
    "Noto Sans Mono",
    "DejaVu Sans Mono",
    "Courier New",
    "New Computer Modern Mono",
    "Latin Modern Mono",
];
pub(crate) const CURSIVE: &[&str] = &["Comic Sans MS", "Apple Chancery", "URW Chancery L"];
pub(crate) const FANTASY: &[&str] = &["Impact", "Papyrus", "Copperplate"];

pub(crate) fn inferred_generic(name: &str) -> FontFamily {
    let lowered = name.to_ascii_lowercase();
    if ["mono", "courier", "consolas", "menlo"]
        .iter()
        .any(|part| lowered.contains(part))
    {
        FontFamily::Monospace
    } else if lowered.contains("sans") {
        FontFamily::SansSerif
    } else if ["serif", "times", "georgia", "cambria", "garamond"]
        .iter()
        .any(|part| lowered.contains(part))
    {
        FontFamily::Serif
    } else {
        FontFamily::SansSerif
    }
}

fn canonical_family<'a>(database: &'a Database, family: &str) -> Option<&'a str> {
    database
        .faces()
        .flat_map(|face| &face.families)
        .find(|(name, _)| name.eq_ignore_ascii_case(family))
        .map(|(name, _)| name.as_str())
}

fn select(database: &Database, candidates: &[&str], fallback: &str) -> String {
    candidates
        .iter()
        .find_map(|family| canonical_family(database, family))
        .unwrap_or(fallback)
        .to_string()
}

/// Choose available families once per database generation. Both text engines
/// use these lists, including the same embedded faces when Typst is enabled.
pub(crate) fn configure_database(database: &mut Database) {
    let first_family = database
        .faces()
        .flat_map(|face| &face.families)
        .map(|(name, _)| name.as_str())
        .min()
        .unwrap_or("sans-serif")
        .to_string();
    let sans = select(database, SANS, &first_family);
    let serif = select(database, SERIF, &sans);
    let mono = select(database, MONO, &sans);
    let cursive = select(database, CURSIVE, &sans);
    let fantasy = select(database, FANTASY, &sans);
    database.set_sans_serif_family(sans);
    database.set_serif_family(serif);
    database.set_monospace_family(mono);
    database.set_cursive_family(cursive);
    database.set_fantasy_family(fantasy);
}

/// Resolve primary font availability without inspecting individual characters.
/// Script shaping and ordered glyph fallback remain the engine's responsibility.
pub(crate) fn resolve<'a>(
    database: &Database,
    family: &'a FontFamily,
    options: &TextOptions,
) -> Cow<'a, str> {
    if let FontFamily::Name(name) = family {
        if let Some(canonical) = canonical_family(database, name) {
            return if canonical == name {
                Cow::Borrowed(name)
            } else {
                Cow::Owned(canonical.to_string())
            };
        }
        if let Some(fallback) = options
            .fallback_families()
            .iter()
            .map(String::as_str)
            .chain(options.language_fallbacks().iter().copied())
            .find_map(|candidate| canonical_family(database, candidate))
        {
            return Cow::Owned(fallback.to_string());
        }
        return Cow::Owned(
            database
                .family_name(&inferred_generic(name).to_cosmic_family())
                .to_string(),
        );
    }
    Cow::Owned(database.family_name(&family.to_cosmic_family()).to_string())
}

/// Include the already bundled Typst faces in plain mode as well. Sources share
/// their immutable bytes; enabling this does not add another font bundle.
#[cfg(feature = "typst-math")]
pub(crate) fn load_embedded_fonts(database: &mut Database) {
    use std::sync::{Arc, OnceLock};
    static SOURCES: OnceLock<Vec<cosmic_text::fontdb::Source>> = OnceLock::new();
    let sources = SOURCES.get_or_init(|| {
        let mut searcher = typst_kit::fonts::FontSearcher::new();
        searcher.include_system_fonts(false);
        searcher
            .search()
            .fonts
            .into_iter()
            .filter_map(|slot| slot.get())
            .map(|font| cosmic_text::fontdb::Source::Binary(Arc::new(font.data().clone())))
            .collect()
    });
    for source in sources {
        database.load_font_source(source.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmic_text::fontdb::Family;

    #[test]
    fn missing_primary_preserves_fallback_order_and_generic_style() {
        let mut db = Database::new();
        db.load_font_data(include_bytes!("../dejavu-sans.ttf").to_vec());
        let Some(noto) = crate::render::font_registry::renamed_test_font(b"Noto") else {
            return;
        };
        db.load_font_data(noto);
        configure_database(&mut db);
        let missing = FontFamily::from("Missing serif");
        assert_eq!(
            resolve(
                &db,
                &missing,
                &TextOptions::new().font_fallbacks(["Noto Sans", "DejaVu Sans"])
            ),
            "Noto Sans"
        );
        assert_eq!(
            resolve(&db, &FontFamily::from("noto sans"), &TextOptions::new()),
            "Noto Sans"
        );
        assert_eq!(
            inferred_generic("Missing Sans-Serif"),
            FontFamily::SansSerif
        );
        assert_eq!(inferred_generic("Missing Mono"), FontFamily::Monospace);
        assert_eq!(inferred_generic("Missing Times"), FontFamily::Serif);
        assert_eq!(db.family_name(&Family::SansSerif), "Noto Sans");
    }
}
