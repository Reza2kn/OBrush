//! Shared host preference data. No UI, document, renderer, font, file or timer.
//! Source-only donor; native/WASM compilation and normal adapters remain unrun.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PreferenceProfile {
    #[default]
    OBrush,
    SculptGl,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Locale {
    #[default]
    English,
    Japanese,
    Chinese,
    Korean,
    Russian,
    Turkish,
    Swedish,
    French,
    German,
    Italian,
}

impl Locale {
    pub const ALL: [Self; 10] = [
        Self::English,
        Self::Japanese,
        Self::Chinese,
        Self::Korean,
        Self::Russian,
        Self::Turkish,
        Self::Swedish,
        Self::French,
        Self::German,
        Self::Italian,
    ];

    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::English => "english",
            Self::Japanese => "japanese",
            Self::Chinese => "chinese",
            Self::Korean => "korean",
            Self::Russian => "russian",
            Self::Turkish => "turkish",
            Self::Swedish => "swedish",
            Self::French => "french",
            Self::German => "german",
            Self::Italian => "italian",
        }
    }

    /// Exact pinned GuiTR picker names; stable IDs stay unchanged.
    pub const fn native_name(self) -> &'static str {
        match self {
            Self::English => "english",
            Self::Japanese => "日本語",
            Self::Chinese => "中文",
            Self::Korean => "한국어",
            Self::Russian => "русский",
            Self::Turkish => "turkish",
            Self::Swedish => "svenska",
            Self::French => "français",
            Self::German => "deutsch",
            Self::Italian => "italiano",
        }
    }

    pub fn source_url(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|locale| locale.stable_id() == value)
    }

    pub fn source_initial(navigator: Option<&str>, query: Option<&str>) -> Self {
        let prefix = navigator.unwrap_or("").chars().take(2).collect::<String>();
        let automatic = match prefix.as_str() {
            "ja" => Self::Japanese,
            "zh" => Self::Chinese,
            "ko" => Self::Korean,
            "tr" => Self::Turkish,
            "sv" => Self::Swedish,
            "fr" => Self::French,
            "de" => Self::German,
            _ => Self::English,
        };
        query.and_then(Self::source_url).unwrap_or(automatic)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourcePalette {
    widget: [f64; 4],
    background: [f64; 3],
    text: [f64; 4],
    border: bool,
}

impl SourcePalette {
    pub const fn widget(&self) -> [f64; 4] {
        self.widget
    }
    pub const fn background(&self) -> [f64; 3] {
        self.background
    }
    pub const fn text(&self) -> [f64; 4] {
        self.text
    }
    pub const fn border(&self) -> bool {
        self.border
    }
    pub fn set_border(&mut self, value: bool) {
        self.border = value;
    }
    pub fn edit_widget(&mut self, color: [f64; 4]) -> Result<(), &'static str> {
        let validated = Self::from_overall(color)?;
        self.widget = validated.widget;
        Ok(())
    }
    pub fn edit_background(&mut self, color: [f64; 3]) -> Result<(), &'static str> {
        Self::from_overall([color[0], color[1], color[2], 1.0])?;
        self.background = color;
        Ok(())
    }
    pub fn edit_text(&mut self, color: [f64; 4]) -> Result<(), &'static str> {
        Self::from_overall(color)?;
        self.text = color;
        Ok(())
    }
    /// Derived topbar/menu background is half the independently edited Back RGB.
    pub fn topbar_background(&self) -> [f64; 3] {
        [
            self.background[0] * 0.5,
            self.background[1] * 0.5,
            self.background[2] * 0.5,
        ]
    }
    pub fn from_overall(color: [f64; 4]) -> Result<Self, &'static str> {
        if !color
            .iter()
            .all(|x| x.is_finite() && (0.0..=1.0).contains(x))
        {
            return Err("Palette requires finite normalized RGBA channels");
        }
        Ok(Self {
            widget: color,
            background: [color[0] * 0.5, color[1] * 0.5, color[2] * 0.5],
            text: [
                (color[0] + 0.5).min(0.8),
                (color[1] + 0.5).min(0.8),
                (color[2] + 0.5).min(0.8),
                color[3],
            ],
            border: false,
        })
    }

    pub fn edit_overall(&mut self, color: [f64; 4]) -> Result<(), &'static str> {
        let mut candidate = Self::from_overall(color)?;
        candidate.border = self.border;
        *self = candidate;
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum ThemePreference {
    /// Existing encoded Dark colors remain in app.rs; this enum never applies UI.
    #[default]
    LegacyDark,
    SourceCustom(SourcePalette),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AppPreferences {
    pub locale: Locale,
    pub theme: ThemePreference,
    pub shortcut_profile: PreferenceProfile,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_preferences_keep_english_dark_legacy() {
        let p = AppPreferences::default();
        assert_eq!(p.locale, Locale::English);
        assert_eq!(p.theme, ThemePreference::LegacyDark);
        assert_eq!(p.shortcut_profile, PreferenceProfile::OBrush);
    }

    #[test]
    fn native_picker_exact() {
        assert_eq!(
            Locale::ALL.map(Locale::native_name),
            [
                "english",
                "日本語",
                "中文",
                "한국어",
                "русский",
                "turkish",
                "svenska",
                "français",
                "deutsch",
                "italiano"
            ]
        );
        assert_eq!(
            Locale::ALL.map(Locale::stable_id),
            [
                "english", "japanese", "chinese", "korean", "russian", "turkish", "swedish",
                "french", "german", "italian"
            ]
        );
    }
    #[test]
    fn ten_locale_ids_are_distinct_exact_query_choices() {
        let ids = Locale::ALL.map(Locale::stable_id);
        assert_eq!(
            ids,
            [
                "english", "japanese", "chinese", "korean", "russian", "turkish", "swedish",
                "french", "german", "italian"
            ]
        );
        for locale in Locale::ALL {
            assert_eq!(Locale::source_url(locale.stable_id()), Some(locale));
        }
        assert_eq!(Locale::source_url("Japanese"), None);
    }

    #[test]
    fn only_seven_source_navigator_prefixes_auto_select() {
        for (prefix, locale) in [
            ("ja-JP", Locale::Japanese),
            ("zh-CN", Locale::Chinese),
            ("ko-KR", Locale::Korean),
            ("tr-TR", Locale::Turkish),
            ("sv-SE", Locale::Swedish),
            ("fr-FR", Locale::French),
            ("de-DE", Locale::German),
        ] {
            assert_eq!(Locale::source_initial(Some(prefix), None), locale);
        }
        for language in ["ru-RU", "it-IT", "en-US", "xx", "JA"] {
            assert_eq!(
                Locale::source_initial(Some(language), None),
                Locale::English
            );
        }
    }

    #[test]
    fn valid_url_overrides_and_unknown_url_retains_navigator() {
        assert_eq!(
            Locale::source_initial(Some("ja"), Some("italian")),
            Locale::Italian
        );
        assert_eq!(
            Locale::source_initial(Some("ja"), Some("Italian")),
            Locale::Japanese
        );
        assert_eq!(
            Locale::source_initial(None, Some("unknown")),
            Locale::English
        );
    }

    #[test]
    fn independently_typed_palette_math_keeps_alpha_and_border() {
        let mut p = SourcePalette::from_overall([0.0, 0.25, 1.0, 0.125]).unwrap();
        assert_eq!(p.background, [0.0, 0.125, 0.5]);
        assert_eq!(p.text, [0.5, 0.75, 0.8, 0.125]);
        p.border = true;
        p.edit_overall([1.0, 0.0, 0.25, 1.0]).unwrap();
        assert_eq!(p.widget, [1.0, 0.0, 0.25, 1.0]);
        assert_eq!(p.text, [0.8, 0.5, 0.75, 1.0]);
        assert!(p.border);
    }

    #[test]
    fn individual_palette_slots_preserve_other_roles_and_reject_atomically() {
        let mut p = SourcePalette::from_overall([0.25, 0.5, 0.75, 1.0]).unwrap();
        let text = p.text();
        let back = p.background();
        p.edit_widget([0.5, 0.25, 0.0, 0.5]).unwrap();
        assert_eq!(p.text(), text);
        assert_eq!(p.background(), back);
        p.edit_background([0.5, 0.25, 1.0]).unwrap();
        assert_eq!(p.topbar_background(), [0.25, 0.125, 0.5]);
        p.edit_text([1.0, 0.5, 0.25, 0.5]).unwrap();
        let old = p.clone();
        assert!(p.edit_background([f64::NAN, 0.0, 0.0]).is_err());
        assert_eq!(p, old);
    }
    #[test]
    fn invalid_palette_edit_is_atomic() {
        let mut p = SourcePalette::from_overall([0.3, 0.34, 0.4, 1.0]).unwrap();
        let before = p.clone();
        for bad in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
            assert!(p.edit_overall([bad, 0.0, 0.0, 1.0]).is_err());
            assert_eq!(p, before);
        }
    }
}
