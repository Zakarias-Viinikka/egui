use eframe::egui;

const DURATION_SECS: f64 = 0.8;
const MAX_EXPANSION: f32 = 30.0;
const THICKNESS: f32 = 3.0;

pub struct PulseState {
    center: egui::Pos2,
    size: egui::Vec2,
    corner_radius: f32,
    color: egui::Color32,
    started_at: f64,
}

impl PulseState {
    pub fn new(
        ui: &egui::Ui,
        center: egui::Pos2,
        size: egui::Vec2,
        corner_radius: f32,
        color: egui::Color32,
    ) -> Self {
        Self {
            center,
            size,
            corner_radius,
            color,
            started_at: ui.input(|i| i.time),
        }
    }
}

pub fn pulse_ui(ui: &mut egui::Ui, state: &PulseState) -> bool {
    let now = ui.input(|i| i.time);
    let progress = ((now - state.started_at) / DURATION_SECS) as f32;

    if progress >= 1.0 {
        return true;
    }

    let expand = MAX_EXPANSION * progress;
    let rect = egui::Rect::from_center_size(state.center, state.size).expand(expand);
    let alpha = ((1.0 - progress) * 255.0) as u8;
    let color = egui::Color32::from_rgba_unmultiplied(
        state.color.r(),
        state.color.g(),
        state.color.b(),
        alpha,
    );

    ui.painter().rect_stroke(
        rect,
        state.corner_radius + expand,
        egui::Stroke::new(THICKNESS, color),
        egui::StrokeKind::Outside,
    );

    false
}
