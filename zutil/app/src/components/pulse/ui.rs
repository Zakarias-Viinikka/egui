use eframe::egui;
use std::sync::atomic::{AtomicU64, Ordering};
use design::numbers::{
    PULSE_CANCEL_WINDOW_SECS, PULSE_DURATION_SECS, PULSE_MAX_EXPANSION, PULSE_THICKNESS,
};

static NEXT_PULSE_ID: AtomicU64 = AtomicU64::new(0);

pub struct PulseState {
    pub id: egui::Id,
    pub started_at: f64,
    pub rect: egui::Rect,
    pub corner_radius: f32,
    pub color: egui::Color32,
}

impl PulseState {
    pub fn new(
        ui: &egui::Ui,
        rect: egui::Rect,
        corner_radius: f32,
        color: egui::Color32,
    ) -> Self {
        let id = egui::Id::new(("pulse", NEXT_PULSE_ID.fetch_add(1, Ordering::Relaxed)));
        Self {
            id,
            started_at: ui.input(|i| i.time),
            rect,
            corner_radius,
            color,
        }
    }

    pub fn is_cancellable(&self, now: f64) -> bool {
        (now - self.started_at) < PULSE_CANCEL_WINDOW_SECS
    }
}

pub fn pulse_ui(ui: &mut egui::Ui, state: &PulseState) -> bool {
    let now = ui.input(|i| i.time);
    let progress = ((now - state.started_at) / PULSE_DURATION_SECS) as f32;

    if progress >= 1.0 {
        return true;
    }

    let expand = PULSE_MAX_EXPANSION * progress;
    let rect = state.rect.expand(expand);
    let alpha = ((1.0 - progress) * 255.0) as u8;
    let color = egui::Color32::from_rgba_unmultiplied(
        state.color.r(),
        state.color.g(),
        state.color.b(),
        alpha,
    );

    let clip_rect = rect.expand(PULSE_THICKNESS + 1.0);
    let painter = ui.painter().with_clip_rect(clip_rect);
    painter.rect_stroke(
        rect,
        state.corner_radius + expand,
        egui::Stroke::new(PULSE_THICKNESS, color),
        egui::StrokeKind::Outside,
    );

    false
}
