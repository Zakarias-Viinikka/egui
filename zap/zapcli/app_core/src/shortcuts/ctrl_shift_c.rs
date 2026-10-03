use super::Action;
use eframe::egui::{InputState, Key};

/// Ctrl+Shift+C copies the entire scrollback to the clipboard. This is the
/// terminal convention, and it frees plain Ctrl+C to mean "cancel" without
/// ever touching the clipboard.
pub fn check(input: &InputState) -> Option<Action> {
    if input.modifiers.ctrl
        && input.modifiers.shift
        && input.key_pressed(Key::C)
    {
        return Some(Action::CopyAll);
    }
    None
}
