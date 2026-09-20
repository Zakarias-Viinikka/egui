use eframe::egui;

pub enum MainMenuAction {
    None,
    GoToEditText,
    GoToJson,
    GoToProjects,
}

pub fn main_menu_ui(ui: &mut egui::Ui) -> MainMenuAction {
    let mut action = MainMenuAction::None;

    ui.heading("zutil");
    ui.separator();

    let width = ui.available_width();
    if ui.add_sized([width, 60.0], egui::Button::new("View all")).clicked() {
        action = MainMenuAction::GoToEditText;
    }
    if ui.add_sized([width, 60.0], egui::Button::new("Projects")).clicked() {
        action = MainMenuAction::GoToProjects;
    }

    if ui.button("Json").clicked() {
        action = MainMenuAction::GoToJson;
    }

    action
}
