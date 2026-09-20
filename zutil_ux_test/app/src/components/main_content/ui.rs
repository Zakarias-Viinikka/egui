use eframe::egui;
use crate::components::white_box::ui::{WhiteBoxAction, WhiteBoxState};

const WORD_POOL: &[&str] = &[
    "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf",
    "hotel", "india", "juliet", "kilo", "lima", "mike", "november",
    "oscar", "papa", "quebec", "romeo", "sierra", "tango", "uniform",
    "victor", "whiskey", "xray", "yankee", "zulu",
];

fn next_word(seed: &mut u64) -> String {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    WORD_POOL[(*seed as usize) % WORD_POOL.len()].to_string()
}

pub enum Page {
    Menu,
    Circle,
}

pub struct MainContentState {
    pub page: Page,
    pub white_box: WhiteBoxState,
    pub textbox_texts: Vec<String>,
    pub main_panel: crate::components::main_panel::ui::MainPanelState,
    pub bounces: Vec<crate::components::bounce_text::ui::BounceTextState>,
}

impl Default for MainContentState {
    fn default() -> Self {
        Self {
            page: Page::Menu,
            white_box: Default::default(),
            textbox_texts: Vec::new(),
            main_panel: Default::default(),
            bounces: Vec::new(),
        }
    }
}

pub fn main_content_ui(ui: &mut egui::Ui, state: &mut MainContentState) {
    match state.page {
        Page::Menu => {
            match crate::components::white_box::ui::white_box_ui(ui, &mut state.white_box) {
                WhiteBoxAction::GoToCircle => {
                    if let Ok(n) = state.white_box.text.trim().parse::<usize>() {
                        let mut seed = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_nanos() as u64)
                            .unwrap_or(0x9E3779B97F4A7C15);
                        if seed == 0 {
                            seed = 0x9E3779B97F4A7C15;
                        }
                        state.textbox_texts = (0..n).map(|_| next_word(&mut seed)).collect();
                        state.page = Page::Circle;
                    }
                }
                WhiteBoxAction::None => {}
            }
        }
        Page::Circle => {
            use crate::components::main_panel::ui::MainPanelAction;
            match crate::components::main_panel::ui::main_panel_ui(
                ui,
                &mut state.main_panel,
                &state.textbox_texts,
            ) {
                MainPanelAction::None => {}
                MainPanelAction::Back => state.page = Page::Menu,
                MainPanelAction::TextClicked { text, center, size } => {
                    state.page = Page::Menu;
                    state.bounces.push(
                        crate::components::bounce_text::ui::BounceTextState::new(
                            ui, center, size, text,
                        ),
                    );
                }
            }
        }
    }

    if !state.bounces.is_empty() {
        ui.ctx().request_repaint();
    }
    state.bounces.retain(|b| !crate::components::bounce_text::ui::bounce_text_ui(ui, b));
}
