use eframe::egui;

const SIZE: f32 = 26.0;
const PAD: f32 = 7.0;
const HOVER_BG: egui::Color32 = egui::Color32::from_gray(225);
const X_COLOR: egui::Color32 = egui::Color32::from_gray(70);

pub fn close_button_ui(ui: &mut egui::Ui, center: egui::Pos2) -> bool {
    let rect = egui::Rect::from_center_size(center, egui::vec2(SIZE, SIZE));
    let response = ui.interact(rect, egui::Id::new("close_button"), egui::Sense::click());

    if response.hovered() {
        ui.painter().rect_filled(rect, 4.0, HOVER_BG);
    }

    let stroke = egui::Stroke::new(2.0, X_COLOR);
    let tl = egui::pos2(rect.left() + PAD, rect.top() + PAD);
    let br = egui::pos2(rect.right() - PAD, rect.bottom() - PAD);
    let tr = egui::pos2(rect.right() - PAD, rect.top() + PAD);
    let bl = egui::pos2(rect.left() + PAD, rect.bottom() - PAD);
    ui.painter().line_segment([tl, br], stroke);
    ui.painter().line_segment([tr, bl], stroke);

    response.clicked()
}
