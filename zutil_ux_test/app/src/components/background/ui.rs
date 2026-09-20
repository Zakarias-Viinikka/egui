use eframe::egui;

const CLOSE_MARGIN: f32 = 20.0;

pub fn background_color() -> [f32; 4] {
    [0.0, 0.0, 0.0, 0.8]
}

pub fn background_ui<F: FnOnce(&mut egui::Ui)>(ui: &mut egui::Ui, content: F) {
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(egui::Color32::TRANSPARENT))
        .show(ui, |ui| {
            content(ui);

            let screen = ui.available_rect_before_wrap();
            let close_center =
                egui::pos2(screen.right() - CLOSE_MARGIN, screen.top() + CLOSE_MARGIN);
            if crate::components::close_button::ui::close_button_ui(ui, close_center) {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
        });
}
