use eframe::egui;
use design::colors::{ICON_BG_HOVER, ICON_FG};
use design::numbers::{BACK_BUTTON_CLICK_SIZE, BACK_BUTTON_GLYPH_SIZE, ICON_BUTTON_HOVER_CORNER};

pub fn back_button_ui(ui: &mut egui::Ui, center: egui::Pos2) -> bool {
    let rect = egui::Rect::from_center_size(center, egui::vec2(BACK_BUTTON_CLICK_SIZE, BACK_BUTTON_CLICK_SIZE));
    let response = ui.interact(rect, egui::Id::new("back_button"), egui::Sense::click());

    if response.hovered() {
        ui.painter().rect_filled(rect, ICON_BUTTON_HOVER_CORNER, ICON_BG_HOVER);
    }

    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "<",
        egui::FontId::proportional(BACK_BUTTON_GLYPH_SIZE),
        ICON_FG,
    );

    response.clicked()
}
