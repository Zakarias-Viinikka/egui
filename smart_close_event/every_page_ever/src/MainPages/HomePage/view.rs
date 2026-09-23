use crate::draw_me::DrawMe;
use eframe::egui;

#[derive(Clone)]
pub struct HomePageDrawer {}

impl DrawMe for HomePageDrawer {
    fn draw_me(&self, ui: &mut egui::Ui) {
        draw_the_page(ui);
    }
}

pub fn draw_the_page(ui: &mut egui::Ui) {
    ui.label("This is HomePage");
    ui.separator();
    if ui.button("Click me").clicked() {
        // do nothing
    }
}
