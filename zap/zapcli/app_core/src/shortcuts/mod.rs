pub mod ctrl_c;
pub mod ctrl_shift_c;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Cancel whatever's currently typed. Like SIGINT at a real shell prompt,
    /// except there's no process to interrupt here, so the only thing to
    /// cancel is the input line.
    ClearInput,
    /// Copy the entire scrollback to the clipboard. Same as the `copy` command.
    CopyAll,
}

/// Ask each shortcut module whether the current frame's input matches it.
/// Shift-flagged variants are checked first because their modifier set is a
/// superset of the plain variant.
pub fn check_all(input: &eframe::egui::InputState) -> Option<Action> {
    if let Some(a) = ctrl_shift_c::check(input) {
        return Some(a);
    }
    if let Some(a) = ctrl_c::check(input) {
        return Some(a);
    }
    None
}
