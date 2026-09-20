use eframe::egui;
use design::colors::{TEXTBOX_BG, TEXTBOX_BG_HIGHLIGHT, TEXTBOX_TEXT};
use design::numbers::{CORNER_SMALL, TEXTBOX_FONT_SIZE, TEXTBOX_PADDING};

const SHADOW_OFFSET: f32 = 3.0;
const SHADOW_ALPHA: u8 = 60;

fn mul_alpha(c: egui::Color32, a: f32) -> egui::Color32 {
    let alpha = ((c.a() as f32 / 255.0) * a.clamp(0.0, 1.0) * 255.0) as u8;
    egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

pub fn textbox_ui(
    ui: &mut egui::Ui,
    center: egui::Pos2,
    text: &str,
    max_width: f32,
    highlighted: bool,
    alpha: f32,
) -> egui::Rect {
    let wrap_width = max_width - TEXTBOX_PADDING * 2.0;
    let galley = ui.painter().layout(
        text.to_string(),
        egui::FontId::proportional(TEXTBOX_FONT_SIZE),
        mul_alpha(TEXTBOX_TEXT, alpha),
        wrap_width,
    );

    let text_size = galley.size();
    let box_width = (text_size.x + TEXTBOX_PADDING * 2.0).min(max_width);
    let box_height = text_size.y + TEXTBOX_PADDING * 2.0;

    let rect = egui::Rect::from_center_size(center, egui::vec2(box_width, box_height));

    let shadow = egui::Rect::from_min_size(
        rect.min + egui::vec2(0.0, SHADOW_OFFSET),
        rect.size(),
    );
    ui.painter().rect_filled(
        shadow,
        CORNER_SMALL,
        egui::Color32::from_black_alpha((SHADOW_ALPHA as f32 * alpha) as u8),
    );

    let bg = if highlighted { TEXTBOX_BG_HIGHLIGHT } else { TEXTBOX_BG };
    ui.painter().rect_filled(rect, CORNER_SMALL, mul_alpha(bg, alpha));

    let text_pos = egui::pos2(
        rect.left() + (box_width - text_size.x) / 2.0,
        rect.top() + TEXTBOX_PADDING,
    );
    ui.painter().galley(text_pos, galley, mul_alpha(TEXTBOX_TEXT, alpha));

    rect
}
