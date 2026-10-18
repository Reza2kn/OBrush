//! Licensed static source text + independent bounded typed formatting. Catalog
//! readiness is not effective UI/font/translation-quality or reference parity.
use crate::app_preferences::Locale;
use crate::public_text_keys::PublicTextKey;
use crate::source_text_keys::SourceTextKey;
use std::{borrow::Cow, sync::OnceLock};
const MAX_CATALOG_BYTES: usize = 1024 * 1024;
const MAX_VALUE_BYTES: usize = 4096;
const MAX_FORMATTED_BYTES: usize = 8192;
const CATALOGS: [&str; 10] = [
    include_str!("../assets/sculptgl/catalogs/english.json"),
    include_str!("../assets/sculptgl/catalogs/japanese.json"),
    include_str!("../assets/sculptgl/catalogs/chinese.json"),
    include_str!("../assets/sculptgl/catalogs/korean.json"),
    include_str!("../assets/sculptgl/catalogs/russian.json"),
    include_str!("../assets/sculptgl/catalogs/turkish.json"),
    include_str!("../assets/sculptgl/catalogs/swedish.json"),
    include_str!("../assets/sculptgl/catalogs/french.json"),
    include_str!("../assets/sculptgl/catalogs/german.json"),
    include_str!("../assets/sculptgl/catalogs/italian.json"),
];
static PARSED: [OnceLock<Result<serde_json::Value, CatalogError>>; 10] =
    [const { OnceLock::new() }; 10];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogError {
    CatalogTooLarge,
    InvalidCatalog,
    MissingEnglishKey,
    WrongArguments,
    ArgumentTooLarge,
    InvalidTemplate,
}
#[derive(Clone, Copy, Debug)]
pub enum TextArguments<'a> {
    None,
    Faces(u32),
    Error(&'a str),
}
#[derive(Clone, Debug)]
pub struct TextResolution {
    pub text: Cow<'static, str>,
    pub requested: Locale,
    pub catalog_used: Locale,
    pub fallback: bool,
}
fn index(locale: Locale) -> usize {
    match locale {
        Locale::English => 0,
        Locale::Japanese => 1,
        Locale::Chinese => 2,
        Locale::Korean => 3,
        Locale::Russian => 4,
        Locale::Turkish => 5,
        Locale::Swedish => 6,
        Locale::French => 7,
        Locale::German => 8,
        Locale::Italian => 9,
    }
}
fn parse(data: &str) -> Result<serde_json::Value, CatalogError> {
    if data.len() > MAX_CATALOG_BYTES {
        return Err(CatalogError::CatalogTooLarge);
    }
    let value: serde_json::Value =
        serde_json::from_str(data).map_err(|_| CatalogError::InvalidCatalog)?;
    let object = value.as_object().ok_or(CatalogError::InvalidCatalog)?;
    if object.len() > 4096 {
        return Err(CatalogError::CatalogTooLarge);
    }
    for entry in object.values() {
        match entry {
            serde_json::Value::Null => {}
            serde_json::Value::String(s) if s.len() <= MAX_VALUE_BYTES => {}
            serde_json::Value::Object(t) if t.len() == 3 => {
                let formatter = t
                    .get("formatter")
                    .and_then(|v| v.as_str())
                    .ok_or(CatalogError::InvalidTemplate)?;
                if !matches!(formatter, "faces" | "error") {
                    return Err(CatalogError::InvalidTemplate);
                }
                for key in ["before", "after"] {
                    if t.get(key)
                        .and_then(|v| v.as_str())
                        .is_none_or(|s| s.len() > MAX_VALUE_BYTES)
                    {
                        return Err(CatalogError::InvalidTemplate);
                    }
                }
            }
            _ => return Err(CatalogError::InvalidCatalog),
        }
    }
    Ok(value)
}
fn catalog(locale: Locale) -> Result<&'static serde_json::Value, CatalogError> {
    PARSED[index(locale)]
        .get_or_init(|| parse(CATALOGS[index(locale)]))
        .as_ref()
        .map_err(|e| *e)
}
fn truthy(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => false,
        serde_json::Value::String(s) => !s.is_empty(),
        _ => true,
    }
}
fn render(
    value: &'static serde_json::Value,
    args: TextArguments<'_>,
) -> Result<Cow<'static, str>, CatalogError> {
    if let Some(s) = value.as_str() {
        return if matches!(args, TextArguments::None) {
            Ok(Cow::Borrowed(s))
        } else {
            Err(CatalogError::WrongArguments)
        };
    }
    let template = value.as_object().ok_or(CatalogError::InvalidTemplate)?;
    let formatter = template
        .get("formatter")
        .and_then(|v| v.as_str())
        .ok_or(CatalogError::InvalidTemplate)?;
    let before = template
        .get("before")
        .and_then(|v| v.as_str())
        .ok_or(CatalogError::InvalidTemplate)?;
    let after = template
        .get("after")
        .and_then(|v| v.as_str())
        .ok_or(CatalogError::InvalidTemplate)?;
    let argument = match (formatter, args) {
        ("faces", TextArguments::Faces(faces)) if faces <= 2147483648 => {
            Cow::Owned(faces.to_string())
        }
        ("error", TextArguments::Error(error)) if error.len() <= MAX_VALUE_BYTES => {
            Cow::Borrowed(error)
        }
        ("faces", TextArguments::Faces(_)) | ("error", TextArguments::Error(_)) => {
            return Err(CatalogError::ArgumentTooLarge);
        }
        _ => return Err(CatalogError::WrongArguments),
    };
    let total = before
        .len()
        .checked_add(argument.len())
        .and_then(|n| n.checked_add(after.len()))
        .ok_or(CatalogError::ArgumentTooLarge)?;
    if total > MAX_FORMATTED_BYTES {
        return Err(CatalogError::ArgumentTooLarge);
    }
    let mut text = String::new();
    text.try_reserve_exact(total)
        .map_err(|_| CatalogError::ArgumentTooLarge)?;
    text.push_str(before);
    text.push_str(&argument);
    text.push_str(after);
    Ok(Cow::Owned(text))
}
pub fn source_text(
    locale: Locale,
    key: SourceTextKey,
    args: TextArguments<'_>,
) -> Result<TextResolution, CatalogError> {
    let name = key.as_str();
    let selected = catalog(locale)?.get(name).filter(|v| truthy(v));
    let (fallback, value, used) = if let Some(value) = selected {
        (false, value, locale)
    } else {
        let value = catalog(Locale::English)?
            .get(name)
            .filter(|v| truthy(v))
            .ok_or(CatalogError::MissingEnglishKey)?;
        (locale != Locale::English, value, Locale::English)
    };
    Ok(TextResolution {
        text: render(value, args)?,
        requested: locale,
        catalog_used: used,
        fallback,
    })
}
/// Default English uses exact OBrush captions, even where a Source synonym differs.
pub fn public_label(locale: Locale, key: PublicTextKey) -> Result<TextResolution, CatalogError> {
    if locale == Locale::English {
        return Ok(TextResolution {
            text: Cow::Borrowed(key.legacy_english()),
            requested: locale,
            catalog_used: locale,
            fallback: false,
        });
    }
    source_text(locale, key.source(), TextArguments::None)
}
/// Read-only data readiness; required glyphs/UI wiring are separate capabilities.
pub fn catalog_metadata(locale: Locale) -> Result<serde_json::Value, CatalogError> {
    let c = catalog(locale)?;
    let keys = c.as_object().ok_or(CatalogError::InvalidCatalog)?;
    let fallback = SourceTextKey::ALL
        .iter()
        .filter(|k| keys.get(k.as_str()).is_none_or(|v| !truthy(v)))
        .count();
    Ok(
        serde_json::json!({"locale":locale.stable_id(),"static_bytes":CATALOGS[index(locale)].len(),"declared_unique_keys":keys.len(),
        "fallback_keys":fallback,"source_key_roster":SourceTextKey::ALL.len(),"mapped_label_count":PublicTextKey::ALL.len(),"effective_ui_ready":false,
        "scope":"Static data/formatter only. Glyph, full mapping, normal UI, notice and reference execution gates pending."}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_key_roster_has_all_168_unique_stable_ids() {
        let mut ids = SourceTextKey::ALL.map(SourceTextKey::as_str);
        ids.sort();
        assert!(ids.windows(2).all(|w| w[0] != w[1]));
        assert_eq!(ids.len(), 168);
    }
    #[test]
    fn english_default_application_draw_keeps_original_label() {
        assert_eq!(
            public_label(Locale::English, PublicTextKey::BrushDraw)
                .unwrap()
                .text,
            "Draw"
        );
        assert_eq!(
            source_text(
                Locale::English,
                SourceTextKey::SculptBrush,
                TextArguments::None
            )
            .unwrap()
            .text,
            "Brush"
        );
    }
    #[test]
    fn chinese_last_duplicate_null_and_missing_delete_use_english_fallback() {
        for key in [SourceTextKey::SceneDuplicate, SourceTextKey::SceneDelete] {
            let r = source_text(Locale::Chinese, key, TextArguments::None).unwrap();
            assert!(r.fallback);
            assert_eq!(r.catalog_used, Locale::English);
            assert_eq!(
                r.text,
                source_text(Locale::English, key, TextArguments::None)
                    .unwrap()
                    .text
            );
        }
    }
    #[test]
    fn japanese_null_fallback_and_real_native_camera_label_are_distinct() {
        assert!(
            source_text(
                Locale::Japanese,
                SourceTextKey::CameraFront,
                TextArguments::None
            )
            .unwrap()
            .fallback
        );
        let title = source_text(
            Locale::Japanese,
            SourceTextKey::CameraTitle,
            TextArguments::None,
        )
        .unwrap();
        assert!(!title.fallback);
        assert_eq!(title.text, "カメラ");
    }
    #[test]
    fn all_1680_data_lookups_have_typed_formatter_slots() {
        for locale in Locale::ALL {
            for key in SourceTextKey::ALL {
                let args = match key {
                    SourceTextKey::MultiresWarnBigMesh => TextArguments::Faces(1048576),
                    SourceTextKey::SketchfabUploadError => {
                        TextArguments::Error("literal & <error> 日本語")
                    }
                    _ => TextArguments::None,
                };
                assert!(!source_text(locale, key, args).unwrap().text.is_empty());
            }
        }
    }
    #[test]
    fn typed_arguments_remain_plain_text_and_wrong_type_is_rejected() {
        let r = source_text(
            Locale::English,
            SourceTextKey::SketchfabUploadError,
            TextArguments::Error("<script>&literal"),
        )
        .unwrap();
        assert!(r.text.contains("<script>&literal"));
        assert_eq!(
            source_text(
                Locale::English,
                SourceTextKey::MultiresWarnBigMesh,
                TextArguments::Error("1")
            )
            .unwrap_err(),
            CatalogError::WrongArguments
        );
    }
    #[test]
    fn formatter_argument_and_catalog_limits_are_enforced() {
        assert_eq!(
            source_text(
                Locale::English,
                SourceTextKey::MultiresWarnBigMesh,
                TextArguments::Faces(2147483649)
            )
            .unwrap_err(),
            CatalogError::ArgumentTooLarge
        );
        assert_eq!(
            source_text(
                Locale::English,
                SourceTextKey::SketchfabUploadError,
                TextArguments::Error(&"x".repeat(4097))
            )
            .unwrap_err(),
            CatalogError::ArgumentTooLarge
        );
        assert_eq!(
            parse(&"x".repeat(MAX_CATALOG_BYTES + 1)).unwrap_err(),
            CatalogError::CatalogTooLarge
        );
    }
    #[test]
    fn template_schema_rejects_numeric_unknown_and_missing_parts() {
        assert!(parse(r#"{"k":4}"#).is_err());
        assert!(parse(r#"{"k":{"formatter":"code","before":"","after":""}}"#).is_err());
        assert!(parse(r#"{"k":{"formatter":"faces","before":""}}"#).is_err());
    }
    #[test]
    fn public_key_ids_and_legacy_captions_remain_stable_without_font_enablement() {
        let mut ids = PublicTextKey::ALL.map(PublicTextKey::id);
        ids.sort();
        assert_eq!(ids.len(), 40);
        assert!(ids.windows(2).all(|w| w[0] != w[1]));
        for key in PublicTextKey::ALL {
            assert_eq!(
                public_label(Locale::English, key).unwrap().text,
                key.legacy_english()
            );
        }
    }
    #[test]
    fn data_metadata_registers_the_distinct_rosters_without_claiming_mapping_completion() {
        let data = catalog_metadata(Locale::Japanese).unwrap();
        assert_eq!(data["source_key_roster"], 168);
        assert_eq!(data["mapped_label_count"], 40);
        assert_eq!(data["effective_ui_ready"], false);
    }
    #[test]
    fn locale_data_summary_retains_exact_chinese_fallback_count_and_bytes() {
        let data = catalog_metadata(Locale::Chinese).unwrap();
        assert_eq!(data["declared_unique_keys"], 164);
        assert_eq!(data["fallback_keys"], 10);
        assert_eq!(data["static_bytes"], 6533);
    }
    #[test]
    fn catalog_availability_never_advertises_effective_font_or_ui_support() {
        for locale in Locale::ALL {
            assert_eq!(
                catalog_metadata(locale).unwrap()["effective_ui_ready"],
                false
            );
        }
    }
}
