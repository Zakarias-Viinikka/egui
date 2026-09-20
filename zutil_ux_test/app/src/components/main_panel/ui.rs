use eframe::egui;
use crate::components::pulse::ui::PulseState;

const WIDTH_FRACTION: f32 = 0.95;
const HEIGHT_FRACTION: f32 = 0.95;
const CIRCLE_DIAMETER_FRACTION: f32 = 0.90;
const CIRCLE_STROKE_WIDTH: f32 = 2.0;
const BACK_MARGIN: f32 = 20.0;
const TEXTBOX_MAX_WIDTH: f32 = 140.0;
const TEXTBOX_CORNER_RADIUS: f32 = 6.0;
const PULSE_COLOR: egui::Color32 = egui::Color32::WHITE;
const PULSE_DELAY_SECS: f64 = 0.05;

struct PendingPulse {
    started_at: f64,
    index: usize,
}

pub struct MainPanelState {
    pulses: Vec<PulseState>,
    last_highlighted: Option<usize>,
    pending: Option<PendingPulse>,
}

impl Default for MainPanelState {
    fn default() -> Self {
        Self {
            pulses: Vec::new(),
            last_highlighted: None,
            pending: None,
        }
    }
}

pub enum MainPanelAction {
    None,
    Back,
    TextClicked { text: String, center: egui::Pos2, size: egui::Vec2 },
}

pub fn main_panel_ui(ui: &mut egui::Ui, state: &mut MainPanelState, texts: &[String]) -> MainPanelAction {
    let mut action = MainPanelAction::None;

    let screen = ui.available_rect_before_wrap();
    let width = screen.width() * WIDTH_FRACTION;
    let height = screen.height() * HEIGHT_FRACTION;
    let x = screen.left() + (screen.width() - width) / 2.0;
    let y = screen.top() + (screen.height() - height) / 2.0;
    let rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(width, height));

    let radius = rect.width().min(rect.height()) * CIRCLE_DIAMETER_FRACTION / 2.0;
    let stroke = egui::Stroke::new(CIRCLE_STROKE_WIDTH, egui::Color32::from_gray(60));
    ui.painter().circle_stroke(rect.center(), radius, stroke);

    let n = texts.len();
    let mut closest: Option<usize> = None;
    let mut rects: Vec<egui::Rect> = Vec::new();

    if n > 0 {
        let start = -std::f32::consts::FRAC_PI_2;
        let points: Vec<egui::Pos2> = (0..n)
            .map(|i| {
                let angle = start + (i as f32) * std::f32::consts::TAU / (n as f32);
                egui::pos2(
                    rect.center().x + angle.cos() * radius,
                    rect.center().y + angle.sin() * radius,
                )
            })
            .collect();

        let mouse = ui.ctx().input(|i| i.pointer.latest_pos());
        closest = mouse.map(|m| {
            let mut best = 0usize;
            let mut best_d = f32::MAX;
            for (i, p) in points.iter().enumerate() {
                let d = p.distance(m);
                if d < best_d {
                    best_d = d;
                    best = i;
                }
            }
            best
        });

        if closest != state.last_highlighted {
            state.pending = closest.map(|i| PendingPulse {
                started_at: ui.input(|inp| inp.time),
                index: i,
            });
        }
        state.last_highlighted = closest;

        for (i, text) in texts.iter().enumerate() {
            let highlighted = closest == Some(i);
            let r = crate::components::textbox::ui::textbox_ui(
                ui,
                points[i],
                text,
                TEXTBOX_MAX_WIDTH,
                highlighted,
            );
            rects.push(r);
        }

        if let Some(pending) = &state.pending {
            let now = ui.input(|inp| inp.time);
            if now - pending.started_at >= PULSE_DELAY_SECS {
                if let Some(r) = rects.get(pending.index) {
                    state.pulses.push(PulseState::new(
                        ui,
                        r.center(),
                        r.size(),
                        TEXTBOX_CORNER_RADIUS,
                        PULSE_COLOR,
                    ));
                }
                state.pending = None;
            }
        }
    }

    if !state.pulses.is_empty() || state.pending.is_some() {
        ui.ctx().request_repaint();
    }
    state.pulses.retain(|p| !crate::components::pulse::ui::pulse_ui(ui, p));

    let back_center = egui::pos2(rect.right() - BACK_MARGIN, rect.top() + BACK_MARGIN);
    let back_clicked = crate::components::back_button::ui::back_button_ui(ui, back_center);

    if back_clicked {
        action = MainPanelAction::Back;
    } else {
        let pointer_clicked = ui.input(|i| i.pointer.primary_clicked());
        let pointer_pos = ui.input(|i| i.pointer.interact_pos());
        if pointer_clicked {
            if let (Some(pos), Some(i)) = (pointer_pos, closest) {
                if rect.contains(pos) {
                    if let Some(r) = rects.get(i) {
                        action = MainPanelAction::TextClicked {
                            text: texts[i].clone(),
                            center: r.center(),
                            size: r.size(),
                        };
                    }
                }
            }
        }
    }

    action
}
