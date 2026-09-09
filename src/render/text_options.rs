//! Shared font fallback and language settings for plot text.

use std::{borrow::Cow, sync::Arc};

use crate::core::{PlottingError, Result};

/// Dominant paragraph direction. Individual script runs still use Unicode bidi.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum TextDirection {
    /// Follow the configured language, or the text when no language is set.
    #[default]
    Auto,
    /// Left-to-right paragraph layout.
    LeftToRight,
    /// Right-to-left paragraph layout.
    RightToLeft,
}

/// International typography settings, shared cheaply between labels.
///
/// The primary family remains [`FontConfig::family`](super::FontConfig::family).
/// Fallback fonts are tried in order before language and system fallbacks.
/// Fonts must be installed or supplied with [`super::register_font_bytes`].
/// Mathematical expressions have a separate OpenType math font.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct TextOptions(Option<Arc<Options>>);

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
struct Options {
    fallbacks: Option<Arc<[String]>>,
    language: Option<String>,
    direction: Option<TextDirection>,
    math_font: Option<String>,
    require_all_glyphs: Option<bool>,
}

impl TextOptions {
    /// Create settings that inherit renderer defaults, without allocating.
    pub fn new() -> Self {
        Self::default()
    }

    fn edit(&mut self) -> &mut Options {
        Arc::make_mut(self.0.get_or_insert_with(|| Arc::new(Options::default())))
    }

    /// Set an ordered list of fallback family names (at most 32).
    pub fn font_fallbacks<I, S>(mut self, families: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.edit().fallbacks = Some(families.into_iter().map(Into::into).collect());
        self
    }

    /// Set an ISO 639 language with optional script/region, such as `ja`, `zh-TW`, or `ar`.
    ///
    /// This selects regional CJK fallback families and the paragraph direction.
    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.edit().language = Some(language.into().replace('_', "-").to_ascii_lowercase());
        self
    }

    /// Set the dominant paragraph direction explicitly.
    pub fn direction(mut self, direction: TextDirection) -> Self {
        self.edit().direction = Some(direction);
        self
    }

    /// Select an OpenType math font for Typst equations.
    ///
    /// For example, `Fira Math` provides sans-serif equations. The font must be
    /// installed or registered; unavailable or non-math fonts produce an error.
    pub fn math_font(mut self, family: impl Into<String>) -> Self {
        self.edit().math_font = Some(family.into());
        self
    }

    /// Fail on missing glyphs instead of exporting replacement boxes.
    /// Disabled by default for compatibility; useful for publication exports.
    pub fn require_all_glyphs(mut self, required: bool) -> Self {
        self.edit().require_all_glyphs = Some(required);
        self
    }

    /// Whether a missing glyph causes an export error.
    pub fn requires_all_glyphs(&self) -> bool {
        self.0
            .as_ref()
            .and_then(|o| o.require_all_glyphs)
            .unwrap_or(false)
    }

    /// Configured fallback family names in priority order.
    pub fn fallback_families(&self) -> &[String] {
        self.0
            .as_ref()
            .and_then(|o| o.fallbacks.as_deref())
            .unwrap_or(&[])
    }

    /// Explicitly selected language, if any.
    pub fn text_language(&self) -> Option<&str> {
        self.0.as_ref().and_then(|o| o.language.as_deref())
    }

    /// Explicit paragraph direction, or automatic direction.
    pub fn text_direction(&self) -> TextDirection {
        self.0
            .as_ref()
            .and_then(|o| o.direction)
            .unwrap_or_default()
    }

    /// Explicitly selected math family, if any.
    pub fn math_font_family(&self) -> Option<&str> {
        self.0.as_ref().and_then(|o| o.math_font.as_deref())
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.fallback_families().len() > 32 {
            return Err(PlottingError::InvalidInput(
                "at most 32 fallback font families are supported".into(),
            ));
        }
        if self.fallback_families().iter().any(|f| f.trim().is_empty())
            || self.math_font_family().is_some_and(|f| f.trim().is_empty())
        {
            return Err(PlottingError::InvalidInput(
                "font family names must not be empty".into(),
            ));
        }
        if let Some(language) = self.text_language()
            && (language.len() > 63
                || !language.split('-').next().is_some_and(|primary| {
                    (2..=3).contains(&primary.len())
                        && primary.bytes().all(|b| b.is_ascii_alphabetic())
                })
                || language.split('-').any(|s| {
                    s.is_empty() || s.len() > 8 || !s.bytes().all(|b| b.is_ascii_alphanumeric())
                }))
        {
            return Err(PlottingError::InvalidInput(
                "language must start with a two- or three-letter ISO language code, such as ja, zh-TW, or ar".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn overlay(&self, child: Option<&Self>) -> Self {
        let Some(child) = child.and_then(|o| o.0.as_ref()) else {
            return self.clone();
        };
        let Some(parent) = self.0.as_ref() else {
            return Self(Some(child.clone()));
        };
        Self(Some(Arc::new(Options {
            fallbacks: child.fallbacks.clone().or_else(|| parent.fallbacks.clone()),
            language: child.language.clone().or_else(|| parent.language.clone()),
            direction: child.direction.or(parent.direction),
            math_font: child.math_font.clone().or_else(|| parent.math_font.clone()),
            require_all_glyphs: child.require_all_glyphs.or(parent.require_all_glyphs),
        })))
    }

    pub(crate) fn needs_font_context(&self) -> bool {
        !self.fallback_families().is_empty() || self.text_language().is_some()
    }

    pub(crate) fn language_components(&self) -> Option<(&str, Option<&str>, Option<&str>)> {
        let mut parts = self.text_language()?.split('-');
        let primary = parts.next()?;
        let mut next = parts.next();
        let script = if next
            .is_some_and(|part| part.len() == 4 && part.bytes().all(|b| b.is_ascii_alphabetic()))
        {
            let script = next;
            next = parts.next();
            script
        } else {
            None
        };
        let region = next.filter(|part| {
            part.len() == 2 && part.bytes().all(|b| b.is_ascii_alphabetic())
                || part.len() == 3 && part.bytes().all(|b| b.is_ascii_digit())
        });
        Some((primary, script, region))
    }

    pub(crate) fn same_font_context(&self, other: &Self) -> bool {
        self.fallback_families() == other.fallback_families()
            && self.text_language() == other.text_language()
    }

    pub(crate) fn resolved_direction(&self) -> Option<TextDirection> {
        match self.text_direction() {
            TextDirection::Auto => self.language_components().map(|(language, script, _)| {
                if let Some(script) = script {
                    return if matches!(
                        script,
                        "arab" | "hebr" | "syrc" | "thaa" | "nkoo" | "adlm" | "rohg"
                    ) {
                        TextDirection::RightToLeft
                    } else {
                        TextDirection::LeftToRight
                    };
                }
                match language {
                    "ar" | "he" | "fa" | "ur" | "ps" | "sd" | "yi" | "dv" | "ug" | "syr"
                    | "nqo" | "ckb" => TextDirection::RightToLeft,
                    _ => TextDirection::LeftToRight,
                }
            }),
            explicit => Some(explicit),
        }
    }

    /// Add bidi embeddings only when the requested direction differs from the
    /// paragraph's natural direction. Avoiding redundant controls preserves the
    /// shaper's fast path for ordinary CJK, Latin, and Arabic labels.
    pub(crate) fn directed_text<'a>(&self, text: &'a str) -> Cow<'a, str> {
        use unicode_bidi::{BidiClass, Direction, bidi_class, get_base_direction};
        let Some(direction) = self.resolved_direction() else {
            return Cow::Borrowed(text);
        };
        let rtl = direction == TextDirection::RightToLeft;
        let is_separator = |character| bidi_class(character) == BidiClass::B;
        let needs_override = |paragraph: &str| {
            !paragraph.trim().is_empty() && (get_base_direction(paragraph) == Direction::Rtl) != rtl
        };
        if text
            .split_inclusive(is_separator)
            .all(|paragraph| !needs_override(paragraph))
        {
            return Cow::Borrowed(text);
        }
        let embedding = if rtl { '\u{202b}' } else { '\u{202a}' };
        let mut directed = String::with_capacity(text.len() + 6);
        for paragraph in text.split_inclusive(is_separator) {
            if !needs_override(paragraph) {
                directed.push_str(paragraph);
                continue;
            }
            let separator_start = paragraph
                .char_indices()
                .next_back()
                .filter(|(_, character)| is_separator(*character))
                .map(|(index, _)| index)
                .unwrap_or(paragraph.len());
            directed.push(embedding);
            directed.push_str(&paragraph[..separator_start]);
            directed.push('\u{202c}');
            directed.push_str(&paragraph[separator_start..]);
        }
        Cow::Owned(directed)
    }

    pub(crate) fn language_fallbacks(&self) -> &'static [&'static str] {
        let Some((language, script, region)) = self.language_components() else {
            return &[];
        };
        match language {
            "ja" => &[
                "Noto Sans CJK JP",
                "Noto Sans JP",
                "Hiragino Sans",
                "Yu Gothic",
                "Meiryo",
            ],
            "ko" => &[
                "Noto Sans CJK KR",
                "Noto Sans KR",
                "Apple SD Gothic Neo",
                "Malgun Gothic",
            ],
            "zh" if matches!(region, Some("hk" | "mo")) => &[
                "Noto Sans CJK HK",
                "Noto Sans HK",
                "PingFang HK",
                "Noto Sans CJK TC",
                "Microsoft JhengHei",
            ],
            "zh" if script == Some("hant") || region == Some("tw") => &[
                "Noto Sans CJK TC",
                "Noto Sans TC",
                "PingFang TC",
                "Microsoft JhengHei",
            ],
            "zh" => &[
                "Noto Sans CJK SC",
                "Noto Sans SC",
                "PingFang SC",
                "Microsoft YaHei",
            ],
            "ar" | "fa" | "ur" => &["Noto Sans Arabic", "Noto Naskh Arabic"],
            "he" | "yi" => &["Noto Sans Hebrew"],
            _ => &[],
        }
    }
}

/// Ignore formatting controls; these intentionally have no visible glyph.
pub(crate) fn missing_glyph_error(cluster: &str) -> Option<PlottingError> {
    let character = cluster.chars().find(|c| !c.is_whitespace() && !c.is_control()
        && !matches!(*c, '\u{00ad}' | '\u{061c}' | '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{206f}' | '\u{fe00}'..='\u{fe0f}' | '\u{feff}' | '\u{e0100}'..='\u{e01ef}'))?;
    Some(PlottingError::RenderError(format!(
        "No font supplies a glyph for U+{:04X} ({character}); install or register a font and add it to font_fallbacks",
        character as u32
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_only_adds_controls_to_paragraphs_that_need_an_override() {
        for (language, text) in [
            ("ja", "散乱強度 / Signal"),
            ("ar", "العربية / Signal"),
            ("en", "\u{2067}עברית\u{2069} Signal"),
        ] {
            assert!(
                matches!(
                    TextOptions::new().language(language).directed_text(text),
                    Cow::Borrowed(_)
                ),
                "{language}: {text}"
            );
        }
        let options = TextOptions::new().language("ar");
        assert_eq!(
            options.directed_text("Signal 123"),
            "\u{202b}Signal 123\u{202c}"
        );
        assert_eq!(
            options.directed_text("العربية\nSignal\u{2029}עברית"),
            "العربية\n\u{202b}Signal\u{202c}\u{2029}עברית"
        );
        assert_eq!(
            TextOptions::new().language("ar-Latn").resolved_direction(),
            Some(TextDirection::LeftToRight)
        );
        assert_eq!(
            TextOptions::new().language("az-Arab").resolved_direction(),
            Some(TextDirection::RightToLeft)
        );
        assert_eq!(
            TextOptions::new()
                .language("zh-Hant-TW-u-nu-hanidec")
                .language_fallbacks()[0],
            "Noto Sans CJK TC"
        );
    }

    #[test]
    fn regional_fonts_and_direction_follow_the_language() {
        for (language, family) in [
            ("ja", "Noto Sans CJK JP"),
            ("ko", "Noto Sans CJK KR"),
            ("zh-CN", "Noto Sans CJK SC"),
            ("zh-TW", "Noto Sans CJK TC"),
            ("zh-HK", "Noto Sans CJK HK"),
        ] {
            let options = TextOptions::new().language(language);
            assert_eq!(options.language_fallbacks()[0], family);
        }
        let arabic = TextOptions::new().language("ar");
        assert_eq!(
            arabic.resolved_direction(),
            Some(TextDirection::RightToLeft)
        );
        assert_eq!(
            arabic
                .direction(TextDirection::LeftToRight)
                .resolved_direction(),
            Some(TextDirection::LeftToRight)
        );
        assert!(matches!(
            TextOptions::new().directed_text("Å"),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn annotation_overrides_keep_unspecified_parent_settings() {
        let parent = TextOptions::new()
            .font_fallbacks(["Alpha", "Beta"])
            .language("ja")
            .math_font("Fira Math")
            .require_all_glyphs(true);
        let child = TextOptions::new()
            .language("zh_TW")
            .direction(TextDirection::Auto);
        let merged = parent.overlay(Some(&child));
        assert_eq!(merged.fallback_families(), ["Alpha", "Beta"]);
        assert_eq!(merged.text_language(), Some("zh-tw"));
        assert_eq!(merged.math_font_family(), Some("Fira Math"));
        assert!(merged.requires_all_glyphs());
        assert!(
            parent
                .overlay(Some(
                    &TextOptions::new().font_fallbacks(Vec::<String>::new())
                ))
                .fallback_families()
                .is_empty()
        );
    }

    #[test]
    fn malformed_options_fail_and_formatting_controls_are_ignored() {
        for options in [
            TextOptions::new().language("ja\""),
            TextOptions::new().language(""),
            TextOptions::new().font_fallbacks([""]),
            TextOptions::new().font_fallbacks(vec!["Font"; 33]),
            TextOptions::new().math_font(" "),
        ] {
            assert!(options.validate().is_err());
        }
        assert!(missing_glyph_error("\u{200d}\u{202b}\u{fe0f}").is_none());
        assert!(
            missing_glyph_error("\u{10ffff}")
                .unwrap()
                .to_string()
                .contains("U+10FFFF")
        );
    }
}
