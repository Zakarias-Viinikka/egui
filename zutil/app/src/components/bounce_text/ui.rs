use eframe::egui;

const DURATION_SECS: f64 = 0.25;
const PADDING: f32 = 10.0;
const CORNER_RADIUS: f32 = 6.0;
const BG_COLOR: egui::Color32 = egui::Color32::from_gray(230);
const TEXT_COLOR: egui::Color32 = egui::Color32::from_gray(30);
const FONT_SIZE: f32 = 24.0;
const SHADOW_OFFSET: f32 = 3.0;
const SHADOW_ALPHA: u8 = 60;

pub struct BounceTextState {
    center: egui::Pos2,
    size: egui::Vec2,
    text: String,
    started_at: f64,
    id: egui::Id,
}

impl BounceTextState {
    pub fn new(ui: &egui::Ui, center: egui::Pos2, size: egui::Vec2, text: String) -> Self {
        let started_at = ui.input(|i| i.time);
        let id = egui::Id::new(("bounce_text", started_at.to_bits()));
        Self { center, size, text, started_at, id }
    }
}

fn with_alpha(c: egui::Color32, alpha: f32) -> egui::Color32 {
    let a = (alpha.clamp(0.0, 1.0) * 255.0) as u8;
    egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

pub fn bounce_text_ui(ui: &egui::Ui, state: &BounceTextState) -> bool {
    let now = ui.input(|i| i.time);
    let t = ((now - state.started_at) / DURATION_SECS) as f32;
    if t >= 1.0 {
        return true;
    }

    let alpha = 1.0 - t;
    let y_off = SHADOW_OFFSET * t;

    let layer_id = egui::LayerId::new(egui::Order::Foreground, state.id);
    let painter = ui.ctx().layer_painter(layer_id);

    // shadow stays put
    let shadow = egui::Rect::from_center_size(
        state.center + egui::vec2(0.0, SHADOW_OFFSET),
        state.size,
    );
    painter.rect_filled(
        shadow,
        CORNER_RADIUS,
        egui::Color32::from_black_alpha((SHADOW_ALPHA as f32 * alpha) as u8),
    );

    // box slides down toward the shadow while fading
    let rect = egui::Rect::from_center_size(state.center + egui::vec2(0.0, y_off), state.size);
    painter.rect_filled(rect, CORNER_RADIUS, with_alpha(BG_COLOR, alpha));

    let wrap = state.size.x - PADDING * 2.0;
    let galley = ui.painter().layout(
        state.text.clone(),
        egui::FontId::proportional(FONT_SIZE),
        with_alpha(TEXT_COLOR, alpha),
        wrap,
    );
    let text_pos = egui::pos2(
        rect.center().x - galley.size().x / 2.0,
        rect.center().y - galley.size().y / 2.0,
    );
    painter.galley(text_pos, galley, with_alpha(TEXT_COLOR, alpha));

    false
}
