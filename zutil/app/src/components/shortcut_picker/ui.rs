use eframe::egui;
use design::numbers::{SHORTCUT_PICKER_COMBO_SIZE, SHORTCUT_PICKER_INSTRUCTION_SIZE};

pub struct ShortcutPickerState {
    /// e.g. ("main_nav", "Templates")
    pub owner_kind: &'static str,
    pub owner_id: String,
    pub owner_label: String,
    pub combo: Option<String>,
    pub error: Option<String>,
}

pub enum ShortcutPickerAction {
    None,
    Save { owner_kind: &'static str, owner_id: String, combo: String },
    Cancel,
}

pub fn shortcut_picker_ui(
    ui: &mut egui::Ui,
    state: &mut ShortcutPickerState,
) -> ShortcutPickerAction {
    let mut action = ShortcutPickerAction::None;

    // fullscreen black backdrop
    let screen = ui.ctx().input(|i| i.raw.screen_rect).unwrap_or(ui.available_rect_before_wrap());
    ui.painter()
        .rect_filled(screen, 0.0, egui::Color32::from_rgba_premultiplied(0, 0, 0, 255));

    // key capture
    let mut pressed: Option<String> = None;
    let mut enter = false;
    let mut escape = false;

    ui.input(|i| {
        for ev in &i.events {
            match ev {
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    if *key == egui::Key::Escape {
                        escape = true;
                    } else if *key == egui::Key::Enter {
                        enter = true;
                    } else if is_modifier(*key) {
                        // ignore modifier-only presses
                    } else {
                        pressed = Some(build_combo(*key, modifiers));
                    }
                }
                _ => {}
            }
        }
    });

    if let Some(combo) = pressed {
        state.combo = Some(combo);
        state.error = None;
    }

    if escape {
        return ShortcutPickerAction::Cancel;
    }

    // right-click cancels
    if ui.input(|i| i.pointer.secondary_clicked()) {
        return ShortcutPickerAction::Cancel;
    }

    // center content
    let center = screen.center();
    let instruction_pos = egui::pos2(center.x, center.y - 40.0);
    ui.painter().text(
        instruction_pos,
        egui::Align2::CENTER_CENTER,
        "press enter to confirm",
        egui::FontId::proportional(SHORTCUT_PICKER_INSTRUCTION_SIZE),
        egui::Color32::from_gray(230),
    );

    let combo_text = state.combo.clone().unwrap_or_default();
    let combo_display = if combo_text.is_empty() {
        "[ ... ]".to_string()
    } else {
        format!("[ {} ]", combo_text)
    };
    let combo_pos = egui::pos2(center.x, center.y + 20.0);
    ui.painter().text(
        combo_pos,
        egui::Align2::CENTER_CENTER,
        combo_display,
        egui::FontId::proportional(SHORTCUT_PICKER_COMBO_SIZE),
        egui::Color32::WHITE,
    );

    if let Some(err) = &state.error {
        ui.painter().text(
            egui::pos2(center.x, center.y + 70.0),
            egui::Align2::CENTER_CENTER,
            err,
            egui::FontId::proportional(14.0),
            egui::Color32::from_rgb(220, 120, 120),
        );
    }

    if enter {
        if let Some(combo) = state.combo.clone() {
            action = ShortcutPickerAction::Save {
                owner_kind: state.owner_kind,
                owner_id: state.owner_id.clone(),
                combo,
            };
        }
        // if no combo yet, ignore enter
    }

    action
}

pub fn is_modifier(key: egui::Key) -> bool {
    matches!(
        key,
        egui::Key::ControlLeft
            | egui::Key::ControlRight
            | egui::Key::ShiftLeft
            | egui::Key::ShiftRight
            | egui::Key::AltLeft
            | egui::Key::AltRight
            | egui::Key::SuperLeft
            | egui::Key::SuperRight
    )
}

pub fn build_combo(key: egui::Key, mods: &egui::Modifiers) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if mods.ctrl {
        parts.push("ctrl");
    }
    if mods.shift {
        parts.push("shift");
    }
    if mods.alt {
        parts.push("alt");
    }
    if mods.mac_cmd || mods.command {
        parts.push("cmd");
    }
    let key_name = key_name(key);
    parts.push(&key_name);
    parts.join("+")
}

fn key_name(key: egui::Key) -> String {
    format!("{:?}", key).to_lowercase()
}
