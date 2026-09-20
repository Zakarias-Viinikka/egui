use eframe::egui;

pub struct ViewTextState {
    pub title: String,
    pub body: String,
}

pub enum ViewTextAction {
    None,
    Back,
}

pub fn view_text_ui(ui: &mut egui::Ui, state: &ViewTextState) -> ViewTextAction {
    let mut action = ViewTextAction::None;

    ui.horizontal(|ui| {
        if ui.button("Back").clicked() {
            action = ViewTextAction::Back;
        }
        ui.separator();
        ui.heading(&state.title);
    });

    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add(egui::Label::new(&state.body).selectable(true));
    });

    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
        ui.add_space(8.0);
        let width = ui.available_width();
        if ui.add_sized([width, 40.0], egui::Button::new("Copy")).clicked() {
            ui.ctx().copy_text(state.body.clone());
        }
    });

    action
}
