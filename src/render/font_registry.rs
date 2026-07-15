use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use cosmic_text::fontdb;

use crate::core::{PlottingError, Result};

#[derive(Clone, Debug)]
pub(crate) struct RegisteredFace {
    pub(crate) index: u32,
    pub(crate) family: String,
    pub(crate) post_script_name: String,
    style: fontdb::Style,
    weight: fontdb::Weight,
    stretch: fontdb::Stretch,
}

#[derive(Clone, Debug)]
pub(crate) struct SharedFontBytes(Arc<Vec<u8>>);

impl SharedFontBytes {
    pub(crate) fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl AsRef<[u8]> for SharedFontBytes {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RegisteredFont {
    pub(crate) bytes: SharedFontBytes,
    pub(crate) faces: Arc<[RegisteredFace]>,
}

impl RegisteredFont {
    pub(crate) fn fontdb_source(&self) -> fontdb::Source {
        fontdb::Source::Binary(Arc::new(self.bytes.clone()))
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct RegistrySnapshot {
    pub(crate) generation: u64,
    pub(crate) fonts: Arc<[RegisteredFont]>,
}

#[derive(Debug, Default)]
struct FontRegistry {
    generation: u64,
    fonts: Vec<RegisteredFont>,
}

#[derive(Debug)]
pub(crate) enum Registration {
    Duplicate,
    Added(RegistrySnapshot),
}

static REGISTRY: OnceLock<Mutex<FontRegistry>> = OnceLock::new();

fn registry() -> &'static Mutex<FontRegistry> {
    REGISTRY.get_or_init(|| Mutex::new(FontRegistry::default()))
}

fn lock_registry() -> Result<MutexGuard<'static, FontRegistry>> {
    registry().lock().map_err(|_| {
        PlottingError::RenderError(
            "Font registration aborted because the font registry lock is poisoned".to_string(),
        )
    })
}

pub(crate) fn validate(bytes: Vec<u8>) -> Result<RegisteredFont> {
    let bytes = SharedFontBytes(Arc::new(bytes));
    let face_count = ttf_parser::fonts_in_collection(bytes.as_slice()).unwrap_or(1);
    if face_count == 0 {
        return Err(invalid_font("font collection contains no faces"));
    }

    for index in 0..face_count {
        ttf_parser::Face::parse(bytes.as_slice(), index)
            .map_err(|err| invalid_font(format!("face {index} could not be parsed: {err}")))?;
    }

    let mut database = fontdb::Database::new();
    let ids = database.load_font_source(fontdb::Source::Binary(Arc::new(bytes.clone())));
    if ids.len() != face_count as usize {
        return Err(invalid_font(
            "one or more faces have no usable canonical metadata",
        ));
    }

    let faces = ids
        .iter()
        .map(|id| {
            let face = database.face(*id).ok_or_else(|| {
                invalid_font("validated face metadata disappeared from the font database")
            })?;
            let family = face
                .families
                .first()
                .map(|(family, _)| family.clone())
                .ok_or_else(|| invalid_font(format!("face {} has no family name", face.index)))?;
            Ok(RegisteredFace {
                index: face.index,
                family,
                post_script_name: face.post_script_name.clone(),
                style: face.style,
                weight: face.weight,
                stretch: face.stretch,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(RegisteredFont {
        bytes,
        faces: faces.into(),
    })
}

fn invalid_font(reason: impl std::fmt::Display) -> PlottingError {
    PlottingError::RenderError(format!("Invalid font data: {reason}"))
}

pub(crate) fn register(font: RegisteredFont) -> Result<Registration> {
    let mut registry = lock_registry()?;
    register_in(&mut registry, font)
}

fn register_in(registry: &mut FontRegistry, font: RegisteredFont) -> Result<Registration> {
    if registry
        .fonts
        .iter()
        .any(|existing| existing.bytes.as_slice() == font.bytes.as_slice())
    {
        return Ok(Registration::Duplicate);
    }

    registry.generation = registry.generation.checked_add(1).ok_or_else(|| {
        PlottingError::RenderError("Font registry generation overflowed".to_string())
    })?;
    registry.fonts.push(font);
    Ok(Registration::Added(RegistrySnapshot {
        generation: registry.generation,
        fonts: registry.fonts.clone().into(),
    }))
}

pub(crate) fn snapshot() -> Result<RegistrySnapshot> {
    let registry = lock_registry()?;
    Ok(RegistrySnapshot {
        generation: registry.generation,
        fonts: registry.fonts.clone().into(),
    })
}

/// Load registered faces into an existing database while giving exact
/// family/style/weight/stretch ties precedence over preloaded system faces.
pub(crate) fn load_with_registered_precedence(
    database: &mut fontdb::Database,
    snapshot: &RegistrySnapshot,
) {
    let superseded = database
        .faces()
        .filter(|candidate| {
            snapshot.fonts.iter().any(|font| {
                font.faces.iter().any(|registered| {
                    candidate.style == registered.style
                        && candidate.weight == registered.weight
                        && candidate.stretch == registered.stretch
                        && candidate
                            .families
                            .iter()
                            .any(|(family, _)| family == &registered.family)
                })
            })
        })
        .map(|face| face.id)
        .collect::<Vec<_>>();
    for id in superseded {
        database.remove_face(id);
    }

    for font in snapshot.fonts.iter() {
        database.load_font_source(font.fontdb_source());
    }
}

#[cfg(test)]
fn deterministic_test_font() -> Option<Vec<u8>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("crates/ruviz-web/assets/NotoSans-Regular.ttf");
    std::fs::read(path).ok()
}

#[cfg(test)]
pub(crate) fn renamed_test_font(prefix: &[u8; 4]) -> Option<Vec<u8>> {
    fn replace_all(bytes: &mut [u8], from: &[u8], to: &[u8]) {
        debug_assert_eq!(from.len(), to.len());
        let mut offset = 0;
        while let Some(index) = bytes[offset..]
            .windows(from.len())
            .position(|window| window == from)
        {
            let start = offset + index;
            bytes[start..start + to.len()].copy_from_slice(to);
            offset = start + to.len();
        }
    }

    let mut bytes = deterministic_test_font()?;
    replace_all(&mut bytes, b"Noto", prefix);
    let utf16_from: Vec<_> = "Noto".encode_utf16().flat_map(u16::to_be_bytes).collect();
    let utf16_to: Vec<_> = std::str::from_utf8(prefix)
        .ok()?
        .encode_utf16()
        .flat_map(u16::to_be_bytes)
        .collect();
    replace_all(&mut bytes, &utf16_from, &utf16_to);
    Some(bytes)
}

#[cfg(all(test, feature = "typst-math"))]
pub(crate) fn distinct_typographic_family_test_font() -> Option<Vec<u8>> {
    fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
        Some(u16::from_be_bytes(
            bytes.get(offset..offset + 2)?.try_into().ok()?,
        ))
    }

    fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
        Some(u32::from_be_bytes(
            bytes.get(offset..offset + 4)?.try_into().ok()?,
        ))
    }

    let mut bytes = deterministic_test_font()?;
    let table_count = usize::from(read_u16(&bytes, 4)?);
    let name_offset = (0..table_count).find_map(|index| {
        let record = 12 + index.checked_mul(16)?;
        (bytes.get(record..record + 4)? == b"name")
            .then(|| usize::try_from(read_u32(&bytes, record + 8)?).ok())
            .flatten()
    })?;
    let name_count = usize::from(read_u16(&bytes, name_offset + 2)?);

    for index in 0..name_count {
        let record = name_offset + 6 + index.checked_mul(12)?;
        let platform_id = read_u16(&bytes, record)?;
        let name_id = read_u16(&bytes, record + 6)?;
        if platform_id == 3 && name_id == ttf_parser::name_id::FULL_NAME {
            bytes
                .get_mut(record + 6..record + 8)?
                .copy_from_slice(&ttf_parser::name_id::TYPOGRAPHIC_FAMILY.to_be_bytes());
            return Some(bytes);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_extracts_canonical_metadata_and_rejects_invalid_bytes() {
        let Some(bytes) = deterministic_test_font() else {
            return;
        };
        let font = validate(bytes).unwrap();
        assert_eq!(font.faces.len(), 1);
        assert_eq!(font.faces[0].family, "Noto Sans");
        assert!(!font.faces[0].post_script_name.is_empty());

        let err = validate(b"not a font".to_vec()).unwrap_err();
        assert!(err.to_string().contains("Invalid font data"));
    }

    #[test]
    fn duplicate_and_invalid_fonts_do_not_advance_local_generation() {
        let Some(bytes) = renamed_test_font(b"PDup") else {
            return;
        };
        let mut registry = FontRegistry::default();
        let font = validate(bytes).unwrap();

        assert!(matches!(
            register_in(&mut registry, font.clone()).unwrap(),
            Registration::Added(_)
        ));
        assert_eq!(registry.generation, 1);
        assert_eq!(registry.fonts.len(), 1);

        assert!(matches!(
            register_in(&mut registry, font).unwrap(),
            Registration::Duplicate
        ));
        assert_eq!(registry.generation, 1);
        assert_eq!(registry.fonts.len(), 1);

        assert!(validate(b"invalid font".to_vec()).is_err());
        assert_eq!(registry.generation, 1);
        assert_eq!(registry.fonts.len(), 1);
    }

    #[test]
    fn registered_faces_replace_exact_preloaded_ties() {
        let Some(bytes) = renamed_test_font(b"PPrx") else {
            return;
        };
        let font = validate(bytes).unwrap();
        let registered = font.faces[0].clone();
        let mut database = fontdb::Database::new();
        let preloaded = database.load_font_source(font.fontdb_source());
        let preloaded_id = preloaded[0];
        let snapshot = RegistrySnapshot {
            generation: 1,
            fonts: vec![font].into(),
        };

        load_with_registered_precedence(&mut database, &snapshot);
        let selected = database.query(&fontdb::Query {
            families: &[fontdb::Family::Name(&registered.family)],
            weight: registered.weight,
            stretch: registered.stretch,
            style: registered.style,
        });

        assert!(database.face(preloaded_id).is_none());
        assert!(selected.is_some());
        assert_eq!(database.len(), 1);
    }
}
