//! Shared native/browser UI font ownership. No document or sculpt geometry.
//! Fonts become active on a later egui pass; bytes alone do not admit a locale.
use crate::app_preferences::Locale;
use eframe::egui::{self, FontData, FontDefinitions, FontFamily, FontId};
use std::sync::Arc;

const MAX_FONT_BYTES: usize = 2 * 1024 * 1024;
const MAX_ALL_FONT_BYTES: usize = 8 * 1024 * 1024;
const MAX_ATLAS_SIDE: usize = 4096;
const MAX_ATLAS_RGBA_BYTES: usize = 64 * 1024 * 1024;
const UNION: &str = include_str!("../assets/ui-fonts/union787.txt");
const JP: &[u8] = include_bytes!("../assets/ui-fonts/OBrushUIJP-Regular.otf");
const KR: &[u8] = include_bytes!("../assets/ui-fonts/OBrushUIKR-Regular.otf");
const TC: &[u8] = include_bytes!("../assets/ui-fonts/OBrushUITC-Regular.otf");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Chain {
    Japanese,
    Korean,
    Chinese,
}
impl Chain {
    fn for_locale(locale: Locale) -> Self {
        match locale {
            Locale::Korean => Self::Korean,
            Locale::Chinese => Self::Chinese,
            _ => Self::Japanese,
        }
    }
    fn key(self) -> &'static str {
        match self {
            Self::Japanese => "OBrush UI JP",
            Self::Korean => "OBrush UI KR",
            Self::Chinese => "OBrush UI TC",
        }
    }
    fn bytes(self) -> &'static [u8] {
        match self {
            Self::Japanese => JP,
            Self::Korean => KR,
            Self::Chinese => TC,
        }
    }
    fn sha256(self) -> &'static str {
        match self {
            Self::Japanese => "03f907c083b73b0f3bec6780162e40a89584d5a33c62a3ab3a4d3277de85b7f0",
            Self::Korean => "f64ad2793cb5fabf826398eb7cc092120e0ddd2f7a6582c1ecf551e01bc7860f",
            Self::Chinese => "c4580415ec72787c8abcaa1e61635e96e066580afbddf064683f3c5488e31bd9",
        }
    }
}

fn definitions(chain: Chain) -> Result<FontDefinitions, String> {
    let total = JP
        .len()
        .checked_add(KR.len())
        .and_then(|n| n.checked_add(TC.len()));
    if [JP, KR, TC].iter().any(|data| data.len() > MAX_FONT_BYTES)
        || total.is_none_or(|n| n > MAX_ALL_FONT_BYTES)
        || UNION.chars().count() != 787
    {
        return Err("Embedded UI font bounds or corpus changed".into());
    }
    // Preserve every default face, byte stream, tweak and fallback priority.
    let mut result = FontDefinitions::default();
    result.font_data.insert(
        chain.key().into(),
        Arc::new(FontData::from_static(chain.bytes())),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        result
            .families
            .get_mut(&family)
            .ok_or("Default UI font family is unavailable")?
            .push(chain.key().into());
    }
    Ok(result)
}

fn atlas_valid(size: [usize; 2]) -> bool {
    size.into_iter()
        .all(|side| side > 0 && side <= MAX_ATLAS_SIDE)
        && size[0]
            .checked_mul(size[1])
            .and_then(|n| n.checked_mul(4))
            .is_some_and(|n| n <= MAX_ATLAS_RGBA_BYTES)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Pending {
    id: u64,
    locale: Locale,
    chain: Chain,
    issued_pass: u64,
}

/// The caption layer can read this capability, but cannot construct admission.
#[derive(Clone, Copy, Debug)]
pub(crate) struct UiFontCapability {
    requested: Locale,
    effective: Locale,
    ready: bool,
}
impl UiFontCapability {
    pub(crate) fn requested(self) -> Locale {
        self.requested
    }
    pub(crate) fn effective(self) -> Locale {
        self.effective
    }
    pub(crate) fn ready(self) -> bool {
        self.ready
    }
}

#[derive(Debug)]
pub(crate) struct UiFonts {
    requested: Locale,
    effective: Locale,
    started: bool,
    next_id: u64,
    pending: Option<Pending>,
    scheduled_chain: Option<Chain>,
    admitted_chain: Option<Chain>,
    installation_count: u64,
    acknowledgment_count: u64,
    last_admitted_id: Option<u64>,
    atlas: [usize; 2],
    error: Option<String>,
    admitted_definitions: Option<serde_json::Value>,
    current_observation: Option<serde_json::Value>,
}
impl Default for UiFonts {
    fn default() -> Self {
        Self {
            requested: Locale::English,
            effective: Locale::English,
            started: false,
            next_id: 0,
            pending: None,
            scheduled_chain: None,
            admitted_chain: None,
            installation_count: 0,
            acknowledgment_count: 0,
            last_admitted_id: None,
            atlas: [0; 2],
            error: None,
            admitted_definitions: None,
            current_observation: None,
        }
    }
}
impl UiFonts {
    fn request(&mut self, locale: Locale, pass: u64) -> Result<Option<Chain>, String> {
        if self.started && self.requested == locale {
            return Ok(None);
        }
        let id = self
            .next_id
            .checked_add(1)
            .ok_or("UI font request counter exhausted")?;
        let chain = Chain::for_locale(locale);
        let install = self.scheduled_chain != Some(chain);
        self.requested = locale;
        self.started = true;
        self.next_id = id;
        self.error = None;
        self.pending = Some(Pending {
            id,
            locale,
            chain,
            issued_pass: pass,
        });
        if install {
            self.scheduled_chain = Some(chain);
            self.installation_count += 1;
        }
        Ok(install.then_some(chain))
    }
    fn complete(&mut self, id: u64, pass: u64, result: Result<[usize; 2], String>) -> bool {
        let Some(pending) = self.pending.filter(|p| p.id == id && pass > p.issued_pass) else {
            return false;
        };
        self.pending = None;
        match result {
            Ok(atlas) if atlas_valid(atlas) => {
                self.effective = pending.locale;
                self.admitted_chain = Some(pending.chain);
                self.atlas = atlas;
                self.last_admitted_id = Some(id);
                self.acknowledgment_count += 1;
                self.error = None;
            }
            Ok(_) => {
                self.error = Some("UI font atlas exceeded its bounded admission policy".into())
            }
            Err(error) => self.error = Some(error),
        }
        true
    }
    /// Called by the real shared update owner, before any translated captions.
    pub(crate) fn update(&mut self, ctx: &egui::Context, requested: Locale) {
        let pass = ctx.cumulative_pass_nr();
        match self.request(requested, pass) {
            Ok(Some(chain)) => match definitions(chain) {
                Ok(definitions) => {
                    ctx.set_fonts(definitions);
                    ctx.request_repaint();
                }
                Err(error) => {
                    self.pending = None;
                    self.error = Some(error);
                }
            },
            Ok(None) => {}
            Err(error) => self.error = Some(error),
        }
        if let Some(pending) = self.pending {
            if pass > pending.issued_pass {
                let observed = inspect(ctx, pending.chain);
                self.complete(pending.id, pass, observed);
                if self.last_admitted_id == Some(pending.id) && self.error.is_none() {
                    self.admitted_definitions = Some(definition_snapshot(ctx, pending.chain));
                }
            } else {
                ctx.request_repaint();
            }
        }
    }
    pub(crate) fn capability(&self) -> UiFontCapability {
        UiFontCapability {
            requested: self.requested,
            effective: self.effective,
            ready: self.admitted_chain.is_some(),
        }
    }
    /// Called once at the end of the real update. This reads existing engine
    /// state only; browser metrics getters read the cached bounded value.
    pub(crate) fn observe_end_update(&mut self, ctx: &egui::Context) {
        let pass = ctx.cumulative_pass_nr();
        let pixels_per_point = ctx.pixels_per_point();
        let max_texture_side = ctx.input(|input| input.max_texture_side);
        let atlas = ctx.fonts(|fonts| fonts.font_image_size());
        self.current_observation = Some(serde_json::json!({"pass":pass,
            "pixels_per_point":pixels_per_point,"pixels_per_point_bits":pixels_per_point.to_bits(),
            "atlas":atlas,"atlas_bounded":atlas_valid(atlas),"max_texture_side":max_texture_side,
            "scope":"Cached end-of-real-update observation before painting; no getter rasterization"}));
    }
    pub(crate) fn metrics(&self) -> serde_json::Value {
        serde_json::json!({"requested":self.requested.stable_id(),"effective":self.effective.stable_id(),
            "ready":self.admitted_chain.is_some(),"pending":self.pending.map(|p|serde_json::json!({"id":p.id,"locale":p.locale.stable_id(),"chain":p.chain.key(),"issued_pass":p.issued_pass})),
            "last_admitted_id":self.last_admitted_id,"error":self.error,"installation_count":self.installation_count,
            "acknowledgment_count":self.acknowledgment_count,"admitted_chain":self.admitted_chain.map(Chain::key),
            "asset_sha256":self.admitted_chain.map(Chain::sha256),"atlas":self.atlas,"corpus_scalars":787,
            "admitted_definitions":self.admitted_definitions,"current_observation":self.current_observation,
            "structural_newline_exclusions":1,"max_font_bytes":MAX_FONT_BYTES,"all_embedded_bytes":JP.len()+KR.len()+TC.len(),
            "max_atlas_side":MAX_ATLAS_SIDE,"max_atlas_rgba_bytes":MAX_ATLAS_RGBA_BYTES,
            "scope":"Finite static caption/native-picker coverage. Arbitrary user names and filenames remain verbatim; unrestricted Unicode and reference font appearance are unverified."})
    }
    pub(crate) fn status(&self) -> String {
        if let Some(error) = &self.error {
            format!(
                "Effective interface: {}. Font admission failed: {}. OBrush-specific details use English fallback.",
                self.effective.stable_id(),
                error
            )
        } else if self.pending.is_some() {
            format!(
                "Effective interface: {}. Requested {} is waiting for the next font pass. OBrush-specific details use English fallback.",
                self.effective.stable_id(),
                self.requested.stable_id()
            )
        } else {
            format!(
                "Effective interface: {}. Static captions and native picker glyphs admitted. OBrush-specific details use English fallback.",
                self.effective.stable_id()
            )
        }
    }
}

fn definition_snapshot(ctx: &egui::Context, chain: Chain) -> serde_json::Value {
    let expected = definitions(chain).ok();
    ctx.fonts(|fonts| {
        let actual = fonts.definitions();
        let rows: Vec<_> = actual.font_data.iter().map(|(name,data)| {
            let same = expected.as_ref().and_then(|d|d.font_data.get(name)).is_some_and(|d|d.as_ref()==data.as_ref());
            serde_json::json!({"name":name,"face_index":data.index,"bytes":data.font.len(),
                "tweak_bits":[data.tweak.scale.to_bits(),data.tweak.y_offset_factor.to_bits(),data.tweak.y_offset.to_bits()],
                "data_matches_expected":same})
        }).collect();
        serde_json::json!({"chain":chain.key(),"embedded_asset_sha256":chain.sha256(),
            "families":{"proportional":actual.families.get(&FontFamily::Proportional),"monospace":actual.families.get(&FontFamily::Monospace)},
            "data":rows,"complete_definitions_match":expected.as_ref()==Some(actual),
            "scope":"Actual admitted engine definitions, original stock plus exact embedded bytes/face/tweaks"})
    })
}

fn inspect(ctx: &egui::Context, chain: Chain) -> Result<[usize; 2], String> {
    let expected = definitions(chain)?;
    ctx.fonts_mut(|fonts| {
        if fonts.definitions() != &expected {
            return Err(
                "Actual UI font definitions differ from the requested stock-first chain".into(),
            );
        }
        check_glyphs(|family, character| fonts.has_glyph(&FontId::new(14.0, family), character))?;
        let size = fonts.font_image_size();
        if !atlas_valid(size) {
            return Err("Actual UI atlas bounds are unavailable or exceeded".into());
        }
        Ok(size)
    })
}

fn check_glyphs(mut present: impl FnMut(FontFamily, char) -> bool) -> Result<(), String> {
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        for character in UNION.chars() {
            // Exact structural newline exception from the checked original corpora.
            if character != '\n' && !present(family.clone(), character) {
                return Err(format!(
                    "Missing admitted UI glyph U+{:04X}",
                    character as u32
                ));
            }
        }
    }
    Ok(())
}

/// Bound the UI atlas before egui's next allocation, retaining smaller GPU caps.
pub(crate) fn bound_raw_input(input: &mut egui::RawInput) {
    input.max_texture_side = Some(
        input
            .max_texture_side
            .unwrap_or(MAX_ATLAS_SIDE)
            .min(MAX_ATLAS_SIDE),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn started(locale: Locale) -> UiFonts {
        let mut owner = UiFonts::default();
        owner.request(locale, 10).unwrap();
        owner
    }
    #[test]
    fn default_english_guard() {
        let owner = UiFonts::default();
        assert_eq!(owner.capability().effective(), Locale::English);
        assert!(!owner.capability().ready());
        assert_eq!(owner.installation_count, 0);
    }
    #[test]
    fn all_ten_chain_mappings() {
        for (locale, expected) in [
            (Locale::English, Chain::Japanese),
            (Locale::Japanese, Chain::Japanese),
            (Locale::Chinese, Chain::Chinese),
            (Locale::Korean, Chain::Korean),
            (Locale::Russian, Chain::Japanese),
            (Locale::Turkish, Chain::Japanese),
            (Locale::Swedish, Chain::Japanese),
            (Locale::French, Chain::Japanese),
            (Locale::German, Chain::Japanese),
            (Locale::Italian, Chain::Japanese),
        ] {
            assert_eq!(Chain::for_locale(locale), expected);
        }
    }
    #[test]
    fn stock_definitions_append_only() {
        let stock = FontDefinitions::default();
        assert_eq!(
            stock.families[&FontFamily::Proportional],
            ["Ubuntu-Light", "NotoEmoji-Regular", "emoji-icon-font"]
        );
        assert_eq!(
            stock.families[&FontFamily::Monospace],
            [
                "Hack",
                "Ubuntu-Light",
                "NotoEmoji-Regular",
                "emoji-icon-font"
            ]
        );
        assert_eq!(
            stock.font_data["NotoEmoji-Regular"].tweak.scale.to_bits(),
            0.81f32.to_bits()
        );
        assert_eq!(
            stock.font_data["emoji-icon-font"].tweak.scale.to_bits(),
            0.90f32.to_bits()
        );
        for chain in [Chain::Japanese, Chain::Korean, Chain::Chinese] {
            let actual = definitions(chain).unwrap();
            for (key, data) in &stock.font_data {
                assert_eq!(actual.font_data[key], *data);
            }
            assert_eq!(actual.font_data.len(), stock.font_data.len() + 1);
            for family in [FontFamily::Proportional, FontFamily::Monospace] {
                let mut expected = stock.families[&family].clone();
                expected.push(chain.key().into());
                assert_eq!(actual.families[&family], expected);
            }
            let data = &actual.font_data[chain.key()];
            assert_eq!(data.index, 0);
            assert_eq!(data.tweak, egui::FontTweak::default());
            assert_eq!(data.font.as_ref(), chain.bytes());
        }
    }
    #[test]
    fn asset_bounds() {
        assert_eq!(
            [JP.len(), KR.len(), TC.len()],
            [1_115_180, 1_112_428, 1_111_848]
        );
        assert_eq!(JP.len() + KR.len() + TC.len(), 3_339_456);
        assert_eq!(UNION.chars().count(), 787);
        assert_eq!(UNION.chars().filter(|c| *c == '\n').count(), 1);
        assert!(UNION.chars().all(|c| c != '\u{fffd}'));
    }
    #[test]
    fn same_pass_not_admitted() {
        let mut owner = started(Locale::Japanese);
        assert!(!owner.complete(1, 10, Ok([1024, 128])));
        assert_eq!(owner.effective, Locale::English);
        assert!(owner.pending.is_some());
    }
    #[test]
    fn later_pass_success() {
        let mut owner = started(Locale::Japanese);
        assert!(owner.complete(1, 11, Ok([1024, 128])));
        assert_eq!(owner.effective, Locale::Japanese);
        assert_eq!(owner.last_admitted_id, Some(1));
        assert_eq!(owner.acknowledgment_count, 1);
        assert!(owner.capability().ready());
    }
    #[test]
    fn wrong_definitions_reject() {
        let mut owner = started(Locale::Japanese);
        let ctx = egui::Context::default();
        let mut observed = Ok([0; 2]);
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            observed = inspect(ctx, Chain::Japanese)
        });
        assert!(observed.as_ref().unwrap_err().contains("stock-first chain"));
        assert!(owner.complete(1, 11, observed));
        assert_eq!(owner.effective, Locale::English);
        assert!(!owner.capability().ready());
    }
    #[test]
    fn missing_glyph_reject() {
        let mut owner = started(Locale::Korean);
        let missing = check_glyphs(|_, character| character != '這');
        assert_eq!(missing, Err("Missing admitted UI glyph U+9019".into()));
        owner.complete(1, 11, missing.map(|()| [1024, 128]));
        assert_eq!(owner.effective, Locale::English);
        assert!(owner.error.as_ref().unwrap().contains("U+9019"));
    }
    #[test]
    fn atlas_cap_reject() {
        assert!(!atlas_valid([0, 64]));
        assert!(!atlas_valid([4097, 64]));
        assert!(!atlas_valid([usize::MAX, usize::MAX]));
        assert!(atlas_valid([4096, 4096]));
        let mut owner = started(Locale::Chinese);
        owner.complete(1, 11, Ok([4097, 64]));
        assert_eq!(owner.effective, Locale::English);
        assert!(!owner.capability().ready());
    }
    #[test]
    fn stale_ack_ignored() {
        let mut owner = started(Locale::Japanese);
        owner.request(Locale::Chinese, 10).unwrap();
        assert!(!owner.complete(1, 11, Ok([1024, 128])));
        assert_eq!(owner.pending.unwrap().locale, Locale::Chinese);
        assert!(owner.complete(2, 11, Ok([1024, 128])));
        assert_eq!(owner.effective, Locale::Chinese);
    }
    #[test]
    fn english_reset_invalidates() {
        let mut owner = started(Locale::Korean);
        owner.request(Locale::English, 10).unwrap();
        assert!(!owner.complete(1, 11, Ok([1024, 128])));
        assert!(owner.complete(2, 11, Ok([1024, 128])));
        assert_eq!(owner.effective, Locale::English);
    }
    #[test]
    fn rapid_selection_latest_wins() {
        let mut owner = started(Locale::Chinese);
        owner.request(Locale::Korean, 10).unwrap();
        owner.request(Locale::Japanese, 10).unwrap();
        assert!(!owner.complete(1, 11, Ok([1024, 128])));
        assert!(!owner.complete(2, 11, Ok([1024, 128])));
        assert!(owner.complete(3, 11, Ok([1024, 128])));
        assert_eq!(owner.effective, Locale::Japanese);
    }
    #[test]
    fn same_chain_no_reinstall() {
        let mut owner = started(Locale::Japanese);
        owner.complete(1, 11, Ok([1024, 128]));
        assert_eq!(owner.request(Locale::Turkish, 11).unwrap(), None);
        assert_eq!(owner.installation_count, 1);
        assert_eq!(owner.effective, Locale::Japanese);
        assert!(!owner.complete(2, 11, Ok([1024, 128])));
        owner.complete(2, 12, Ok([1024, 128]));
        assert_eq!(owner.effective, Locale::Turkish);
    }
    #[test]
    fn failed_request_retains_effective() {
        let mut owner = started(Locale::Japanese);
        owner.complete(1, 11, Ok([1024, 128]));
        owner.request(Locale::Chinese, 11).unwrap();
        owner.complete(2, 12, Err("Missing admitted UI glyph".into()));
        assert_eq!(owner.effective, Locale::Japanese);
        assert!(owner.error.is_some());
        assert_eq!(owner.last_admitted_id, Some(1));
    }
    #[test]
    fn idle_no_reinstall() {
        let mut owner = started(Locale::French);
        owner.complete(1, 11, Ok([1024, 128]));
        for pass in 12..32 {
            assert_eq!(owner.request(Locale::French, pass).unwrap(), None);
        }
        assert_eq!(owner.installation_count, 1);
        assert_eq!(owner.acknowledgment_count, 1);
    }
    #[test]
    fn serial_overflow_guard() {
        let mut owner = UiFonts {
            next_id: u64::MAX,
            ..Default::default()
        };
        assert!(owner.request(Locale::Japanese, 10).is_err());
        assert!(!owner.started);
        assert_eq!(owner.effective, Locale::English);
        assert!(owner.pending.is_none());
    }
    fn pass(ctx: &egui::Context, owner: &mut UiFonts, locale: Locale) {
        let mut input = egui::RawInput::default();
        bound_raw_input(&mut input);
        let _ = ctx.run(input, |ctx| owner.update(ctx, locale));
    }
    #[test]
    fn actual_egui_next_pass_admission() {
        let ctx = egui::Context::default();
        let mut owner = UiFonts::default();
        pass(&ctx, &mut owner, Locale::Japanese);
        assert!(!owner.capability().ready());
        pass(&ctx, &mut owner, Locale::Japanese);
        assert_eq!(owner.effective, Locale::Japanese);
        assert!(owner.capability().ready());
        assert!(owner.error.is_none());
        assert_eq!(owner.installation_count, 1);
    }
    #[test]
    fn actual_egui_all_locales() {
        let ctx = egui::Context::default();
        let mut owner = UiFonts::default();
        for locale in Locale::ALL {
            pass(&ctx, &mut owner, locale);
            pass(&ctx, &mut owner, locale);
            assert_eq!(owner.capability().requested(), locale);
            assert_eq!(owner.effective, locale);
            assert!(owner.capability().ready());
            assert!(owner.error.is_none());
        }
        assert_eq!(owner.installation_count, 4);
    }
    #[test]
    fn raw_input_texture_bound() {
        for (before, after) in [
            (None, 4096),
            (Some(8192), 4096),
            (Some(2048), 2048),
            (Some(1024), 1024),
        ] {
            let mut input = egui::RawInput {
                max_texture_side: before,
                ..Default::default()
            };
            bound_raw_input(&mut input);
            assert_eq!(input.max_texture_side, Some(after));
        }
    }
    #[test]
    fn admitted_definition_snapshot_exact_stock_and_chain() {
        let ctx = egui::Context::default();
        let mut owner = UiFonts::default();
        pass(&ctx, &mut owner, Locale::Japanese);
        pass(&ctx, &mut owner, Locale::Japanese);
        let d = owner.admitted_definitions.as_ref().unwrap();
        assert_eq!(
            d["families"]["proportional"],
            serde_json::json!([
                "Ubuntu-Light",
                "NotoEmoji-Regular",
                "emoji-icon-font",
                "OBrush UI JP"
            ])
        );
        assert_eq!(
            d["families"]["monospace"],
            serde_json::json!([
                "Hack",
                "Ubuntu-Light",
                "NotoEmoji-Regular",
                "emoji-icon-font",
                "OBrush UI JP"
            ])
        );
        assert_eq!(d["complete_definitions_match"], true);
        let rows = d["data"].as_array().unwrap();
        assert_eq!(rows.len(), 5);
        assert!(
            rows.iter()
                .all(|row| row["face_index"] == 0 && row["data_matches_expected"] == true)
        );
        let jp = rows
            .iter()
            .find(|row| row["name"] == "OBrush UI JP")
            .unwrap();
        assert_eq!(jp["bytes"], 1_115_180);
        assert_eq!(jp["tweak_bits"], serde_json::json!([0x3f800000u32, 0, 0]));
    }
    #[test]
    fn default_snapshot_reports_no_cjk_instead_of_invented_admission() {
        let ctx = egui::Context::default();
        let mut observed = serde_json::Value::Null;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            observed = definition_snapshot(ctx, Chain::Japanese)
        });
        assert_eq!(observed["complete_definitions_match"], false);
        assert_eq!(observed["data"].as_array().unwrap().len(), 4);
        assert!(
            observed["families"]["proportional"]
                .as_array()
                .unwrap()
                .iter()
                .all(|key| key != "OBrush UI JP")
        );
    }
    #[test]
    fn real_end_pass_atlas_cache_has_current_pass_and_device_scale() {
        let ctx = egui::Context::default();
        let mut owner = UiFonts::default();
        for _ in 0..2 {
            let mut input = egui::RawInput::default();
            bound_raw_input(&mut input);
            let _ = ctx.run(input, |ctx| {
                owner.update(ctx, Locale::English);
                owner.observe_end_update(ctx);
            });
        }
        let observed = owner.current_observation.as_ref().unwrap();
        assert_eq!(observed["pass"], ctx.cumulative_pass_nr() - 1);
        assert_eq!(
            observed["pixels_per_point_bits"],
            ctx.pixels_per_point().to_bits()
        );
        assert_eq!(observed["atlas_bounded"], true);
        assert_eq!(observed["max_texture_side"], 4096);
        assert_eq!(observed["atlas"].as_array().unwrap().len(), 2);
    }
}
