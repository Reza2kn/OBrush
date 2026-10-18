//! Stable phase-aware shortcut data/resolver. No Egui/window/timer/IO/history.
//! A future host adapter must preserve delivered event modifiers, frame folding
//! and action owners. Merely resolving commands is not reference parity.
use crate::app_preferences::PreferenceProfile;
use crate::startup_options::integer_prefix;
use obrush_core::BrushKind;
use std::collections::BTreeMap;

pub const MAX_SHORTCUT_BYTES: usize = 4096;
pub const MAX_SHORTCUT_ENTRIES: usize = 64;
pub const MAX_LEGACY_KEYCODE: u32 = 65535;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modal {
    Radius,
    Intensity,
    Picker,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    SelectBrush(BrushKind),
    SelectTransform,
    ToggleTransform,
    Undo,
    Redo,
    SaveProject,
    OpenProject,
    AddFile,
    ExportObj,
    ClearScene,
    Duplicate,
    Isolate,
    DeleteSelection,
    FrameScene,
    ViewFront,
    ViewTop,
    ViewLeft,
    ResetView,
    SnapView,
    Wireframe,
    Remesh,
    ToggleNegative,
    Symmetry,
    Help,
    RadiusStep(bool),
    ModalStart(Modal),
    ModalEnd(Modal),
    Strafe(Direction, bool),
    CancelContact,
    CancelTransform,
}
impl Command {
    /// Stable protocol identities; translated labels never choose commands.
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::SelectBrush(kind) => match kind {
                BrushKind::Draw => "brush.draw",
                BrushKind::Clay => "brush.clay",
                BrushKind::Inflate => "brush.inflate",
                BrushKind::Smooth => "brush.smooth",
                BrushKind::Flatten => "brush.flatten",
                BrushKind::Crease => "brush.crease",
                BrushKind::Move => "brush.move",
                BrushKind::Mask => "brush.mask",
                BrushKind::Paint => "brush.paint",
                BrushKind::Morph => "brush.morph",
                BrushKind::Pinch => "brush.pinch",
                BrushKind::Twist => "brush.twist",
                BrushKind::LocalScale => "brush.local_scale",
                BrushKind::Drag => "brush.drag",
            },
            Self::SelectTransform => "transform.select",
            Self::ToggleTransform => "transform.toggle",
            Self::Undo => "history.undo",
            Self::Redo => "history.redo",
            Self::SaveProject => "file.save_project",
            Self::OpenProject => "file.open_project",
            Self::AddFile => "file.add",
            Self::ExportObj => "file.export_obj",
            Self::ClearScene => "scene.clear",
            Self::Duplicate => "scene.duplicate",
            Self::Isolate => "scene.isolate",
            Self::DeleteSelection => "scene.delete",
            Self::FrameScene => "camera.frame",
            Self::ViewFront => "camera.front",
            Self::ViewTop => "camera.top",
            Self::ViewLeft => "camera.left",
            Self::ResetView => "camera.reset",
            Self::SnapView => "camera.snap",
            Self::Wireframe => "viewport.wireframe",
            Self::Remesh => "object.remesh",
            Self::ToggleNegative => "brush.negative",
            Self::Symmetry => "brush.symmetry",
            Self::Help => "app.help",
            Self::RadiusStep(true) => "brush.radius.decrease",
            Self::RadiusStep(false) => "brush.radius.increase",
            Self::ModalStart(Modal::Radius) => "modal.radius.start",
            Self::ModalEnd(Modal::Radius) => "modal.radius.end",
            Self::ModalStart(Modal::Intensity) => "modal.intensity.start",
            Self::ModalEnd(Modal::Intensity) => "modal.intensity.end",
            Self::ModalStart(Modal::Picker) => "modal.picker.start",
            Self::ModalEnd(Modal::Picker) => "modal.picker.end",
            Self::Strafe(Direction::Left, true) => "camera.left.start",
            Self::Strafe(Direction::Left, false) => "camera.left.end",
            Self::Strafe(Direction::Right, true) => "camera.right.start",
            Self::Strafe(Direction::Right, false) => "camera.right.end",
            Self::Strafe(Direction::Up, true) => "camera.up.start",
            Self::Strafe(Direction::Up, false) => "camera.up.end",
            Self::Strafe(Direction::Down, true) => "camera.down.start",
            Self::Strafe(Direction::Down, false) => "camera.down.end",
            Self::CancelContact => "contact.cancel",
            Self::CancelTransform => "transform.cancel",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceAction {
    Select(BrushKind),
    Transform,
    Intensity,
    Radius,
    Negative,
    Picker,
    Delete,
    Front,
    Top,
    Left,
    Reset,
    Strafe(Direction),
    Wireframe,
    Remesh,
}

impl SourceAction {
    fn name(value: &str) -> Option<Self> {
        match value {
            "BRUSH" => Some(Self::Select(BrushKind::Draw)),
            "INFLATE" => Some(Self::Select(BrushKind::Inflate)),
            "TWIST" => Some(Self::Select(BrushKind::Twist)),
            "SMOOTH" => Some(Self::Select(BrushKind::Smooth)),
            "FLATTEN" => Some(Self::Select(BrushKind::Flatten)),
            "PINCH" => Some(Self::Select(BrushKind::Pinch)),
            "CREASE" => Some(Self::Select(BrushKind::Crease)),
            "DRAG" => Some(Self::Select(BrushKind::Drag)),
            "PAINT" => Some(Self::Select(BrushKind::Paint)),
            "MOVE" => Some(Self::Select(BrushKind::Move)),
            "MASKING" => Some(Self::Select(BrushKind::Mask)),
            "LOCALSCALE" => Some(Self::Select(BrushKind::LocalScale)),
            "TRANSFORM" => Some(Self::Transform),
            "INTENSITY" => Some(Self::Intensity),
            "RADIUS" => Some(Self::Radius),
            "NEGATIVE" => Some(Self::Negative),
            "PICKER" => Some(Self::Picker),
            "DELETE" => Some(Self::Delete),
            "CAMERA_FRONT" => Some(Self::Front),
            "CAMERA_TOP" => Some(Self::Top),
            "CAMERA_LEFT" => Some(Self::Left),
            "CAMERA_RESET" => Some(Self::Reset),
            "STRIFE_LEFT" => Some(Self::Strafe(Direction::Left)),
            "STRIFE_RIGHT" => Some(Self::Strafe(Direction::Right)),
            "STRIFE_UP" => Some(Self::Strafe(Direction::Up)),
            "STRIFE_DOWN" => Some(Self::Strafe(Direction::Down)),
            "WIREFRAME" => Some(Self::Wireframe),
            "REMESH" => Some(Self::Remesh),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub control: bool,
    pub shift: bool,
    pub alt: bool,
    pub command: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyDelivery {
    pub code: u16,
    pub pressed: bool,
    pub repeat: bool,
    pub modifiers: Modifiers,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShortcutContext {
    pub gui_focus: bool,
    pub idle: bool,
    pub camera_rotating: bool,
}
impl Default for ShortcutContext {
    fn default() -> Self {
        Self {
            gui_focus: false,
            idle: true,
            camera_rotating: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShortcutResolver {
    profile: PreferenceProfile,
    source: BTreeMap<u16, SourceAction>,
}

impl ShortcutResolver {
    pub fn new(profile: PreferenceProfile) -> Self {
        let mut source = BTreeMap::new();
        for (code, name) in [
            (48, "MOVE"),
            (49, "BRUSH"),
            (50, "INFLATE"),
            (51, "TWIST"),
            (52, "SMOOTH"),
            (53, "FLATTEN"),
            (54, "PINCH"),
            (55, "CREASE"),
            (56, "DRAG"),
            (57, "PAINT"),
            (69, "TRANSFORM"),
            (67, "INTENSITY"),
            (88, "RADIUS"),
            (78, "NEGATIVE"),
            (83, "PICKER"),
            (46, "DELETE"),
            (70, "CAMERA_FRONT"),
            (84, "CAMERA_TOP"),
            (76, "CAMERA_LEFT"),
            (32, "CAMERA_RESET"),
            (37, "STRIFE_LEFT"),
            (39, "STRIFE_RIGHT"),
            (38, "STRIFE_UP"),
            (40, "STRIFE_DOWN"),
            (87, "WIREFRAME"),
            (82, "REMESH"),
        ] {
            // Literal registered names; no runtime input can reach this expect.
            source.insert(
                code,
                SourceAction::name(name).expect("Registered source action"),
            );
        }
        Self { profile, source }
    }

    /// Observational hints for implemented command owners only. Does not dispatch.
    /// The returned event can be checked against the same production resolver.
    pub fn implemented_command_hint(&self, command: Command) -> Option<String> {
        let event = self.implemented_hint_event(command)?;
        let key = match event.code {
            27 => "Escape".to_owned(),
            32 => "Space".to_owned(),
            46 => "Delete".to_owned(),
            219 => "[".to_owned(),
            221 => "]".to_owned(),
            48..=57 | 65..=90 => char::from_u32(u32::from(event.code))?.to_string(),
            _ => return None,
        };
        let prefix = match (
            event.modifiers.control,
            event.modifiers.shift,
            event.modifiers.alt,
        ) {
            (true, true, false) => "Cmd/Ctrl+Shift+",
            (true, false, true) => "Cmd/Ctrl+Alt+",
            (true, false, false) => "Cmd/Ctrl+",
            _ => "",
        };
        Some(format!(
            "{}{} ({})",
            prefix,
            key,
            if event.pressed { "press" } else { "release" }
        ))
    }
    pub fn implemented_hint_event(&self, command: Command) -> Option<KeyDelivery> {
        if matches!(
            command,
            Command::ModalStart(_) | Command::ModalEnd(_) | Command::SnapView
        ) {
            return None;
        }
        // These are exactly the keycodes admitted by the actual Egui host adapter.
        let codes = [
            27, 32, 46, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 65, 66, 67, 68, 69, 70, 71, 72, 73,
            37, 38, 39, 40, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90,
            219, 221,
        ];
        let modifiers = [
            Modifiers::default(),
            Modifiers {
                control: true,
                ..Default::default()
            },
            Modifiers {
                control: true,
                shift: true,
                ..Default::default()
            },
            Modifiers {
                control: true,
                alt: true,
                ..Default::default()
            },
        ];
        for modifier in modifiers {
            for code in codes {
                for pressed in [true, false] {
                    let event = KeyDelivery {
                        code,
                        pressed,
                        repeat: false,
                        modifiers: modifier,
                    };
                    if self
                        .resolve(event, ShortcutContext::default())
                        .contains(&command)
                    {
                        return Some(event);
                    }
                }
            }
        }
        None
    }
    pub const fn profile(&self) -> PreferenceProfile {
        self.profile
    }
    pub fn source_binding_count(&self) -> usize {
        self.source.len()
    }
    pub fn brush_labels(&self, kind: BrushKind) -> String {
        self.source
            .iter()
            .filter_map(|(code, action)| match action {
                SourceAction::Select(target) if *target == kind => char::from_u32(u32::from(*code))
                    .filter(|c| c.is_ascii_alphanumeric())
                    .map(|c| c.to_string()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("/")
    }

    /// Validate a candidate before replacing bindings: invalid later rows cannot
    /// leave a partially updated map. Existing default aliases remain.
    pub fn source_overrides(&mut self, text: &str) -> Result<(), &'static str> {
        if self.profile != PreferenceProfile::SculptGl {
            return Err("Source overrides require source profile");
        }
        if text.len() > MAX_SHORTCUT_BYTES {
            return Err("Shortcut override bytes exceed limit");
        }
        let mut candidate = self.source.clone();
        for (count, row) in text.split(',').enumerate() {
            if count >= MAX_SHORTCUT_ENTRIES {
                return Err("Shortcut override rows exceed limit");
            }
            let mut pair = row.split(':');
            let (Some(name), Some(key)) = (pair.next(), pair.next()) else {
                continue;
            };
            let Some(action) = SourceAction::name(&name.to_uppercase()) else {
                continue;
            };
            let upper = key.to_uppercase();
            let parsed = integer_prefix(&upper)?;
            let code = if let Some(n) = parsed.filter(|v| *v >= 10) {
                if n > i64::from(MAX_LEGACY_KEYCODE) {
                    return Err("Shortcut keycode exceeds supported range");
                }
                n as u16
            } else {
                // JS charCodeAt uses the first UTF16 code unit, not a Unicode
                // scalar. Actual platform delivery for exotic keys is unverified.
                upper.encode_utf16().next().ok_or("Shortcut key is empty")?
            };
            candidate.insert(code, action);
        }
        self.source = candidate;
        Ok(())
    }

    /// Bounded command batch (at most two). No timer is created; repeat/focus/
    /// keyup coalescing and temporary tools need the future normal host adapter.
    pub fn resolve(&self, event: KeyDelivery, context: ShortcutContext) -> Vec<Command> {
        match self.profile {
            PreferenceProfile::OBrush => self.legacy(event, context),
            PreferenceProfile::SculptGl => self.reference(event, context),
        }
    }

    fn legacy(&self, e: KeyDelivery, c: ShortcutContext) -> Vec<Command> {
        if !e.pressed {
            return Vec::new();
        }
        if e.code == 27 {
            return if c.gui_focus {
                vec![Command::CancelContact]
            } else {
                vec![Command::CancelContact, Command::CancelTransform]
            };
        }
        if e.modifiers.command {
            return match e.code {
                90 if e.modifiers.shift => vec![Command::Redo],
                90 => vec![Command::Undo],
                89 => vec![Command::Redo],
                83 => vec![Command::SaveProject],
                79 => vec![Command::OpenProject],
                _ => Vec::new(),
            };
        }
        if c.gui_focus {
            return Vec::new();
        }
        let code = digit_alias(e.code);
        let brush = match code {
            49 => Some(BrushKind::Draw),
            50 => Some(BrushKind::Clay),
            51 => Some(BrushKind::Inflate),
            52 => Some(BrushKind::Smooth),
            53 => Some(BrushKind::Flatten),
            54 => Some(BrushKind::Crease),
            55 => Some(BrushKind::Move),
            56 => Some(BrushKind::Mask),
            57 => Some(BrushKind::Paint),
            48 => Some(BrushKind::Morph),
            80 => Some(BrushKind::Pinch),
            74 => Some(BrushKind::Twist),
            76 => Some(BrushKind::LocalScale),
            71 => Some(BrushKind::Drag),
            _ => None,
        };
        if let Some(kind) = brush {
            return vec![Command::SelectBrush(kind)];
        }
        match code {
            70 => vec![Command::FrameScene],
            88 => vec![Command::Symmetry],
            87 => vec![Command::Wireframe],
            72 => vec![Command::Help],
            84 => vec![Command::ToggleTransform],
            219 => vec![Command::RadiusStep(true)],
            221 => vec![Command::RadiusStep(false)],
            _ => Vec::new(),
        }
    }

    fn reference(&self, e: KeyDelivery, c: ShortcutContext) -> Vec<Command> {
        let m = e.modifiers;
        // Fixed controllers precede the configurable tool/camera map.
        if e.pressed {
            if m.control {
                let fixed = match e.code {
                    78 if m.alt => Some(Command::ClearScene),
                    79 | 73 => Some(Command::AddFile),
                    69 => Some(Command::ExportObj),
                    68 => Some(Command::Duplicate),
                    90 => Some(Command::Undo),
                    89 => Some(Command::Redo),
                    _ => None,
                };
                if let Some(command) = fixed {
                    return vec![command];
                }
            }
            if e.code == 73 {
                return vec![Command::Isolate];
            }
        }
        let action = self.source.get(&digit_alias(e.code)).copied();
        let mut commands = Vec::with_capacity(2);
        if e.pressed && m.shift && c.camera_rotating && !c.gui_focus {
            commands.push(Command::SnapView);
        }
        let mapped = match action {
            Some(SourceAction::Strafe(direction)) if !c.gui_focus => {
                Some(Command::Strafe(direction, e.pressed))
            }
            Some(SourceAction::Front) if !e.pressed && !c.gui_focus => Some(Command::ViewFront),
            Some(SourceAction::Top) if !e.pressed && !c.gui_focus => Some(Command::ViewTop),
            Some(SourceAction::Left) if !e.pressed && !c.gui_focus => Some(Command::ViewLeft),
            Some(SourceAction::Reset) if !e.pressed && !c.gui_focus => Some(Command::ResetView),
            Some(SourceAction::Wireframe) if !e.pressed && !m.control => Some(Command::Wireframe),
            Some(SourceAction::Remesh) if !e.pressed => Some(Command::Remesh),
            Some(SourceAction::Radius) => modal(e.pressed, c.idle && !m.alt, Modal::Radius),
            Some(SourceAction::Intensity) => modal(e.pressed, c.idle && !m.alt, Modal::Intensity),
            Some(SourceAction::Picker) => modal(e.pressed, c.idle && !m.alt, Modal::Picker),
            Some(SourceAction::Select(kind)) if e.pressed && c.idle && !m.alt => {
                Some(Command::SelectBrush(kind))
            }
            Some(SourceAction::Transform) if e.pressed && c.idle && !m.alt => {
                Some(Command::SelectTransform)
            }
            Some(SourceAction::Negative) if e.pressed && c.idle && !m.alt => {
                Some(Command::ToggleNegative)
            }
            Some(SourceAction::Delete) if e.pressed && c.idle && !m.alt => {
                Some(Command::DeleteSelection)
            }
            _ => None,
        };
        if let Some(command) = mapped {
            commands.push(command);
        }
        commands
    }
}

fn digit_alias(code: u16) -> u16 {
    if (96..=105).contains(&code) {
        code - 48
    } else {
        code
    }
}
fn modal(pressed: bool, idle: bool, kind: Modal) -> Option<Command> {
    if !pressed {
        Some(Command::ModalEnd(kind))
    } else if idle {
        Some(Command::ModalStart(kind))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key(code: u16, pressed: bool) -> KeyDelivery {
        KeyDelivery {
            code,
            pressed,
            repeat: false,
            modifiers: Modifiers::default(),
        }
    }
    fn source() -> ShortcutResolver {
        ShortcutResolver::new(PreferenceProfile::SculptGl)
    }
    #[test]
    fn source_has_twenty_six_defaults_and_ten_numpad_aliases() {
        let r = source();
        assert_eq!(r.source_binding_count(), 26);
        for digit in 0..10 {
            assert_eq!(
                r.resolve(key(48 + digit, true), ShortcutContext::default()),
                r.resolve(key(96 + digit, true), ShortcutContext::default())
            );
        }
    }
    #[test]
    fn profiles_intentionally_distinguish_conflicting_keys() {
        let legacy = ShortcutResolver::new(PreferenceProfile::OBrush);
        let source = source();
        let c = ShortcutContext::default();
        assert_eq!(
            legacy.resolve(key(50, true), c),
            vec![Command::SelectBrush(BrushKind::Clay)]
        );
        assert_eq!(
            source.resolve(key(50, true), c),
            vec![Command::SelectBrush(BrushKind::Inflate)]
        );
        assert_eq!(legacy.resolve(key(88, true), c), vec![Command::Symmetry]);
        assert_eq!(
            source.resolve(key(88, true), c),
            vec![Command::ModalStart(Modal::Radius)]
        );
    }
    #[test]
    fn legacy_roster_order_is_exact_and_all_fourteen_selectors_present() {
        let r = ShortcutResolver::new(PreferenceProfile::OBrush);
        for (code, kind) in [49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 80, 74, 76, 71]
            .into_iter()
            .zip(BrushKind::ALL)
        {
            assert_eq!(
                r.resolve(key(code, true), ShortcutContext::default()),
                vec![Command::SelectBrush(kind)]
            );
        }
    }
    #[test]
    fn legacy_file_history_priority_uses_delivered_command_modifiers() {
        let r = ShortcutResolver::new(PreferenceProfile::OBrush);
        let c = ShortcutContext {
            gui_focus: true,
            ..Default::default()
        };
        let mut e = key(90, true);
        e.modifiers.command = true;
        e.modifiers.shift = true;
        assert_eq!(r.resolve(e, c), vec![Command::Redo]);
        e.modifiers.shift = false;
        assert_eq!(r.resolve(e, c), vec![Command::Undo]);
        e.code = 79;
        assert_eq!(r.resolve(e, c), vec![Command::OpenProject]);
    }
    #[test]
    fn seven_fixed_source_rows_have_actual_action_identities() {
        let r = source();
        let c = ShortcutContext::default();
        for (code, alt, expected) in [
            (90, false, Command::Undo),
            (89, false, Command::Redo),
            (78, true, Command::ClearScene),
            (79, false, Command::AddFile),
            (73, false, Command::AddFile),
            (69, false, Command::ExportObj),
            (68, false, Command::Duplicate),
        ] {
            let mut e = key(code, true);
            e.modifiers.control = true;
            e.modifiers.alt = alt;
            assert_eq!(r.resolve(e, c), vec![expected]);
        }
        assert_eq!(r.resolve(key(73, true), c), vec![Command::Isolate]);
        assert_eq!(Command::AddFile.stable_id(), "file.add");
        assert_ne!(
            Command::SelectTransform.stable_id(),
            Command::ToggleTransform.stable_id()
        );
    }
    #[test]
    fn source_fixed_ctrl_o_i_are_add_aliases_and_meta_is_not_ctrl() {
        let r = source();
        let c = ShortcutContext {
            gui_focus: true,
            ..Default::default()
        };
        for code in [79, 73] {
            let mut e = key(code, true);
            e.modifiers.control = true;
            assert_eq!(r.resolve(e, c), vec![Command::AddFile]);
        }
        let mut e = key(79, true);
        e.modifiers.command = true;
        assert!(r.resolve(e, c).is_empty());
    }
    #[test]
    fn camera_wire_and_remesh_use_registered_release_phase() {
        let r = source();
        let c = ShortcutContext::default();
        for (code, expected) in [
            (70, Command::ViewFront),
            (84, Command::ViewTop),
            (76, Command::ViewLeft),
            (32, Command::ResetView),
            (87, Command::Wireframe),
            (82, Command::Remesh),
        ] {
            assert!(r.resolve(key(code, true), c).is_empty());
            assert_eq!(r.resolve(key(code, false), c), vec![expected]);
        }
    }
    #[test]
    fn modal_releases_are_resolved_even_with_focus_or_active_contact() {
        let r = source();
        let c = ShortcutContext {
            gui_focus: true,
            idle: false,
            camera_rotating: false,
        };
        assert!(r.resolve(key(88, true), c).is_empty());
        assert_eq!(
            r.resolve(key(88, false), c),
            vec![Command::ModalEnd(Modal::Radius)]
        );
        assert_eq!(
            r.resolve(key(83, false), c),
            vec![Command::ModalEnd(Modal::Picker)]
        );
    }
    #[test]
    fn held_arrows_and_shift_camera_snap_are_separate_bounded_commands() {
        let r = source();
        let mut e = key(37, true);
        e.modifiers.shift = true;
        let c = ShortcutContext {
            camera_rotating: true,
            ..Default::default()
        };
        assert_eq!(
            r.resolve(e, c),
            vec![Command::SnapView, Command::Strafe(Direction::Left, true)]
        );
        assert_eq!(
            r.resolve(key(37, false), c),
            vec![Command::Strafe(Direction::Left, false)]
        );
    }
    #[test]
    fn overrides_add_alias_and_enable_two_unbound_tool_targets() {
        let mut r = source();
        r.source_overrides("masking:A,localscale:B,brush:K")
            .unwrap();
        let c = ShortcutContext::default();
        assert_eq!(r.source_binding_count(), 29);
        assert_eq!(
            r.resolve(key(65, true), c),
            vec![Command::SelectBrush(BrushKind::Mask)]
        );
        assert_eq!(
            r.resolve(key(66, true), c),
            vec![Command::SelectBrush(BrushKind::LocalScale)]
        );
        assert_eq!(r.resolve(key(75, true), c), r.resolve(key(49, true), c));
    }
    #[test]
    fn override_numeric_codes_first_char_last_collision_and_unknown_names() {
        let mut r = source();
        r.source_overrides("brush:65junk,inflATE:65,unknown:Z,move:9,paint:AB:ignored")
            .unwrap();
        let c = ShortcutContext::default();
        assert_eq!(
            r.resolve(key(65, true), c),
            vec![Command::SelectBrush(BrushKind::Paint)]
        );
        assert_eq!(
            r.resolve(key(57, true), c),
            vec![Command::SelectBrush(BrushKind::Move)]
        );
        assert!(r.resolve(key(90, true), c).is_empty());
    }
    #[test]
    fn invalid_later_override_is_atomic_and_count_byte_limits_are_enforced() {
        let mut r = source();
        let original = r.clone();
        assert!(r.source_overrides("brush:K,paint:999999").is_err());
        assert_eq!(r, original);
        assert!(
            r.source_overrides(&"x".repeat(MAX_SHORTCUT_BYTES + 1))
                .is_err()
        );
        assert!(
            r.source_overrides(&vec!["brush:A"; MAX_SHORTCUT_ENTRIES + 1].join(","))
                .is_err()
        );
        assert_eq!(r, original);
    }
    #[test]
    fn bracket_steps_and_source_alt_guard_have_explicit_policy() {
        let l = ShortcutResolver::new(PreferenceProfile::OBrush);
        let c = ShortcutContext::default();
        assert_eq!(
            l.resolve(key(219, true), c),
            vec![Command::RadiusStep(true)]
        );
        assert_eq!(
            l.resolve(key(221, true), c),
            vec![Command::RadiusStep(false)]
        );
        let r = source();
        let mut e = key(49, true);
        e.modifiers.alt = true;
        assert!(r.resolve(e, c).is_empty());
        e.code = 88;
        assert!(r.resolve(e, c).is_empty());
    }
    #[test]
    fn repeated_delivery_is_not_silently_discarded_and_no_timer_is_created() {
        let r = source();
        let mut e = key(49, true);
        e.repeat = true;
        assert_eq!(
            r.resolve(e, ShortcutContext::default()),
            vec![Command::SelectBrush(BrushKind::Draw)]
        );
    }
}

#[cfg(test)]
mod display_binding_tests {
    use super::*;
    #[test]
    fn source_tiles_show_actual_overrides_and_unbound_tools() {
        let mut map = ShortcutResolver::new(PreferenceProfile::SculptGl);
        assert_eq!(map.brush_labels(BrushKind::Inflate), "2");
        assert_eq!(map.brush_labels(BrushKind::Clay), "");
        map.source_overrides("BRUSH:A").unwrap();
        assert_eq!(map.brush_labels(BrushKind::Draw), "1/A");
        assert_eq!(
            map.resolve(
                KeyDelivery {
                    code: 65,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::default()
                },
                ShortcutContext::default()
            ),
            vec![Command::SelectBrush(BrushKind::Draw)]
        );
    }
}
