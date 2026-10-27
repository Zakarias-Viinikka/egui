use egui::Ui;

pub fn show(ui: &mut Ui) {
    ui.heading("Home Page");
    ui.label("This is a reusable page component rendered inside Bevy.");
}