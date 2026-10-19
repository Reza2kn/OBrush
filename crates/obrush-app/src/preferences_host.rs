//! Production boot/preferences/phase command adapter. This donor is source-only:
//! held brush-setting modal wiring is bounded and Source Paint-only for sampling.
//! Held camera/modifier restoration remain gaps. Names do not establish parity.
use super::*;
use crate::app_preferences::{Locale, PreferenceProfile, SourcePalette, ThemePreference};
use crate::shortcut_profile::{Command, Direction, KeyDelivery, Modifiers, ShortcutContext};
use crate::startup_model::MAX_MODEL_BYTES;
use crate::startup_options::{SourceStartupSettings, StartupOptions};

const SOURCE_CAMERA_KEY_INTERVAL: std::time::Duration = std::time::Duration::from_micros(16_600);

#[derive(Default)]
pub(super) struct SourceKeyboardCamera {
    move_x: i8,
    move_z: i8,
    next_tick: Option<web_time::Instant>,
}
impl SourceKeyboardCamera {
    fn set(&mut self, direction: Direction, pressed: bool, now: web_time::Instant) {
        let axis = match direction {
            Direction::Left | Direction::Right => &mut self.move_x,
            Direction::Up | Direction::Down => &mut self.move_z,
        };
        if pressed {
            *axis = match direction {
                Direction::Left | Direction::Up => -1,
                Direction::Right | Direction::Down => 1,
            };
            if self.next_tick.is_none() {
                self.next_tick = Some(now + SOURCE_CAMERA_KEY_INTERVAL);
            }
        } else {
            // The pinned key-up handler clears the whole axis, even when its
            // opposite key has also been held.
            *axis = 0;
            if !self.active() {
                self.next_tick = None;
            }
        }
    }
    fn active(&self) -> bool {
        self.move_x != 0 || self.move_z != 0
    }
    fn due(&mut self, now: web_time::Instant) -> Option<(i8, i8)> {
        if !self.active() {
            return None;
        }
        let next = self
            .next_tick
            .get_or_insert(now + SOURCE_CAMERA_KEY_INTERVAL);
        if now < *next {
            return None;
        }
        // One step per rendered frame. Never accumulate seconds of movement
        // while browser rendering or a native window stalls.
        *next = now + SOURCE_CAMERA_KEY_INTERVAL;
        Some((self.move_x, self.move_z))
    }
    fn clear(&mut self) {
        *self = Self::default();
    }
}

#[derive(Clone, Copy)]
struct PaletteTokens {
    muted: Color32,
    panel: Color32,
    top: Color32,
}
const PALETTE_KEY: &str = "obrush.app.palette";
fn color4(v: [f64; 4]) -> Color32 {
    egui::Rgba::from_rgba_unmultiplied(v[0] as f32, v[1] as f32, v[2] as f32, v[3] as f32).into()
}
fn color3(v: [f64; 3]) -> Color32 {
    color4([v[0], v[1], v[2], 1.0])
}
pub(super) fn muted(ui: &egui::Ui) -> Color32 {
    ui.ctx()
        .data(|d| d.get_temp::<PaletteTokens>(egui::Id::new(PALETTE_KEY)))
        .map_or(MUTED, |p| p.muted)
}
pub(super) fn panel(ctx: &egui::Context) -> Color32 {
    ctx.data(|d| d.get_temp::<PaletteTokens>(egui::Id::new(PALETTE_KEY)))
        .map_or(PANEL, |p| p.panel)
}
pub(super) fn top_panel(ctx: &egui::Context) -> Color32 {
    ctx.data(|d| d.get_temp::<PaletteTokens>(egui::Id::new(PALETTE_KEY)))
        .map_or(PANEL, |p| p.top)
}
fn apply_theme(ctx: &egui::Context, preference: &ThemePreference) {
    configure_theme(ctx);
    match preference {
        ThemePreference::LegacyDark => ctx.data_mut(|d| {
            d.remove::<PaletteTokens>(egui::Id::new(PALETTE_KEY));
        }),
        ThemePreference::SourceCustom(p) => {
            let tokens = PaletteTokens {
                muted: color4(p.text()),
                panel: color3(p.background()),
                top: color3(p.topbar_background()),
            };
            let mut style = (*ctx.style()).clone();
            let text = color4(p.text());
            let widget = color4(p.widget());
            style.visuals.panel_fill = tokens.panel;
            style.visuals.window_fill = tokens.panel;
            style.visuals.override_text_color = Some(text);
            style.visuals.selection.bg_fill = widget;
            style.visuals.selection.stroke = Stroke::new(1.0, text);
            for state in [
                &mut style.visuals.widgets.noninteractive,
                &mut style.visuals.widgets.inactive,
                &mut style.visuals.widgets.hovered,
                &mut style.visuals.widgets.active,
                &mut style.visuals.widgets.open,
            ] {
                state.bg_fill = widget;
                state.weak_bg_fill = widget;
                state.fg_stroke = Stroke::new(1.0, text);
                state.bg_stroke = if p.border() {
                    Stroke::new(1.0, text)
                } else {
                    Stroke::NONE
                };
            }
            ctx.set_style(style);
            ctx.data_mut(|d| d.insert_temp(egui::Id::new(PALETTE_KEY), tokens));
        }
    }
}
fn shading(code: u8) -> Option<(ShadingMode, bool)> {
    match code {
        0 => Some((ShadingMode::Pbr, false)),
        1 => Some((ShadingMode::Flat, false)),
        2 => Some((ShadingMode::Normal, false)),
        3 => Some((ShadingMode::Flat, true)),
        4 => Some((ShadingMode::Uv, false)),
        5 => Some((ShadingMode::Matcap, false)),
        _ => None,
    }
}
fn profile_name(p: PreferenceProfile) -> &'static str {
    match p {
        PreferenceProfile::OBrush => "OBrush",
        PreferenceProfile::SculptGl => "SculptGL basic",
    }
}
// Nineteen render/import fields plus two resolver-owned observations. These
// locale/binding values are current owner state, not retained raw query text.
fn source_metrics(
    s: &SourceStartupSettings,
    locale: Locale,
    resolver: &crate::shortcut_profile::ShortcutResolver,
) -> serde_json::Value {
    serde_json::json!({"scale_center":s.scale_center,"grid":s.grid,"outline":s.outline,"outline_color":s.outline_color,
        "mirror_line":s.mirror_line,"darken_unselected":s.darken_unselected,"projection":s.projection,
        "camera_mode":s.camera_mode,"pivot":s.pivot,"fov":s.fov,"flat_shading":s.flat_shading,
        "wireframe":s.wireframe,"curvature":s.curvature,"exposure":s.exposure,"environment":s.environment,
        "matcap":s.matcap,"shader":s.shader,"filmic":s.filmic,"model_url":s.model_url,
        "language":locale.stable_id(),"shortcuts":{"profile":profile_name(resolver.profile()),"bindings_debug":format!("{resolver:?}")}})
}
fn keycode(key: egui::Key) -> Option<u16> {
    use egui::Key::*;
    Some(match key {
        ArrowLeft => 37,
        ArrowRight => 39,
        ArrowUp => 38,
        ArrowDown => 40,
        Space => 32,
        Delete => 46,
        Escape => 27,
        Num0 => 48,
        Num1 => 49,
        Num2 => 50,
        Num3 => 51,
        Num4 => 52,
        Num5 => 53,
        Num6 => 54,
        Num7 => 55,
        Num8 => 56,
        Num9 => 57,
        A => 65,
        B => 66,
        C => 67,
        D => 68,
        E => 69,
        F => 70,
        G => 71,
        H => 72,
        I => 73,
        J => 74,
        K => 75,
        L => 76,
        M => 77,
        N => 78,
        O => 79,
        P => 80,
        Q => 81,
        R => 82,
        S => 83,
        T => 84,
        U => 85,
        V => 86,
        W => 87,
        X => 88,
        Y => 89,
        Z => 90,
        OpenBracket => 219,
        CloseBracket => 221,
        _ => return None,
    })
}
/// A text editor must retain its keyboard. Fixed history/file commands still use
/// their production barriers outside an editor, including focused toolbar tiles.
fn editor_owns_keys(ctx: &egui::Context) -> bool {
    ctx.wants_keyboard_input()
        && ctx
            .memory(|m| m.focused())
            .is_some_and(|id| egui::TextEdit::load_state(ctx, id).is_some())
}

/// Preserve the host's press-repeat admission before Escape/editor routing.
fn source_key_phase_admitted(pressed: bool, repeat: bool) -> bool {
    !(repeat && pressed)
}

fn preference_response_observation(response: &egui::Response) -> serde_json::Value {
    let r = response.rect;
    serde_json::json!({"id_hex":format!("{:016x}", response.id.value()),"has_focus":response.has_focus(),
        "enabled":response.enabled(),"rect_xywh":[r.min.x,r.min.y,r.width(),r.height()]})
}

fn clipped_preference_response_observation(
    response: &egui::Response,
    ui: &egui::Ui,
) -> serde_json::Value {
    let mut observed = preference_response_observation(response);
    let r = ui.clip_rect();
    observed["clip_xywh"] = serde_json::json!([r.min.x, r.min.y, r.width(), r.height()]);
    observed["fully_inside_clip"] =
        serde_json::json!(r.contains_rect(response.rect) && ui.is_rect_visible(response.rect));
    observed
}

impl OBrushApp {
    pub(super) fn apply_startup(&mut self, mut startup: StartupOptions) -> Result<(), String> {
        self.app_preferences = startup.preferences;
        self.shortcut_resolver = startup.shortcuts;
        apply_theme(&self.egui_ctx, &self.app_preferences.theme);
        let Some(mut source) = startup.source.take() else {
            return Ok(());
        };
        source.clamp_presets(
            ENVIRONMENT_PRESETS.len() as u32,
            crate::matcap::MATCAP_NAMES.len() as u32,
        )?;
        self.camera.orthographic = source.projection == 1;
        self.camera.mode = match source.camera_mode {
            0 => CameraMode::Orbit,
            1 => CameraMode::Spherical,
            _ => CameraMode::Plane,
        };
        self.camera.use_pivot = source.pivot;
        self.camera.fov_degrees = source.fov as f32;
        self.camera.source_vertical_fov = true;
        self.camera.boot_source_fov = Some(source.fov);
        self.camera.rotation = Some(glam::Quat::IDENTITY);
        self.camera.yaw = 0.0;
        self.camera.pitch = 0.0;
        self.source_boot_framing_pending = true;
        self.viewport_style.use_vertex_color = true;
        self.viewport_style.flat_shading = source.flat_shading;
        self.viewport_style.wireframe = source.wireframe;
        self.viewport_style.curvature = source.curvature as f32;
        self.viewport_style.matcap_index = source.matcap;
        self.viewport_style.environment.preset = source.environment;
        self.viewport_style.exposure = source.resolved_exposure(f64::from(
            ENVIRONMENT_PRESETS[source.environment as usize].default_exposure,
        ))? as f32;
        self.viewport_style.environment.filmic = source.filmic;
        self.viewport_style.environment.darken_unselected = source.darken_unselected;
        self.viewport_style.source_display.show_grid = source.grid;
        self.viewport_style.source_display.show_contour = source.outline;
        self.viewport_style.source_display.contour_color = source.outline_color.map(|v| v as f32);
        self.viewport_style.source_display.show_symmetry_line = source.mirror_line;
        self.source_sgl_import.display_override = Some(obrush_core::SglDisplaySettings {
            show_grid: source.grid,
            show_symmetry_line: source.mirror_line,
            show_contour: source.outline,
        });
        self.source_sgl_import.options.scale_center = source.scale_center;
        self.source_sgl_import.options.vertex_srgb = true;
        self.startup_gaps=vec!["Held C/X use exposed idle setting owners; S needs ready Source Paint and a small active visible mesh. Dense S, held-camera and Source modifier restoration remain unavailable.".into(),
            "Source Add imports SGL/OBJ/PLY/STL; matched source multi-object OBJ/PLY/STL parsing remains an interchange gap.".into(),
            "Source scene shortcuts act on the active part, not a multi-selection transaction.".into(),
            "Generic keycodes outside mapped Egui letters/digits/arrows/space/Delete/Escape/brackets and keyboard repeats are unsupported.".into()];
        if let Some((mode, wire)) = shading(source.shader) {
            self.viewport_style.shading = mode;
            self.viewport_style.wire_only = wire;
        } else {
            self.startup_gaps.push(format!(
                "Internal shader role{} is captured but cannot be selected for a sculpt viewport",
                source.shader
            ));
        }
        if source.model_url.is_some() {
            // Boot-only clear before any file read. This is not a user document
            // replacement and does not manufacture undo entries for a placeholder.
            self.document = Document::empty();
            self.rendered_scene.clear();
            self.ui_subtool_id = None;
            self.startup_model_pending = source.model_url.clone();
        }
        self.startup_source = Some(source);
        Ok(())
    }
    pub fn source_shortcut_profile(&self) -> bool {
        self.app_preferences.shortcut_profile == PreferenceProfile::SculptGl
    }
    pub(super) fn app_preferences_metrics(&self) -> serde_json::Value {
        let palette = match &self.app_preferences.theme {
            ThemePreference::LegacyDark => serde_json::Value::Null,
            ThemePreference::SourceCustom(p) => {
                serde_json::json!({"widget":p.widget(),"background":p.background(),"text":p.text(),"border":p.border(),"top":p.topbar_background()})
            }
        };
        serde_json::json!({"requested_locale":self.app_preferences.locale.stable_id(),"effective_locale":self.ui_fonts.capability().effective().stable_id(),
            "catalog_supported":true,"fonts_installed":self.ui_fonts.capability().ready(),"font_admission":self.ui_fonts.metrics(),"public_text":crate::ui_text::metadata(self.app_preferences.locale),
            "catalog_data":crate::i18n::catalog_metadata(self.app_preferences.locale).unwrap_or_else(|error|serde_json::json!({"error":format!("{error:?}"),"effective_ui_ready":false})),
            "shortcut_profile":profile_name(self.app_preferences.shortcut_profile),"source_basic_only":true,
            "palette":palette,"startup":self.startup_source.as_ref().map(|s|source_metrics(s,self.app_preferences.locale,&self.shortcut_resolver)),"gaps":self.startup_gaps,
            "controls":self.app_preference_controls,"locale_response_observations":self.locale_response_observations,
            "preferences_window_bounds":self.preferences_window_observation,"last_locale_barrier":self.last_locale_barrier_observation,"model":self.startup_models.metrics()})
    }
    fn track_preference(&mut self, id: &str, response: &egui::Response) {
        let rect = response.rect;
        self.app_preference_controls.insert(
            id.into(),
            [rect.min.x, rect.min.y, rect.width(), rect.height()],
        );
    }
    fn preference_barrier(&mut self) -> bool {
        self.modal_cancel("application preference change");
        self.finish_stroke();
        if self.stroke_rejected {
            return false;
        }
        self.cancel_transform_drag();
        self.cancel_mask_region("Preferences changed; pending selection cancelled.", true);
        self.paint_pick_armed = false;
        self.paint_pick_dragging = false;
        true
    }
    pub(super) fn preferences_window(&mut self, ctx: &egui::Context) {
        self.locale_response_observations.clear();
        self.app_preference_controls
            .retain(|key, _| key == "preferences.open" || key == "preferences.help");
        let mut open = self.show_app_preferences;
        let shown = egui::Window::new(crate::ui_text::caption(
            crate::ui_text::CaptionKey::CAA1F976CA46F,
        ))
        .id(egui::Id::new("app_preferences_window"))
        .open(&mut open)
        .default_width(380.0)
        .show(ctx, |ui| {
            let mut locale = self.app_preferences.locale;
            let select = egui::ComboBox::from_id_salt("requested_locale")
                .width(ui.available_width())
                .truncate()
                .selected_text(locale.native_name())
                .show_ui(ui, |ui| {
                    for candidate in Locale::ALL {
                        let row =
                            ui.selectable_value(&mut locale, candidate, candidate.native_name());
                        self.track_preference(
                            &format!("preferences.locale.{}", candidate.stable_id()),
                            &row,
                        );
                        self.locale_response_observations.insert(
                            format!("preferences.locale.{}", candidate.stable_id()),
                            clipped_preference_response_observation(&row,ui));
                    }
                })
                .response;
            self.track_preference("preferences.locale", &select);
            self.locale_response_observations.insert("preferences.locale".into(),clipped_preference_response_observation(&select,ui));
            if locale != self.app_preferences.locale {
                let previous = self.app_preferences.locale;
                let before = self.locale_barrier_observation();
                let accepted = self.preference_barrier();
                if accepted { self.app_preferences.locale = locale; }
                self.last_locale_barrier_observation = Some(serde_json::json!({
                    "previous":previous.stable_id(),"requested":locale.stable_id(),
                    "applied":self.app_preferences.locale.stable_id(),"accepted":accepted,
                    "pass":ctx.cumulative_pass_nr().to_string(),"before":before,"after":self.locale_barrier_observation()}));
            }
            ui.label(self.ui_fonts.status());
            ui.separator();
            let mut profile = self.app_preferences.shortcut_profile;
            let select = egui::ComboBox::from_id_salt("shortcut_profile")
                .selected_text(profile_name(profile))
                .show_ui(ui, |ui| {
                    for candidate in [PreferenceProfile::OBrush, PreferenceProfile::SculptGl] {
                        let row =
                            ui.selectable_value(&mut profile, candidate, profile_name(candidate));
                        self.track_preference(
                            if candidate == PreferenceProfile::OBrush {
                                "preferences.shortcuts.obrush"
                            } else {
                                "preferences.shortcuts.sculptgl"
                            },
                            &row,
                        );
                    }
                })
                .response;
            self.track_preference("preferences.shortcuts", &select);
            if profile != self.app_preferences.shortcut_profile && self.preference_barrier() {
                self.app_preferences.shortcut_profile = profile;
                self.shortcut_resolver = crate::shortcut_profile::ShortcutResolver::new(profile);
                self.message = format!(
                    "{} shortcuts selected. Document kept.",
                    profile_name(profile)
                );
            }
            if profile == PreferenceProfile::SculptGl {
                ui.label(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C8E657D951526,
                ));
            }
            ui.separator();
            let mut custom = matches!(self.app_preferences.theme, ThemePreference::SourceCustom(_));
            let control = ui.checkbox(
                &mut custom,
                crate::ui_text::caption(crate::ui_text::CaptionKey::C724E01D8184E),
            );
            self.track_preference("preferences.palette.custom", &control);
            let mut theme = self.app_preferences.theme.clone();
            if control.changed() {
                theme = if custom {
                    ThemePreference::SourceCustom(
                        SourcePalette::from_overall([0.3, 0.3, 0.3, 1.0])
                            .expect("Literal normalized palette"),
                    )
                } else {
                    ThemePreference::LegacyDark
                };
            }
            if let ThemePreference::SourceCustom(palette) = &mut theme {
                let mut overall = palette.widget().map(|x| x as f32);
                ui.horizontal(|ui| {
                    ui.label(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::CF6C725C6B0DF,
                    ));
                    let c = ui.color_edit_button_rgba_unmultiplied(&mut overall);
                    self.track_preference("preferences.palette.overall", &c);
                    if c.changed() {
                        let _ = palette.edit_overall(overall.map(f64::from));
                    }
                });
                let mut widget = palette.widget().map(|x| x as f32);
                ui.horizontal(|ui| {
                    ui.label(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::CB66EBE1DC94C,
                    ));
                    let c = ui.color_edit_button_rgba_unmultiplied(&mut widget);
                    self.track_preference("preferences.palette.widget", &c);
                    if c.changed() {
                        let _ = palette.edit_widget(widget.map(f64::from));
                    }
                });
                let mut back = palette.background().map(|x| x as f32);
                ui.horizontal(|ui| {
                    ui.label(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C5F17684A2709,
                    ));
                    let c = ui.color_edit_button_rgb(&mut back);
                    self.track_preference("preferences.palette.back", &c);
                    if c.changed() {
                        let _ = palette.edit_background(back.map(f64::from));
                    }
                });
                let mut text = palette.text().map(|x| x as f32);
                ui.horizontal(|ui| {
                    ui.label(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C9596375FADC7,
                    ));
                    let c = ui.color_edit_button_rgba_unmultiplied(&mut text);
                    self.track_preference("preferences.palette.text", &c);
                    if c.changed() {
                        let _ = palette.edit_text(text.map(f64::from));
                    }
                });
                let mut border = palette.border();
                let c = ui.checkbox(
                    &mut border,
                    crate::ui_text::caption(crate::ui_text::CaptionKey::C49051AAF77BE),
                );
                self.track_preference("preferences.palette.border", &c);
                if c.changed() {
                    palette.set_border(border);
                }
                ui.label(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C28D42BDA850A,
                ));
            }
            if theme != self.app_preferences.theme && self.preference_barrier() {
                self.app_preferences.theme = theme;
                apply_theme(ctx, &self.app_preferences.theme);
            }
            if let Some(source) = self.startup_source.as_ref() {
                ui.separator();
                ui.label(format!(
                    "Startup shader:{} · FOV:{} · scale-center:{}",
                    source.shader, source.fov, source.scale_center
                ));
            }
            for gap in &self.startup_gaps {
                ui.label(gap);
            }
            if self.startup_models.pending().is_some() {
                let c = ui.button(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C2367774907F7,
                ));
                self.track_preference("preferences.model.cancel", &c);
                if c.clicked() {
                    self.cancel_startup_model();
                }
            }
        });
        self.preferences_window_observation = shown.map(|window| {
            let r = window.response.rect;
            [r.min.x, r.min.y, r.width(), r.height()]
        });
        self.show_app_preferences = open;
    }
    fn locale_barrier_observation(&self) -> serde_json::Value {
        serde_json::json!({"stroke_active":self.stroke.is_some(),
            "captured":self.stroke.as_ref().map(|stroke|serde_json::json!({
                "kind":stroke.kind.name(),"radius_bits":stroke.settings.radius.to_bits(),
                "strength_bits":stroke.settings.strength.to_bits(),"invert":stroke.invert,
                "pressure_bits":stroke.settings.pressure.to_bits(),"color_bits":stroke.settings.color.map(f32::to_bits)})),
            "queue":self.pending_dabs.metrics(),"document_revision":self.revision.to_string(),
            "snapshot_revision":self.snapshot_revision.to_string(),"stroke_rejected":self.stroke_rejected})
    }
    pub(super) fn source_shortcuts(&mut self, ctx: &egui::Context) {
        let focused = editor_owns_keys(ctx);
        let events = ctx.input(|i| i.events.clone());
        for event in events {
            self.modal_pointer_event(&event, focused);
            let egui::Event::Key {
                key,
                pressed,
                repeat,
                modifiers,
                ..
            } = event
            else {
                continue;
            };
            let Some(code) = keycode(key) else {
                continue;
            };
            if !source_key_phase_admitted(pressed, repeat) {
                continue;
            }
            if key == egui::Key::Escape && pressed {
                self.modal_cancel("Escape");
                if self.interrupt_source_paint_tail("Escape ended Source Paint contact") {
                    self.unprocessed_release = None;
                    self.finish_stroke_deferred();
                }
                self.paint_pick_armed = false;
                self.paint_pick_dragging = false;
                self.cancel_locked_stamp();
                self.cancel_mask_region("Escape cancelled the pending mask selection.", true);
                if !focused {
                    self.cancel_transform_drag();
                    self.transform_mode = false;
                    self.finish_stroke();
                }
                continue;
            }
            if !pressed {
                let input = KeyDelivery {
                    code,
                    pressed,
                    repeat,
                    modifiers: Modifiers {
                        control: modifiers.ctrl,
                        shift: modifiers.shift,
                        alt: modifiers.alt,
                        command: modifiers.command,
                    },
                };
                for command in self.shortcut_resolver.resolve(
                    input,
                    ShortcutContext {
                        gui_focus: false,
                        idle: false,
                        camera_rotating: false,
                    },
                ) {
                    if let Command::ModalEnd(kind) = command {
                        self.modal_end(kind);
                    }
                }
            }
            if focused {
                continue;
            }
            let context = ShortcutContext {
                gui_focus: false,
                idle: self.modal_shortcut_idle(),
                camera_rotating: self.camera_navigation_active,
            };
            let input = KeyDelivery {
                code,
                pressed,
                repeat,
                modifiers: Modifiers {
                    control: modifiers.ctrl,
                    shift: modifiers.shift,
                    alt: modifiers.alt,
                    command: modifiers.command,
                },
            };
            for command in self.shortcut_resolver.resolve(input, context) {
                if self.source_modal.state.picker
                    && matches!(command, Command::SelectBrush(_) | Command::SelectTransform)
                {
                    continue;
                }
                self.source_command(command, modifiers);
            }
        }
    }
    pub(super) fn source_keyboard_camera_frame(&mut self, ctx: &egui::Context) {
        if self.app_preferences.shortcut_profile != PreferenceProfile::SculptGl
            || !ctx.input(|input| input.focused)
            || editor_owns_keys(ctx)
        {
            self.source_keyboard_camera.clear();
            return;
        }
        if let Some((x, z)) = self.source_keyboard_camera.due(web_time::Instant::now()) {
            self.camera.source_keyboard_step(x, z);
            ctx.request_repaint();
        }
        if self.source_keyboard_camera.active() {
            ctx.request_repaint_after(SOURCE_CAMERA_KEY_INTERVAL);
        }
    }
    fn source_command(&mut self, command: Command, modifiers: egui::Modifiers) {
        if !matches!(
            command,
            Command::ModalStart(_) | Command::ModalEnd(_) | Command::SelectBrush(_) | Command::Help
        ) {
            self.modal_cancel("other shortcut action");
        }
        match command {
            Command::SelectBrush(kind) => {
                self.select_brush(kind);
            }
            Command::Undo => self.history(false),
            Command::Redo => self.history(true),
            Command::SaveProject => self.save_file(false),
            Command::OpenProject => self.import_file(),
            Command::AddFile => self.pick_source_sgl(),
            Command::ExportObj => self.save_file(true),
            Command::ClearScene => {
                self.scene_action(
                    |d| d.clear_scene().map(|_| ()),
                    "Scene cleared. Undo restores it.",
                );
            }
            Command::Duplicate => {
                if self.document.active_subtool_id().is_some() {
                    self.scene_action(
                        |d| d.duplicate_selected_subtools().map(|_| ()),
                        "Selected parts duplicated.",
                    );
                }
            }
            Command::DeleteSelection => {
                if self.document.active_subtool_id().is_some() {
                    self.scene_action(|d| d.delete_selected_subtools(), "Selected parts deleted.");
                }
            }
            Command::Isolate => {
                let hidden = self.document.solo()
                    || self
                        .document
                        .subtool_summaries()
                        .iter()
                        .any(|part| !part.visible);
                if hidden {
                    self.scene_action(|d| d.show_all_subtools(), "All parts visible.");
                } else {
                    self.scene_action(
                        |d| d.isolate_selected_subtools(),
                        "Unselected parts hidden.",
                    );
                }
            }
            Command::SelectTransform => {
                if !self.transform_mode {
                    self.toggle_transform_mode();
                }
            }
            Command::ToggleTransform => self.toggle_transform_mode(),
            Command::FrameScene => self.frame_scene(),
            Command::ViewFront | Command::ViewTop | Command::ViewLeft => {
                self.finish_stroke();
                if !self.stroke_rejected {
                    self.cancel_transform_drag();
                    self.cancel_mask_region("Camera axis changed.", true);
                    self.camera.axis_view(match command {
                        Command::ViewFront => 0,
                        Command::ViewLeft => 1,
                        _ => 2,
                    });
                }
            }
            Command::ResetView => {
                self.finish_stroke();
                if !self.stroke_rejected {
                    self.cancel_transform_drag();
                    self.cancel_mask_region("Camera reset.", true);
                    self.camera.rotation = Some(glam::Quat::IDENTITY);
                    self.camera.yaw = 0.0;
                    self.camera.pitch = 0.0;
                    self.frame_scene();
                }
            }
            Command::Wireframe => {
                self.finish_stroke();
                if !self.stroke_rejected {
                    self.cancel_mask_region("Wireframe changed.", true);
                    self.toggle_wireframe();
                }
            }
            Command::Remesh => self.start_topology(TopologyOperation::Remesh(RemeshSettings {
                resolution: self.remesh_resolution,
                block_mode: self.remesh_block_mode,
                surface_nets_mode: self.remesh_surface_nets_mode,
                allow_uv_loss: self.remesh_discard_uvs,
                ..Default::default()
            })),
            Command::ToggleNegative => {
                self.finish_stroke();
                if !self.stroke_rejected {
                    self.brush.invert = !self.brush.invert;
                    self.commit_brush_view();
                }
            }
            Command::Symmetry => {
                self.finish_stroke();
                if !self.stroke_rejected {
                    self.brush.symmetry_x = !self.brush.symmetry_x;
                    self.commit_brush_view();
                }
            }
            Command::Help => {
                self.show_help = !self.show_help;
            }
            Command::CancelContact => {
                self.cancel_locked_stamp();
                self.cancel_mask_region("Contact cancelled.", true);
            }
            Command::CancelTransform => {
                self.cancel_transform_drag();
            }
            Command::ModalStart(kind) => self.modal_start(kind, modifiers),
            Command::ModalEnd(kind) => self.modal_end(kind),
            Command::Strafe(direction, pressed) => {
                self.source_keyboard_camera
                    .set(direction, pressed, web_time::Instant::now())
            }
            Command::SnapView => self.camera.snap_axis(),
            Command::RadiusStep(_) => {
                self.message="Source brackets are not a configured radius action; use the normal radius control.".into();
            }
        }
        self.egui_ctx.request_repaint();
    }

    pub(super) fn poll_startup_model(&mut self, ctx: &egui::Context) {
        if let Some(url) = self.startup_model_pending.take()
            && let Err(e) = self.begin_startup_model(&url)
        {
            self.message = format!("Startup model failed: {e}");
            self.startup_models.last_result = Some(e);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let result =
                self.native_startup_model
                    .as_ref()
                    .and_then(|job| match job.receiver.try_recv() {
                        Ok(r) => Some((job.id, r)),
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            Some((job.id, Err("Model worker ended unexpectedly".into())))
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => None,
                    });
            if let Some((id, result)) = result {
                self.native_startup_model.take();
                let result = result.and_then(|bytes| self.load_startup_model_bytes(id, &bytes));
                if let Err(e) = result {
                    self.fail_startup_model(id, &e);
                }
            }
            if self.native_startup_model.is_some() {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }
        #[cfg(target_arch = "wasm32")]
        let _ = ctx;
    }
    fn begin_startup_model(&mut self, url: &str) -> Result<(), String> {
        self.cancel_startup_model();
        self.finish_stroke();
        if self.stroke_rejected {
            return Err("Preceding stroke rejected".into());
        }
        let options = self
            .startup_source
            .clone()
            .ok_or("Startup model needs Source profile capture")?;
        #[cfg(not(target_arch = "wasm32"))]
        let native = crate::source_sgl_import::native_byte_order();
        #[cfg(target_arch = "wasm32")]
        let native = if source_sgl_native_little() {
            obrush_core::SglByteOrder::LittleEndian
        } else {
            obrush_core::SglByteOrder::BigEndian
        };
        let mut request = self
            .startup_models
            .begin(self.revision, url, options, native)?;
        if matches!(
            request
                .filename
                .rsplit_once('.')
                .map(|(_, ext)| ext.to_ascii_lowercase())
                .as_deref(),
            Some("sgl" | "obj" | "ply" | "stl")
        ) {
            let old = self.source_sgl_import.options;
            self.source_sgl_import.options.scale_center = request.options.scale_center;
            self.source_sgl_import.options.vertex_srgb = true;
            let sgl = match self
                .source_sgl_import
                .begin_captured(request.revision, native)
            {
                Ok(r) => r,
                Err(e) => {
                    self.source_sgl_import.options = old;
                    self.startup_models.cancel();
                    return Err(e);
                }
            };
            self.startup_models.attach_sgl(request.id, sgl.id)?;
            request.sgl_request_id = Some(sgl.id);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.native_startup_model = Some(crate::startup_model::NativeModelJob::start(
                &request,
                self.egui_ctx.clone(),
            ));
        }
        #[cfg(target_arch = "wasm32")]
        {
            start_model_read(request.id, &request.url);
        }
        self.message = format!("Reading startup model: {}", request.filename);
        Ok(())
    }
    pub fn load_startup_model_bytes(&mut self, id: u32, bytes: &[u8]) -> Result<(), String> {
        let request = self.startup_models.accept(id, self.revision)?;
        let result: Result<(), String> = (|| {
            if bytes.len() > MAX_MODEL_BYTES {
                return Err("Startup model exceeds128MiB".into());
            }
            if let Some(sgl_id) = request.sgl_request_id {
                self.load_source_sgl_for_request(
                    sgl_id,
                    &request.filename,
                    bytes,
                    request.native_order,
                )?;
            } else {
                // Complete shared-core parse before the normal production barrier.
                let prepared = crate::scene_import::prepare(
                    &request.filename,
                    bytes,
                    true,
                    &self.viewport_style,
                )?;
                self.finish_stroke();
                if self.stroke_rejected {
                    return Err("Preceding stroke rejected".into());
                }
                if self.revision != request.revision {
                    return Err("Document changed before model installation".into());
                }
                self.last_import_report =
                    crate::scene_import::install(&mut self.document, prepared)?;
                self.cancel_topology();
                self.frame_scene();
                self.changed();
            }
            Ok(())
        })();
        match &result {
            Ok(()) => {
                self.startup_models.last_result = Some(format!("Installed {}", request.filename));
            }
            Err(error) => {
                if let Some(sgl_id) = request.sgl_request_id {
                    self.source_sgl_import.cancel(sgl_id);
                }
                self.startup_models.last_result = Some(error.clone());
                self.message = format!("Startup model failed: {error}");
            }
        }
        self.egui_ctx.request_repaint();
        result
    }
    pub fn fail_startup_model(&mut self, id: u32, error: &str) {
        if self.startup_models.pending().is_none_or(|r| r.id != id) {
            return;
        }
        self.cancel_startup_model();
        self.startup_models.last_result = Some(error.chars().take(4096).collect());
        self.message = format!(
            "Startup model failed: {}",
            error.chars().take(4096).collect::<String>()
        );
        self.egui_ctx.request_repaint();
    }
    pub(super) fn cancel_startup_model(&mut self) {
        self.startup_model_pending = None;
        if let Some(request) = self.startup_models.cancel() {
            if let Some(id) = request.sgl_request_id {
                self.source_sgl_import.cancel(id);
            }
            #[cfg(target_arch = "wasm32")]
            cancel_model_read(request.id);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(job) = self.native_startup_model.take() {
            job.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn held_camera_keys_tick_once_after_interval_and_stop_on_release() {
        let start = web_time::Instant::now();
        let mut camera = SourceKeyboardCamera::default();
        camera.set(Direction::Left, true, start);
        assert_eq!(camera.due(start), None);
        assert_eq!(
            camera.due(start + SOURCE_CAMERA_KEY_INTERVAL),
            Some((-1, 0))
        );
        // A late browser frame never replays all missed timer intervals.
        assert_eq!(
            camera.due(start + SOURCE_CAMERA_KEY_INTERVAL * 20),
            Some((-1, 0))
        );
        assert_eq!(camera.due(start + SOURCE_CAMERA_KEY_INTERVAL * 20), None);
        camera.set(Direction::Up, true, start + SOURCE_CAMERA_KEY_INTERVAL * 20);
        assert_eq!(
            camera.due(start + SOURCE_CAMERA_KEY_INTERVAL * 21),
            Some((-1, -1))
        );
        camera.set(
            Direction::Left,
            false,
            start + SOURCE_CAMERA_KEY_INTERVAL * 21,
        );
        assert_eq!(camera.move_x, 0);
        camera.set(
            Direction::Up,
            false,
            start + SOURCE_CAMERA_KEY_INTERVAL * 21,
        );
        assert!(!camera.active());
        assert!(camera.next_tick.is_none());
    }
    #[test]
    fn original_shader_data_roles_do_not_become_view_modes() {
        assert_eq!(shading(0), Some((ShadingMode::Pbr, false)));
        assert_eq!(shading(3), Some((ShadingMode::Flat, true)));
        assert_eq!(shading(5), Some((ShadingMode::Matcap, false)));
        for v in 6..=12 {
            assert_eq!(shading(v), None);
        }
    }
    #[test]
    fn default_palette_keeps_literal_legacy_tokens() {
        let ctx = egui::Context::default();
        apply_theme(&ctx, &ThemePreference::LegacyDark);
        assert!(
            ctx.data(|d| d.get_temp::<PaletteTokens>(egui::Id::new(PALETTE_KEY)))
                .is_none()
        );
        assert_eq!(ctx.style().visuals.panel_fill, PANEL);
    }
    #[test]
    fn custom_palette_maps_host_roles_without_touching_scene_style() {
        let ctx = egui::Context::default();
        let palette = SourcePalette::from_overall([0.2, 0.4, 0.6, 0.75]).unwrap();
        let style = ViewportStyle::default();
        apply_theme(&ctx, &ThemePreference::SourceCustom(palette.clone()));
        let tokens = ctx
            .data(|d| d.get_temp::<PaletteTokens>(egui::Id::new(PALETTE_KEY)))
            .unwrap();
        assert_eq!(tokens.panel, color3([0.1, 0.2, 0.3]));
        assert_eq!(tokens.top, color3([0.05, 0.1, 0.15]));
        assert_eq!(
            ctx.style().visuals.override_text_color,
            Some(color4(palette.text()))
        );
        assert_eq!(style.color, [0.66, 0.68, 0.72]);
        assert_eq!(style.matcap_index, 3);
    }
    #[test]
    fn palette_reset_clears_effective_custom_roles() {
        let ctx = egui::Context::default();
        apply_theme(
            &ctx,
            &ThemePreference::SourceCustom(
                SourcePalette::from_overall([0.3, 0.3, 0.3, 1.0]).unwrap(),
            ),
        );
        apply_theme(&ctx, &ThemePreference::LegacyDark);
        assert!(
            ctx.data(|d| d.get_temp::<PaletteTokens>(egui::Id::new(PALETTE_KEY)))
                .is_none()
        );
    }
    #[test]
    fn actual_text_editor_owns_keys_but_a_focused_button_does_not() {
        let ctx = egui::Context::default();
        let id = egui::Id::new("source_key_text_editor");
        let mut text = String::new();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add(egui::TextEdit::singleline(&mut text).id(id))
                    .request_focus();
            });
        });
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            assert!(editor_owns_keys(ctx));
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.button("Tool tile").request_focus();
            });
        });
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            assert!(!editor_owns_keys(ctx));
        });
    }
    #[test]
    fn host_keycodes_preserve_phase_domain() {
        assert_eq!(keycode(egui::Key::F), Some(70));
        assert_eq!(keycode(egui::Key::Delete), Some(46));
        assert_eq!(keycode(egui::Key::OpenBracket), Some(219));
    }
    #[test]
    fn source_host_phase_admission_precedes_dispatch_without_filtering_keyup() {
        let resolver = crate::shortcut_profile::ShortcutResolver::new(PreferenceProfile::SculptGl);
        for (pressed, repeat, admitted, expected) in [
            (false, false, true, vec![Command::ViewFront]),
            (false, true, true, vec![Command::ViewFront]),
            (true, false, true, Vec::new()),
            (true, true, false, Vec::new()),
        ] {
            assert_eq!(source_key_phase_admitted(pressed, repeat), admitted);
            let actual = if source_key_phase_admitted(pressed, repeat) {
                resolver.resolve(
                    KeyDelivery {
                        code: 70,
                        pressed,
                        repeat,
                        modifiers: Modifiers::default(),
                    },
                    ShortcutContext::default(),
                )
            } else {
                Vec::new()
            };
            assert_eq!(actual, expected);
        }
    }
    #[test]
    fn source_camera_release_does_not_trigger_press_action() {
        let resolver = crate::shortcut_profile::ShortcutResolver::new(PreferenceProfile::SculptGl);
        let c = ShortcutContext::default();
        assert!(
            resolver
                .resolve(
                    KeyDelivery {
                        code: 70,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::default()
                    },
                    c
                )
                .is_empty()
        );
        assert_eq!(
            resolver.resolve(
                KeyDelivery {
                    code: 70,
                    pressed: false,
                    repeat: false,
                    modifiers: Modifiers::default()
                },
                c
            ),
            vec![Command::ViewFront]
        );
    }
    #[test]
    fn all_twenty_one_request_fields_remain_visible() {
        let value = source_metrics(
            &SourceStartupSettings::default(),
            Locale::English,
            &crate::shortcut_profile::ShortcutResolver::new(PreferenceProfile::SculptGl),
        );
        assert_eq!(value.as_object().unwrap().len(), 21);
        assert_eq!(value["grid"], true);
        assert_eq!(value["shader"], 5);
        assert!(value["model_url"].is_null());
    }
}

#[cfg(test)]
mod locale_observation_contract {
    use super::*;
    #[test]
    fn normal_response_observation_preserves_full_u64_id_and_rect() {
        let ctx = egui::Context::default();
        let mut captured = None;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let response = ui.button("Locale observation literal");
                let id = response.id.value();
                let r = response.rect;
                captured = Some((
                    id,
                    [r.min.x, r.min.y, r.width(), r.height()],
                    preference_response_observation(&response),
                ));
            });
        });
        let (id, rect, row) = captured.unwrap();
        let text = row["id_hex"].as_str().unwrap();
        assert_eq!(text.len(), 16);
        assert_eq!(u64::from_str_radix(text, 16).unwrap(), id);
        assert_eq!(row["rect_xywh"], serde_json::json!(rect));
        assert_eq!(row["has_focus"], false);
        assert_eq!(row["enabled"], true);
    }
}
