use eframe::egui;
use crate::components::bounce_text::ui::BounceTextState;
use crate::components::main_panel::ui::{MainPanelAction, MainPanelState};

const PLACEHOLDER_TEXTS: &[&str] = &["alpha", "bravo", "charlie", "delta", "echo"];

pub struct MainContentState {
    pub main_panel: MainPanelState,
    pub bounces: Vec<BounceTextState>,
    pub hidden_box: Option<usize>,
}

impl Default for MainContentState {
    fn default() -> Self {
        Self {
            main_panel: MainPanelState::default(),
            bounces: Vec::new(),
            hidden_box: None,
        }
    }
}

pub fn main_content_ui(ui: &mut egui::Ui, state: &mut MainContentState) {
    let texts: Vec<String> = PLACEHOLDER_TEXTS.iter().map(|s| s.to_string()).collect();

    match crate::components::main_panel::ui::main_panel_ui(
        ui,
        &mut state.main_panel,
        &texts,
        state.hidden_box,
    ) {
        MainPanelAction::None => {}
        MainPanelAction::TextClicked { index, rect } => {
            if state.bounces.is_empty() {
                state.hidden_box = Some(index);
                state.bounces.push(BounceTextState::new(
                    ui,
                    rect.center(),
                    rect.size(),
                    texts[index].clone(),
                ));
            }
        }
    }

    state.bounces.retain(|b| !crate::components::bounce_text::ui::bounce_text_ui(ui, b));
    if !state.bounces.is_empty() {
        ui.ctx().request_repaint();
    } else {
        state.hidden_box = None;
    }
}
