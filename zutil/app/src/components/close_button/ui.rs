use eframe::egui;
use design::colors::{ICON_BG_HOVER, ICON_FG};
use design::numbers::{CLOSE_BUTTON_PAD, CLOSE_BUTTON_SIZE};

pub fn close_button_ui(ui: &mut egui::Ui, center: egui::Pos2) -> bool {
    let rect = egui::Rect::from_center_size(center, egui::vec2(CLOSE_BUTTON_SIZE, CLOSE_BUTTON_SIZE));
    let response = ui.interact(rect, egui::Id::new("close_button"), egui::Sense::click());

    if response.hovered() {
        ui.painter().rect_filled(rect, 4.0, ICON_BG_HOVER);
    }

    let stroke = egui::Stroke::new(2.0, ICON_FG);
    let tl = egui::pos2(rect.left() + CLOSE_BUTTON_PAD, rect.top() + CLOSE_BUTTON_PAD);
    let br = egui::pos2(rect.right() - CLOSE_BUTTON_PAD, rect.bottom() - CLOSE_BUTTON_PAD);
    let tr = egui::pos2(rect.right() - CLOSE_BUTTON_PAD, rect.top() + CLOSE_BUTTON_PAD);
    let bl = egui::pos2(rect.left() + CLOSE_BUTTON_PAD, rect.bottom() - CLOSE_BUTTON_PAD);
    ui.painter().line_segment([tl, br], stroke);
    ui.painter().line_segment([tr, bl], stroke);

    response.clicked()
}
