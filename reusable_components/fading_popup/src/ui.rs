use eframe::egui;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const LINGER: Duration = Duration::from_millis(800);
const FADE: Duration = Duration::from_millis(500);
const POPUP_WIDTH: f32 = 320.0;
const POPUP_HEIGHT: f32 = 90.0;

pub enum PopupScreenPosition {
    TopCenter { y_offset: f32 },
}

pub struct FadingPopupParams {
    pub id: String,
    pub title: Option<String>,
    pub body: String,
    pub position: PopupScreenPosition,
    pub done_tx: Sender<()>,
}

pub struct FadingPopupState {
    shared: Arc<Mutex<Shared>>,
    finished: bool,
}

struct Shared {
    created_at: Instant,
    cancelled: bool,
    clicked: bool,
}

impl FadingPopupState {
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Mutex::new(Shared {
                created_at: Instant::now(),
                cancelled: false,
                clicked: false,
            })),
            finished: false,
        }
    }
}

pub fn fading_popup(
    ctx: &egui::Context,
    state: &mut FadingPopupState,
    params: FadingPopupParams,
) {
    if state.finished {
        return;
    }

    {
        let s = state.shared.lock().unwrap();
        if s.cancelled || s.clicked {
            drop(s);
            state.finished = true;
            let _ = params.done_tx.send(());
            return;
        }
    }

    let pos = resolve_position(ctx, &params.position);

    let shared = state.shared.clone();
    let title = params.title.clone();
    let body = params.body.clone();
    let viewport_id = egui::ViewportId::from_hash_of(&params.id);

    ctx.show_viewport_deferred(
        viewport_id,
        egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_taskbar(false)
            .with_resizable(false)
            .with_inner_size([POPUP_WIDTH, POPUP_HEIGHT])
            .with_position([pos.x, pos.y]),
        move |ctx, _class| {
            let now = Instant::now();
            let mut s = shared.lock().unwrap();

            let age = now.duration_since(s.created_at);
            let alpha = if age < LINGER {
                1.0
            } else {
                let t = ((age - LINGER).as_secs_f32() / FADE.as_secs_f32()).min(1.0);
                1.0 - t
            };

            if alpha <= 0.0 {
                s.cancelled = true;
                ctx.request_repaint();
                return;
            }

            let mut hovered = false;
            let mut clicked = false;

            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.fill(egui::Color32::TRANSPARENT))
                .show(ctx, |ui| {
                    let bg = egui::Color32::from_black_alpha((220.0 * alpha) as u8);
                    let text_color =
                        egui::Color32::from_white_alpha((255.0 * alpha) as u8);

                    let response = egui::Frame::NONE
                        .fill(bg)
                        .inner_margin(12.0)
                        .corner_radius(6.0)
                        .show(ui, |ui| {
                            ui.set_min_width(POPUP_WIDTH - 24.0);
                            if let Some(t) = &title {
                                ui.label(
                                    egui::RichText::new(t).color(text_color).strong(),
                                );
                            }
                            ui.label(egui::RichText::new(&body).color(text_color));
                        })
                        .response;

                    hovered = response.hovered();
                    clicked = response.clicked();
                });

            if hovered {
                s.created_at = now;
            }
            if clicked {
                s.clicked = true;
            }

            ctx.request_repaint_after(Duration::from_millis(16));
        },
    );
}

fn resolve_position(ctx: &egui::Context, pos: &PopupScreenPosition) -> egui::Pos2 {
    let monitor_width = ctx
        .input(|i| i.viewport().monitor_size.map(|v| v.x))
        .unwrap_or(1920.0);

    match pos {
        PopupScreenPosition::TopCenter { y_offset } => {
            egui::pos2(monitor_width / 2.0 - POPUP_WIDTH / 2.0, *y_offset)
        }
    }
}
