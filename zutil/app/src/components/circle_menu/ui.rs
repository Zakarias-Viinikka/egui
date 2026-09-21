use eframe::egui;
use crate::components::pulse::ui::PulseState;
use crate::globals::MenuItem;
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
    items: &[MenuItem],
) -> CircleMenuResponse {
    let mut clicked: Option<usize> = None;
    let mut right_clicked: Option<usize> = None;
    let n = items.len();
    let mut rects: Vec<egui::Rect> = vec![egui::Rect::NOTHING; n];

    if crate::globals::modal_open() {
        if !state.pulses.is_empty() {
            ui.ctx().request_repaint();
        }
        state.pulses.retain(|p| !crate::components::pulse::ui::pulse_ui(ui, p));
        return CircleMenuResponse { clicked, right_clicked, rects };
    }

    let screen = ui.available_rect_before_wrap();
    let width = screen.width() * CIRCLE_BOX_WIDTH_FRACTION;
    let height = screen.height() * CIRCLE_BOX_HEIGHT_FRACTION;
    let x = screen.left() + (screen.width() - width) / 2.0;
    let y = screen.top() + (screen.height() - height) / 2.0;
    let rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(width, height));

    let base_radius = rect.width().min(rect.height()) * CIRCLE_DIAMETER_FRACTION / 2.0;
    let depth = crate::globals::depth().max(1);
    let mut radius = base_radius;
    let base = design::colors::CIRCLE_STROKE_BASE_ALPHA as f32;
    let floor = design::colors::CIRCLE_STROKE_FLOOR_ALPHA as f32;
    let decay = design::colors::CIRCLE_STROKE_DECAY;
    for i in 0..depth {
        let alpha = floor + (base - floor) * decay.powi(i as i32);
        let stroke = egui::Stroke::new(CIRCLE_STROKE_WIDTH, light_ring(alpha as u8));
        ui.painter().circle_stroke(rect.center(), radius, stroke);
        radius *= design::numbers::CIRCLE_NEST_SCALE;
        if radius < design::numbers::CIRCLE_NEST_MIN_RADIUS {
            break;
        }
    }

    let now = ui.input(|i| i.time);

    let labels: Vec<String> = items.iter().map(|it| it.label.clone()).collect();
    let menu_changed = labels != state.last_menu;
    if menu_changed {
        state.last_menu = labels;
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

    let mouse = ui
        .ctx()
        .input(|i| i.pointer.latest_pos())
        .filter(|p| p.y > screen.top() + TOP_MENU_ZONE);

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

    if menu_changed {
        state.last_highlighted = closest;
    }

    let mut any_fading = false;
    for (i, item) in items.iter().enumerate() {
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
            &item.label,
            TEXTBOX_MAX_WIDTH,
            highlighted,
            alpha,
        );
        draw_counter(ui, rects[i], item.counter, alpha);
    }

    if closest != state.last_highlighted {
        state.pulses.retain(|p| !p.is_cancellable(now));
        if let Some(i) = closest {
            state.pulses.push(PulseState::new(ui, rects[i], CORNER_SMALL, PULSE));
        }
    }
    state.last_highlighted = closest;

    let pointer_clicked = ui.input(|i| i.pointer.primary_clicked());
    let pointer_secondary = ui.input(|i| i.pointer.secondary_clicked());
    let pointer_pos = ui.input(|i| i.pointer.interact_pos());
    if pointer_clicked {
        if let (Some(pos), Some(i)) = (pointer_pos, closest) {
            if rect.contains(pos) {
                clicked = Some(i);
                crate::globals::record_click(pos);
            }
        }
    }
    if pointer_secondary {
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

fn light_ring(alpha: u8) -> egui::Color32 {
    let a = alpha as f32 / 255.0;
    let base = design::colors::CIRCLE_STROKE_GRAY as f32;
    let r = (base * a) as u8;
    let g = (base * a) as u8;
    let b = (base * a) as u8;
    egui::Color32::from_rgba_premultiplied(r, g, b, alpha)
}

const COUNTER_BOX_W: f32 = 30.0;
const COUNTER_BOX_H: f32 = 20.0;
const COUNTER_GAP: f32 = 6.0;

fn draw_counter(ui: &mut egui::Ui, box_rect: egui::Rect, counter: u32, alpha: f32) {
    if box_rect == egui::Rect::NOTHING {
        return;
    }
    let a = (alpha.clamp(0.0, 1.0) * 255.0) as u8;
    let rect = egui::Rect::from_min_size(
        egui::pos2(box_rect.right() + COUNTER_GAP, box_rect.center().y - COUNTER_BOX_H / 2.0),
        egui::vec2(COUNTER_BOX_W, COUNTER_BOX_H),
    );
    let bg = egui::Color32::from_rgba_premultiplied(40, 40, 40, a);
    let fg = egui::Color32::from_rgba_premultiplied(a, a, a, a);
    ui.painter().rect_filled(rect, 4.0, bg);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        counter.to_string(),
        egui::FontId::proportional(12.0),
        fg,
    );
}
