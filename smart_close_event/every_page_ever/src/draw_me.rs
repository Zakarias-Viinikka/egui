use eframe::egui;

pub trait DrawMe {
    fn draw_me(&self, ui: &mut egui::Ui);
}
