use eframe::egui;

const PADDING: f32 = 10.0;
const CORNER_RADIUS: f32 = 6.0;
const BG_COLOR: egui::Color32 = egui::Color32::from_gray(230);
const HIGHLIGHT_BG: egui::Color32 = egui::Color32::from_rgb(175, 205, 245);
const TEXT_COLOR: egui::Color32 = egui::Color32::from_gray(30);
const FONT_SIZE: f32 = 24.0;

pub fn textbox_ui(
    ui: &mut egui::Ui,
    center: egui::Pos2,
    text: &str,
    max_width: f32,
    highlighted: bool,
) -> egui::Rect {
    let wrap_width = max_width - PADDING * 2.0;
    let galley = ui.painter().layout(
        text.to_string(),
        egui::FontId::proportional(FONT_SIZE),
        TEXT_COLOR,
        wrap_width,
    );

    let text_size = galley.size();
    let box_width = (text_size.x + PADDING * 2.0).min(max_width);
    let box_height = text_size.y + PADDING * 2.0;

    let rect = egui::Rect::from_center_size(center, egui::vec2(box_width, box_height));

    let bg = if highlighted { HIGHLIGHT_BG } else { BG_COLOR };
    ui.painter().rect_filled(rect, CORNER_RADIUS, bg);

    let text_pos = egui::pos2(
        rect.left() + (box_width - text_size.x) / 2.0,
        rect.top() + PADDING,
    );
    ui.painter().galley(text_pos, galley, TEXT_COLOR);

    rect
}
