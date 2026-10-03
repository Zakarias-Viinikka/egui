use super::Action;
use eframe::egui::{InputState, Key};

/// Ctrl+C at the prompt. In a real terminal this sends SIGINT to whatever's
/// running. There's no process here, so the equivalent is clearing the
/// current input line. Plain Ctrl+C, no shift or alt.
pub fn check(input: &InputState) -> Option<Action> {
    if input.modifiers.ctrl
        && !input.modifiers.shift
        && !input.modifiers.alt
        && input.key_pressed(Key::C)
    {
        return Some(Action::ClearInput);
    }
    None
}
