use eframe::egui;

const WIDTH_FRACTION: f32 = 0.70;
const HEIGHT_FRACTION: f32 = 0.70;
const PAD: f32 = 24.0;
const FIELD_HEIGHT: f32 = 32.0;
const GAP: f32 = 14.0;
const BUTTON_HEIGHT: f32 = 34.0;

const FIELD_BG: egui::Color32 = egui::Color32::from_gray(248);
const FIELD_BORDER: egui::Color32 = egui::Color32::from_gray(180);
const FIELD_BORDER_HOVER: egui::Color32 = egui::Color32::from_gray(120);
const FIELD_TEXT: egui::Color32 = egui::Color32::from_gray(30);

pub struct WhiteBoxState {
    pub text: String,
}

impl Default for WhiteBoxState {
    fn default() -> Self {
        Self { text: String::new() }
    }
}

pub enum WhiteBoxAction {
    None,
    GoToCircle,
}

pub fn white_box_ui(ui: &mut egui::Ui, state: &mut WhiteBoxState) -> WhiteBoxAction {
    let mut action = WhiteBoxAction::None;

    let screen = ui.available_rect_before_wrap();
    let width = screen.width() * WIDTH_FRACTION;
    let height = screen.height() * HEIGHT_FRACTION;
    let x = screen.left() + (screen.width() - width) / 2.0;
    let y = screen.top() + (screen.height() - height) / 2.0;
    let rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(width, height));

    ui.painter().rect_filled(rect, 8.0, egui::Color32::WHITE);

    let inner = rect.shrink(PAD);

    let field_rect = egui::Rect::from_min_size(
        inner.min,
        egui::vec2(inner.width(), FIELD_HEIGHT),
    );
    ui.scope(|ui| {
        let v = ui.visuals_mut();
        v.override_text_color = Some(FIELD_TEXT);
        v.extreme_bg_color = FIELD_BG;
        v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, FIELD_BORDER);
        v.widgets.hovered.bg_stroke = egui::Stroke::new(1.5, FIELD_BORDER_HOVER);
        v.widgets.active.bg_stroke = egui::Stroke::new(1.5, FIELD_BORDER_HOVER);
        ui.put(
            field_rect,
            egui::TextEdit::singleline(&mut state.text).hint_text("Type here..."),
        );
    });

    let button_rect = egui::Rect::from_min_size(
        egui::pos2(inner.min.x, field_rect.bottom() + GAP),
        egui::vec2(inner.width(), BUTTON_HEIGHT),
    );
    if ui.put(button_rect, egui::Button::new("Go to circle")).clicked() {
        action = WhiteBoxAction::GoToCircle;
    }

    action
}
