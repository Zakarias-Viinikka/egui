use eframe::egui;
use crate::components::pulse::ui::PulseState;
use design::colors::{CIRCLE_STROKE, PULSE};
use design::numbers::{
    CIRCLE_BOX_HEIGHT_FRACTION, CIRCLE_BOX_WIDTH_FRACTION, CIRCLE_DIAMETER_FRACTION,
    CIRCLE_STROKE_WIDTH, CORNER_SMALL, FADE_DURATION, FADE_RADIUS, TEXTBOX_MAX_WIDTH,
    TOP_MENU_ZONE,
};

pub struct CircleMenuState {
    pulses: Vec<PulseState>,
    last_highlighted: Option<usize>,
    last_menu: Vec<String>,
    fade_from: Option<egui::Pos2>,
    fade_started: f64,
}

impl Default for CircleMenuState {
    fn default() -> Self {
        Self {
            pulses: Vec::new(),
            last_highlighted: None,
            last_menu: Vec::new(),
            fade_from: None,
            fade_started: 0.0,
        }
    }
}

impl CircleMenuState {
    pub fn reset(&mut self) {
        self.last_highlighted = None;
        self.pulses.clear();
    }
}

pub struct CircleMenuResponse {
    pub clicked: Option<usize>,
    pub right_clicked: Option<usize>,
    pub rects: Vec<egui::Rect>,
}

pub fn circle_menu_ui(
    ui: &mut egui::Ui,
    state: &mut CircleMenuState,
    texts: &[String],
) -> CircleMenuResponse {
    let mut clicked: Option<usize> = None;
    let mut right_clicked: Option<usize> = None;
    let n = texts.len();
    let mut rects: Vec<egui::Rect> = vec![egui::Rect::NOTHING; n];

    let screen = ui.available_rect_before_wrap();
    let width = screen.width() * CIRCLE_BOX_WIDTH_FRACTION;
    let height = screen.height() * CIRCLE_BOX_HEIGHT_FRACTION;
    let x = screen.left() + (screen.width() - width) / 2.0;
    let y = screen.top() + (screen.height() - height) / 2.0;
    let rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(width, height));

    let radius = rect.width().min(rect.height()) * CIRCLE_DIAMETER_FRACTION / 2.0;
    let stroke = egui::Stroke::new(CIRCLE_STROKE_WIDTH, CIRCLE_STROKE);
    ui.painter().circle_stroke(rect.center(), radius, stroke);

    let now = ui.input(|i| i.time);

    let menu_changed = texts != state.last_menu.as_slice();
    if menu_changed {
        state.last_menu = texts.to_vec();
        state.fade_started = now;
        state.fade_from = if crate::globals::depth() > 1 {
            crate::globals::last_click()
        } else {
            None
        };
        state.pulses.clear();
    }

    if n == 0 {
        state.pulses.retain(|p| !crate::components::pulse::ui::pulse_ui(ui, p));
        return CircleMenuResponse { clicked, right_clicked, rects };
    }

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

    let modal = crate::globals::modal_open();

    let mouse = ui
        .ctx()
        .input(|i| i.pointer.latest_pos())
        .filter(|p| !modal && p.y > screen.top() + TOP_MENU_ZONE);

    let closest = mouse.and_then(|m| {
        let mut best: Option<usize> = None;
        let mut best_d = f32::MAX;
        for (i, p) in points.iter().enumerate() {
            let d = p.distance(m);
            if d < best_d {
                best_d = d;
                best = Some(i);
            }
        }
        best
    });

    let on_a_box = ui
        .ctx()
        .input(|i| i.pointer.latest_pos())
        .map(|p| rects.iter().any(|r| *r != egui::Rect::NOTHING && r.contains(p)))
        .unwrap_or(false);
    crate::globals::set_disable_rightclick(on_a_box);

    if menu_changed {
        state.last_highlighted = closest;
    }

    let mut any_fading = false;
    for (i, text) in texts.iter().enumerate() {
        let highlighted = closest == Some(i);
        let alpha = match state.fade_from {
            Some(from) if from.distance(points[i]) < FADE_RADIUS => {
                let t = ((now - state.fade_started) / FADE_DURATION).min(1.0) as f32;
                if t < 1.0 {
                    any_fading = true;
                }
                t
            }
            _ => 1.0,
        };
        rects[i] = crate::components::textbox::ui::textbox_ui(
            ui,
            points[i],
            text,
            TEXTBOX_MAX_WIDTH,
            highlighted,
            alpha,
        );
    }

    if closest != state.last_highlighted {
        if !modal {
            state.pulses.retain(|p| !p.is_cancellable(now));
            if let Some(i) = closest {
                state.pulses.push(PulseState::new(ui, rects[i], CORNER_SMALL, PULSE));
            }
        }
    }
    state.last_highlighted = closest;

    let pointer_clicked = ui.input(|i| i.pointer.primary_clicked());
    let pointer_secondary = ui.input(|i| i.pointer.secondary_clicked());
    let pointer_pos = ui.input(|i| i.pointer.interact_pos());
    if pointer_clicked && !modal {
        if let (Some(pos), Some(i)) = (pointer_pos, closest) {
            if rect.contains(pos) {
                clicked = Some(i);
                crate::globals::record_click(pos);
            }
        }
    }
    if pointer_secondary && !modal {
        if let (Some(pos), Some(i)) = (pointer_pos, closest) {
            if rects[i] != egui::Rect::NOTHING && rects[i].contains(pos) {
                right_clicked = Some(i);
            }
        }
    }

    if !state.pulses.is_empty() || any_fading {
        ui.ctx().request_repaint();
    }
    state.pulses.retain(|p| !crate::components::pulse::ui::pulse_ui(ui, p));

    CircleMenuResponse { clicked, right_clicked, rects }
}
