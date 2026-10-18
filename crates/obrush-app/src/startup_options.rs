//! Bounded shared startup capture. Raw grammar is source-compatible in the
//! declared finite domain. This module never applies settings or reads files.
use crate::app_preferences::{AppPreferences, Locale, PreferenceProfile};
use crate::shortcut_profile::ShortcutResolver;
use std::collections::BTreeMap;

pub const MAX_QUERY_BYTES: usize = 16 * 1024;
pub const MAX_QUERY_PAIRS: usize = 128;
pub const MAX_MODEL_URL_BYTES: usize = 4096;

#[derive(Clone, Debug, PartialEq)]
pub struct SourceStartupSettings {
    pub scale_center: bool,
    pub grid: bool,
    pub outline: bool,
    pub outline_color: [f64; 4],
    pub mirror_line: bool,
    pub darken_unselected: bool,
    pub projection: u8,
    pub camera_mode: u8,
    pub pivot: bool,
    pub fov: f64,
    pub flat_shading: bool,
    pub wireframe: bool,
    pub curvature: f64,
    pub exposure: Option<f64>,
    pub environment: u32,
    pub matcap: u32,
    /// Original Enums.Shader code (0..12). Internal roles are captured as data,
    /// not claimed supported by the current OBrush renderer/normal UI.
    pub shader: u8,
    pub filmic: bool,
    pub model_url: Option<String>,
}

impl Default for SourceStartupSettings {
    fn default() -> Self {
        Self {
            scale_center: false,
            grid: true,
            outline: false,
            outline_color: [0.3, 0.0, 0.0, 1.0],
            mirror_line: false,
            darken_unselected: true,
            projection: 0,
            camera_mode: 0,
            pivot: true,
            fov: 45.0,
            flat_shading: false,
            wireframe: false,
            curvature: 0.0,
            exposure: None,
            environment: 0,
            matcap: 3,
            shader: 5,
            filmic: false,
            model_url: None,
        }
    }
}

impl SourceStartupSettings {
    /// Consumers call this only after audited bundled asset counts are known.
    pub fn clamp_presets(&mut self, environments: u32, matcaps: u32) -> Result<(), &'static str> {
        if environments == 0 || matcaps == 0 {
            return Err("Preset sets must be nonempty");
        }
        self.environment = self.environment.min(environments - 1);
        self.matcap = self.matcap.min(matcaps - 1);
        Ok(())
    }

    pub fn resolved_exposure(&self, preset_exposure: f64) -> Result<f64, &'static str> {
        let v = self.exposure.unwrap_or(preset_exposure);
        if v.is_finite() && (0.0..=5.0).contains(&v) {
            Ok(v)
        } else {
            Err("Exposure requires finite supported value")
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StartupOptions {
    pub preferences: AppPreferences,
    pub source: Option<SourceStartupSettings>,
    pub shortcuts: ShortcutResolver,
}

impl StartupOptions {
    /// Both entry points pass the raw source grammar here. The explicit
    /// OBrush extension only selects a profile; it does not decode source values.
    pub fn capture_auto(search: &str, navigator: Option<&str>) -> Result<Self, &'static str> {
        if search.len() > MAX_QUERY_BYTES {
            return Err("Startup query exceeds byte limit");
        }
        let mut profile = PreferenceProfile::OBrush;
        for (count, row) in search
            .strip_prefix('?')
            .unwrap_or(search)
            .split('&')
            .enumerate()
        {
            if count >= MAX_QUERY_PAIRS {
                return Err("Startup query exceeds pair limit");
            }
            let mut pair = row.split('=');
            if pair
                .next()
                .is_some_and(|key| key.eq_ignore_ascii_case("obrushprofile"))
            {
                profile = match pair.next() {
                    Some("sculptgl") => PreferenceProfile::SculptGl,
                    Some("obrush") => PreferenceProfile::OBrush,
                    _ => return Err("Explicit startup profile must be obrush or sculptgl"),
                };
            }
        }
        Self::capture(profile, search, navigator)
    }

    /// Native explicit arguments share the same bounded capture implementation.
    /// Unknown options are rejected, not silently treated as file paths.
    pub fn capture_native(args: &[String]) -> Result<Self, &'static str> {
        let mut profile = None;
        let mut query = "";
        for argument in args {
            if let Some(value) = argument.strip_prefix("--obrush-profile=") {
                profile = Some(match value {
                    "obrush" => PreferenceProfile::OBrush,
                    "sculptgl" => PreferenceProfile::SculptGl,
                    _ => return Err("Explicit startup profile must be obrush or sculptgl"),
                });
            } else if let Some(value) = argument.strip_prefix("--startup-query=") {
                query = value;
            } else {
                return Err("Unknown OBrush startup argument");
            }
        }
        match profile {
            Some(profile) => Self::capture(profile, query, None),
            None => Self::capture_auto(query, None),
        }
    }
    /// The legacy profile intentionally ignores query initializers. Explicit
    /// source profile is required; native/browser transport adapters are future.
    pub fn capture(
        profile: PreferenceProfile,
        search: &str,
        navigator: Option<&str>,
    ) -> Result<Self, &'static str> {
        if search.len() > MAX_QUERY_BYTES {
            return Err("Startup query exceeds byte limit");
        }
        if profile == PreferenceProfile::OBrush {
            return Ok(Self {
                preferences: AppPreferences::default(),
                source: None,
                shortcuts: ShortcutResolver::new(profile),
            });
        }
        let mut params = BTreeMap::new();
        for (count, pair) in search
            .strip_prefix('?')
            .unwrap_or(search)
            .split('&')
            .enumerate()
        {
            if count >= MAX_QUERY_PAIRS {
                return Err("Startup query exceeds pair limit");
            }
            // Source split('=',2) truncates at the next equals, not split_once.
            let mut pieces = pair.split('=');
            if let (Some(key), Some(value)) = (pieces.next(), pieces.next()) {
                params.insert(key.to_lowercase(), value);
            }
        }
        let get = |key: &str| params.get(key).copied();
        let bool_value =
            |key: &str, default: bool| get(key).map_or(default, |v| v != "false" && v != "0");
        let model_url = get("modelurl").filter(|url| !url.is_empty());
        if model_url.is_some_and(|url| url.len() > MAX_MODEL_URL_BYTES) {
            return Err("Model URL exceeds byte limit");
        }
        let settings = SourceStartupSettings {
            scale_center: bool_value("scalecenter", false),
            grid: bool_value("grid", true),
            outline: bool_value("outline", false),
            outline_color: parse_color(get("outlinecolor"), [0.3, 0.0, 0.0, 1.0])?,
            mirror_line: bool_value("mirrorline", false),
            darken_unselected: bool_value("darkenunselected", true),
            projection: enum_value(get("projection"), &["PERSPECTIVE", "ORTHOGRAPHIC"], 0),
            camera_mode: enum_value(get("cameramode"), &["ORBIT", "SPHERICAL", "PLANE"], 0),
            pivot: bool_value("pivot", true),
            fov: number_value(get("fov"), 10.0, 90.0)?.unwrap_or(45.0),
            flat_shading: bool_value("flatshading", false),
            wireframe: bool_value("wireframe", false),
            curvature: number_value(get("curvature"), 0.0, 5.0)?.unwrap_or(0.0),
            exposure: number_value(get("exposure"), 0.0, 5.0)?,
            environment: preset_index(get("environment"))?.unwrap_or(0),
            matcap: preset_index(get("matcap"))?.unwrap_or(3),
            shader: enum_value(
                get("shader"),
                &[
                    "PBR",
                    "FLAT",
                    "NORMAL",
                    "WIREFRAME",
                    "UV",
                    "MATCAP",
                    "SELECTION",
                    "BACKGROUND",
                    "MERGE",
                    "FXAA",
                    "CONTOUR",
                    "PAINTUV",
                    "BLUR",
                ],
                5,
            ),
            filmic: bool_value("filmic", false),
            model_url: model_url.map(str::to_owned),
        };
        let mut shortcuts = ShortcutResolver::new(profile);
        if let Some(overrides) = get("shortcuts") {
            shortcuts.source_overrides(overrides)?;
        }
        Ok(Self {
            preferences: AppPreferences {
                locale: Locale::source_initial(navigator, get("language")),
                shortcut_profile: profile,
                ..AppPreferences::default()
            },
            source: Some(settings),
            shortcuts,
        })
    }
}

fn enum_value(value: Option<&str>, choices: &[&str], default: u8) -> u8 {
    value
        .and_then(|v| choices.iter().position(|x| *x == v.to_uppercase()))
        .map_or(default, |v| v as u8)
}

fn js_space(c: char) -> bool {
    matches!(c, '\u{0009}'..='\u{000D}' | '\u{0020}' | '\u{00A0}' | '\u{1680}' |
        '\u{2000}'..='\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}' |
        '\u{205F}' | '\u{3000}' | '\u{FEFF}')
}

/// Independent finite JS parseFloat prefix schedule, not str::parse on all text.
pub(crate) fn float_prefix(input: &str) -> Result<Option<f64>, &'static str> {
    let s = input.trim_start_matches(js_space);
    if s.starts_with("Infinity") || s.starts_with("+Infinity") || s.starts_with("-Infinity") {
        return Err("Nonfinite startup value is outside bounded domain");
    }
    let b = s.as_bytes();
    let mut i = usize::from(matches!(b.first(), Some(b'+' | b'-')));
    let start = i;
    while b.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    let before = i - start;
    if b.get(i) == Some(&b'.') {
        i += 1;
    }
    let fraction_start = i;
    while b.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    if before == 0 && i == fraction_start {
        return Ok(None);
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        let exponent = i;
        i += 1;
        if matches!(b.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let digits = i;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i == digits {
            i = exponent;
        }
    }
    let v = s[..i]
        .parse::<f64>()
        .map_err(|_| "Invalid startup numeric prefix")?;
    if v.is_finite() {
        Ok(Some(v))
    } else {
        Err("Nonfinite startup numeric prefix")
    }
}

pub(crate) fn integer_prefix(input: &str) -> Result<Option<i64>, &'static str> {
    let s = input.trim_start_matches(js_space);
    let b = s.as_bytes();
    let mut i = usize::from(matches!(b.first(), Some(b'+' | b'-')));
    let start = i;
    while b.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    if i == start {
        return Ok(None);
    }
    s[..i]
        .parse::<i64>()
        .map(Some)
        .map_err(|_| "Startup integer exceeds bounded domain")
}

fn number_value(value: Option<&str>, min: f64, max: f64) -> Result<Option<f64>, &'static str> {
    value.map(float_prefix).transpose().map(|v| {
        v.flatten().map(|x| {
            let clamped = x.clamp(min, max);
            if min == 0.0 && clamped == 0.0 {
                0.0
            } else {
                clamped
            }
        })
    })
}

fn preset_index(value: Option<&str>) -> Result<Option<u32>, &'static str> {
    let Some(n) = value.map(integer_prefix).transpose()?.flatten() else {
        return Ok(None);
    };
    if n > i64::from(u32::MAX) {
        return Err("Preset index exceeds bounded domain");
    }
    Ok(Some(n.max(0) as u32))
}

fn parse_color(value: Option<&str>, default: [f64; 4]) -> Result<[f64; 4], &'static str> {
    let Some(s) = value.filter(|v| !v.is_empty()) else {
        return Ok(default);
    };
    let fields = s.split(',').collect::<Vec<_>>();
    if fields.len() < 3 {
        return Ok(default);
    }
    let mut out = default;
    for k in 0..3 {
        out[k] = integer_prefix(if fields[k].is_empty() { "0" } else { fields[k] })?
            .ok_or("Outline color requires numeric RGB prefix")? as f64
            / 255.0;
    }
    if let Some(alpha) = fields.get(3) {
        out[3] = float_prefix(alpha)?.ok_or("Outline alpha requires numeric prefix")?;
    }
    if out.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v)) {
        Ok(out)
    } else {
        Err("Outline color exceeds finite normalized domain")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source(query: &str) -> StartupOptions {
        StartupOptions::capture(PreferenceProfile::SculptGl, query, None).unwrap()
    }

    #[test]
    fn legacy_profile_preserves_defaults_without_applying_query() {
        let s = StartupOptions::capture(
            PreferenceProfile::OBrush,
            "?fov=80&language=japanese",
            Some("ja"),
        )
        .unwrap();
        assert_eq!(s.preferences, AppPreferences::default());
        assert!(s.source.is_none());
        assert_eq!(s.shortcuts.profile(), PreferenceProfile::OBrush);
    }
    #[test]
    fn source_defaults_include_absent_exposure_and_matcap_three() {
        let s = source("");
        assert_eq!(s.source, Some(SourceStartupSettings::default()));
        assert_eq!(s.preferences.locale, Locale::English);
        assert_eq!(
            s.preferences.theme,
            crate::app_preferences::ThemePreference::LegacyDark
        );
    }
    #[test]
    fn all_twenty_one_fields_and_shortcut_initializer_are_captured() {
        let s = source(
            "language=russian&scalecenter=1&grid=0&outline=1&outlinecolor=128,64,0,.25&mirrorline=1&darkenunselected=0&projection=orthographic&cameramode=spherical&pivot=0&fov=30&flatshading=1&wireframe=1&curvature=2&exposure=1.25&environment=4&matcap=8&shader=normal&filmic=1&modelurl=owned.obj&shortcuts=brush:A",
        );
        assert_eq!(s.preferences.locale, Locale::Russian);
        assert_eq!(
            s.source,
            Some(SourceStartupSettings {
                scale_center: true,
                grid: false,
                outline: true,
                outline_color: [128.0 / 255.0, 64.0 / 255.0, 0.0, 0.25],
                mirror_line: true,
                darken_unselected: false,
                projection: 1,
                camera_mode: 1,
                pivot: false,
                fov: 30.0,
                flat_shading: true,
                wireframe: true,
                curvature: 2.0,
                exposure: Some(1.25),
                environment: 4,
                matcap: 8,
                shader: 2,
                filmic: true,
                model_url: Some("owned.obj".into())
            })
        );
        assert_eq!(
            s.shortcuts.resolve(
                crate::shortcut_profile::KeyDelivery {
                    code: 65,
                    pressed: true,
                    repeat: false,
                    modifiers: Default::default()
                },
                Default::default()
            ),
            vec![crate::shortcut_profile::Command::SelectBrush(
                obrush_core::BrushKind::Draw
            )]
        );
    }
    #[test]
    fn keys_casefold_duplicates_last_and_equals_truncates() {
        let s = source("?FoV=60&fov=75&modelurl=a.obj=discarded&grid")
            .source
            .unwrap();
        assert_eq!(s.fov, 75.0);
        assert_eq!(s.model_url.as_deref(), Some("a.obj"));
        assert!(s.grid);
    }
    #[test]
    fn raw_percent_plus_and_empty_boolean_are_not_decoded() {
        let s = source("language=japan%65se&modelurl=a+b%20.obj&grid=&outline=False&pivot=0");
        assert_eq!(s.preferences.locale, Locale::English);
        let s = s.source.unwrap();
        assert_eq!(s.model_url.as_deref(), Some("a+b%20.obj"));
        assert!(s.grid);
        assert!(s.outline);
        assert!(!s.pivot);
    }
    #[test]
    fn finite_numeric_prefixes_clamp_and_missing_parse_falls_back() {
        let s = source("fov=95junk&curvature=-2x&exposure=.5more&environment=12e2&matcap=bad")
            .source
            .unwrap();
        assert_eq!(
            (s.fov, s.curvature, s.exposure, s.environment, s.matcap),
            (90.0, 0.0, Some(0.5), 12, 3)
        );
        assert_eq!(float_prefix("1e-"), Ok(Some(1.0)));
        assert_eq!(float_prefix("+.25x"), Ok(Some(0.25)));
        assert_eq!(float_prefix("0x10"), Ok(Some(0.0)));
        assert_eq!(float_prefix("bad"), Ok(None));
    }
    #[test]
    fn js_whitespace_and_negative_zero_prefix_are_retained() {
        let n = float_prefix("\u{FEFF}\t-0tail").unwrap().unwrap();
        assert_eq!(n.to_bits(), (-0.0_f64).to_bits());
        assert_eq!(float_prefix("\u{0085}1"), Ok(None));
    }
    #[test]
    fn bool_exact_false_zero_and_case_mismatch() {
        let s = source("grid=false&outline=0&filmic=FALSE&mirrorline=false%20&darkenunselected=")
            .source
            .unwrap();
        assert!(!s.grid && !s.outline && s.filmic && s.mirror_line && s.darken_unselected);
    }
    #[test]
    fn enum_fallback_and_all_original_shader_codes_are_data() {
        let s = source("projection=orthographic&cameramode=plane&shader=paintuv")
            .source
            .unwrap();
        assert_eq!((s.projection, s.camera_mode, s.shader), (1, 2, 11));
        let s = source("projection=missing&cameramode=missing&shader=missing")
            .source
            .unwrap();
        assert_eq!((s.projection, s.camera_mode, s.shader), (0, 0, 5));
    }
    #[test]
    fn valid_outline_color_prefix_and_alpha_are_typed() {
        assert_eq!(
            source("outlinecolor=255,0,128,.5")
                .source
                .unwrap()
                .outline_color,
            [1.0, 0.0, 128.0 / 255.0, 0.5]
        );
        assert_eq!(
            source("outlinecolor=1,2").source.unwrap().outline_color,
            [0.3, 0.0, 0.0, 1.0]
        );
    }
    #[test]
    fn invalid_color_nonfinite_and_large_numbers_reject_without_result() {
        for q in [
            "outlinecolor=NaN,0,0",
            "outlinecolor=256,0,0",
            "fov=Infinity",
            "curvature=1e999",
            "environment=4294967296",
        ] {
            assert!(
                StartupOptions::capture(PreferenceProfile::SculptGl, q, None).is_err(),
                "{q}"
            );
        }
    }
    #[test]
    fn source_zero_minimum_clamp_yields_positive_zero() {
        let s = source("curvature=-0&exposure=-0").source.unwrap();
        assert_eq!(s.curvature.to_bits(), 0_u64);
        assert_eq!(s.exposure.unwrap().to_bits(), 0_u64);
    }
    #[test]
    fn asset_clamp_and_exposure_fallback_do_not_invent_defaults() {
        let mut s = source("environment=99&matcap=99").source.unwrap();
        s.clamp_presets(5, 9).unwrap();
        assert_eq!((s.environment, s.matcap), (4, 8));
        assert_eq!(s.resolved_exposure(2.5), Ok(2.5));
        s.exposure = Some(0.25);
        assert_eq!(s.resolved_exposure(2.5), Ok(0.25));
        let before = s.clone();
        assert!(s.clamp_presets(0, 9).is_err());
        assert_eq!(s, before);
    }
    #[test]
    fn query_count_byte_and_model_url_limits_reject() {
        assert!(
            StartupOptions::capture(
                PreferenceProfile::SculptGl,
                &"a".repeat(MAX_QUERY_BYTES + 1),
                None
            )
            .is_err()
        );
        assert!(
            StartupOptions::capture(
                PreferenceProfile::SculptGl,
                &vec!["x=0"; MAX_QUERY_PAIRS + 1].join("&"),
                None
            )
            .is_err()
        );
        assert!(
            StartupOptions::capture(
                PreferenceProfile::SculptGl,
                &format!("modelurl={}", "a".repeat(MAX_MODEL_URL_BYTES + 1)),
                None
            )
            .is_err()
        );
    }
    #[test]
    fn locale_resolution_and_capture_are_owned_immutable_values() {
        let mut query = String::from("language=italian&fov=60");
        let captured =
            StartupOptions::capture(PreferenceProfile::SculptGl, &query, Some("ja")).unwrap();
        query.clear();
        assert_eq!(captured.preferences.locale, Locale::Italian);
        assert_eq!(captured.source.unwrap().fov, 60.0);
    }
}

#[cfg(test)]
mod boot_adapter_tests {
    use super::*;
    #[test]
    fn no_profile_keeps_legacy_even_with_reference_query_keys() {
        let options =
            StartupOptions::capture_auto("?fov=67&language=japanese", Some("ja-JP")).unwrap();
        assert_eq!(options.preferences, AppPreferences::default());
        assert!(options.source.is_none());
    }
    #[test]
    fn explicit_native_and_web_share_raw_source_capture() {
        let raw =
            "?obrushprofile=sculptgl&fov=45.125&shortcuts=BRUSH:A&modelurl=literal%20name.obj";
        let native = StartupOptions::capture_native(&[
            "--obrush-profile=sculptgl".into(),
            format!("--startup-query={raw}"),
        ])
        .unwrap();
        let web = StartupOptions::capture_auto(raw, None).unwrap();
        assert_eq!(native, web);
        assert_eq!(
            web.source.as_ref().unwrap().fov.to_bits(),
            45.125f64.to_bits()
        );
        assert_eq!(
            web.source.as_ref().unwrap().model_url.as_deref(),
            Some("literal%20name.obj")
        );
    }
    #[test]
    fn duplicate_profile_last_wins_and_invalid_profile_rejects() {
        assert!(
            StartupOptions::capture_auto("?obrushprofile=sculptgl&obrushprofile=obrush", None)
                .unwrap()
                .source
                .is_none()
        );
        assert!(StartupOptions::capture_auto("?obrushprofile=SCULPTGL", None).is_err());
    }
    #[test]
    fn native_unknown_flags_and_oversized_query_fail_before_app_creation() {
        assert!(StartupOptions::capture_native(&["--unregistered=1".into()]).is_err());
        assert!(
            StartupOptions::capture_native(&[format!(
                "--startup-query={}",
                "x".repeat(MAX_QUERY_BYTES + 1)
            )])
            .is_err()
        );
    }
}
