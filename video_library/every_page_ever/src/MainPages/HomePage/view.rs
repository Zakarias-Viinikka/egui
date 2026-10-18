use super::actions::Actions;
use super::ui_state::HomePageUiState;
use super::{menu, progress_popup, timeline, video_view};
use app_core::state::video_library_state::VideoLibraryState;
use eframe::egui;

pub fn draw(ui: &mut egui::Ui, state: &mut VideoLibraryState, ui_state: &mut HomePageUiState) -> Actions {
    let mut actions = Actions::default();

    menu::draw(ui, state, &mut actions);
    ui.separator();

    if let Some(err) = state.last_error.clone() {
        ui.colored_label(ui.visuals().error_fg_color, err);
        ui.separator();
    }

    let timeline_h = 100.0;
    let reserve_for_timeline = timeline_h + 12.0;
    let available = ui.available_size();
    let video_h = (available.y - reserve_for_timeline).max(80.0);

    ui.allocate_ui_with_layout(
        egui::vec2(available.x, video_h),
        egui::Layout::top_down(egui::Align::Center),
        |ui| { video_view::draw(ui, state, ui_state); },
    );

    ui.separator();
    timeline::draw(ui, state, ui_state, &mut actions);

    progress_popup::draw(ui.ctx(), state, &mut actions);
    actions
}
