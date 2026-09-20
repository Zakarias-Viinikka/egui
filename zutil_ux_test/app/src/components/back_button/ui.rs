use eframe::egui;

const SIZE: f32 = 26.0;
const HOVER_BG: egui::Color32 = egui::Color32::from_gray(225);
const GLYPH_COLOR: egui::Color32 = egui::Color32::from_gray(70);

pub fn back_button_ui(ui: &mut egui::Ui, center: egui::Pos2) -> bool {
    let rect = egui::Rect::from_center_size(center, egui::vec2(SIZE, SIZE));
    let response = ui.interact(rect, egui::Id::new("back_button"), egui::Sense::click());

    if response.hovered() {
        ui.painter().rect_filled(rect, 4.0, HOVER_BG);
    }

    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "<",
        egui::FontId::proportional(18.0),
        GLYPH_COLOR,
    );

    response.clicked()
}
