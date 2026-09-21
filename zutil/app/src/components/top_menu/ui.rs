use eframe::egui;
use design::numbers::{
    TOP_MENU_BUTTON_HEIGHT, TOP_MENU_BUTTON_WIDTH, TOP_MENU_GAP, TOP_MENU_MARGIN,
};

pub enum TopMenuAction {
    None,
    GoToViewAll,
    GoToProjects,
    GoToJson,
    NewPlus,
    HalveCounters,
}

pub fn top_menu_ui(ui: &mut egui::Ui) -> TopMenuAction {
    let mut action = TopMenuAction::None;

    let screen_width = ui.available_width();
    let total = TOP_MENU_BUTTON_WIDTH * 5.0 + TOP_MENU_GAP * 4.0;
    let left_pad = ((screen_width - total) / 2.0).max(0.0);

    ui.add_space(TOP_MENU_MARGIN);
    ui.horizontal(|ui| {
        ui.add_space(left_pad);

        if ui.add_sized([TOP_MENU_BUTTON_WIDTH, TOP_MENU_BUTTON_HEIGHT], egui::Button::new("View all")).clicked() {
            action = TopMenuAction::GoToViewAll;
        }
        ui.add_space(TOP_MENU_GAP);
        if ui.add_sized([TOP_MENU_BUTTON_WIDTH, TOP_MENU_BUTTON_HEIGHT], egui::Button::new("Projects")).clicked() {
            action = TopMenuAction::GoToProjects;
        }
        ui.add_space(TOP_MENU_GAP);
        if ui.add_sized([TOP_MENU_BUTTON_WIDTH, TOP_MENU_BUTTON_HEIGHT], egui::Button::new("Json")).clicked() {
            action = TopMenuAction::GoToJson;
        }
        ui.add_space(TOP_MENU_GAP);
        if ui.add_sized([TOP_MENU_BUTTON_WIDTH, TOP_MENU_BUTTON_HEIGHT], egui::Button::new("New +")).clicked() {
            action = TopMenuAction::NewPlus;
        }
        ui.add_space(TOP_MENU_GAP);
        if ui.add_sized([TOP_MENU_BUTTON_WIDTH, TOP_MENU_BUTTON_HEIGHT], egui::Button::new("Half ctrs")).clicked() {
            action = TopMenuAction::HalveCounters;
        }
    });

    action
}
