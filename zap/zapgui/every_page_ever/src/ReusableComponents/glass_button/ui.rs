use eframe::egui;

const SIZE: f32 = 44.0;
const CORNER: f32 = 10.0;
// from_rgba_premultiplied(r*a/255, g*a/255, b*a/255, a)
const FILL: egui::Color32 = egui::Color32::from_rgba_premultiplied(12, 12, 12, 110);
const FILL_HOVER: egui::Color32 = egui::Color32::from_rgba_premultiplied(37, 37, 37, 160);
const BORDER: egui::Color32 = egui::Color32::from_rgba_premultiplied(50, 50, 50, 50);
const BORDER_HOVER: egui::Color32 = egui::Color32::from_rgba_premultiplied(110, 110, 110, 110);

pub fn glass_button_ui(
    ui: &mut egui::Ui,
    center: egui::Pos2,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
) -> bool {
    let rect = egui::Rect::from_center_size(center, egui::vec2(SIZE, SIZE));
    let id = egui::Id::new("glass_button").with(id_salt);
    let response = ui.interact(rect, id, egui::Sense::click());

    let (fill, border) = if response.hovered() {
        (FILL_HOVER, BORDER_HOVER)
    } else {
        (FILL, BORDER)
    };

    ui.painter().rect(
        rect,
        CORNER,
        fill,
        egui::Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );

    response.clicked()
}
