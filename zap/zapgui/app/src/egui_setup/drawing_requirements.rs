use eframe::egui;
use std::sync::Arc;
use std::time::{Duration, Instant};

use app_core::terminal_router::pending_overwrite::PendingOverwrite;
use every_page_ever::ReusableComponents::close_button::ui as close_button;
use every_page_ever::ReusableComponents::diff_viewer::ui as diff_viewer;
use every_page_ever::ReusableComponents::glass_button::ui::{
    self as glass_button, Icon as GlassIcon,
};

const CLICK_GUARD: Duration = Duration::from_millis(500);
const ERROR_COLOR: egui::Color32 = egui::Color32::from_rgb(210, 80, 80);

pub struct NecessaryStructForEgui {
    shown: Option<Arc<PendingOverwrite>>,
    shown_at: Instant,
    error_line: Option<String>,
}

impl Default for NecessaryStructForEgui {
    fn default() -> Self {
        Self {
            shown: None,
            shown_at: Instant::now(),
            error_line: None,
        }
    }
}

impl eframe::App for NecessaryStructForEgui {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.8]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let current = app_core::terminal_router::current_pending();
        let changed = match (&self.shown, &current) {
            (Some(a), Some(b)) => !Arc::ptr_eq(a, b),
            (None, None) => false,
            _ => true,
        };
        if changed {
            self.shown = current;
            self.shown_at = Instant::now();
            self.error_line = None;
            if self.shown.is_some() {
                // Make the window visible before raising it. wmctrl on a
                // hidden window silently does nothing, so the order matters.
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Visible(true));
                app_core::terminal_router::route_requests::focus_window();
            }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::TRANSPARENT))
            .show(ui, |ui| {
                let ctx = CtxForActuallyDrawing { ui };
                self.literally_draw_the_page(ctx);
            });
    }
}

pub trait LiterallyDrawThePage {
    fn literally_draw_the_page(&mut self, ctx: CtxForActuallyDrawing);
}

impl LiterallyDrawThePage for NecessaryStructForEgui {
    fn literally_draw_the_page(&mut self, ctx: CtxForActuallyDrawing) {
        let ui = ctx.ui;
        let screen = ui.available_rect_before_wrap();
        let margin = 40.0;
        let buttons_col_width = 60.0;
        let gap = 12.0;
        let button_size = 44.0;

        let viewer_rect = egui::Rect::from_min_max(
            egui::pos2(screen.left() + margin, screen.top() + margin),
            egui::pos2(
                screen.right() - margin - buttons_col_width - gap,
                screen.bottom() - margin,
            ),
        );

        let blocks: &[app_core::text_diff::DiffRowProcessedForUi] = self
            .shown
            .as_ref()
            .map(|p| p.rows.as_slice())
            .unwrap_or(&[]);
        diff_viewer::diff_viewer_ui(ui, viewer_rect, blocks);

        let click_ok = self.shown_at.elapsed() >= CLICK_GUARD;
        let mut confirmed = false;
        let mut denied = false;

        if self.shown.is_some() {
            let button_center_x = screen.right() - margin - buttons_col_width / 2.0;
            let total = 2.0 * button_size + gap;
            let start_y = screen.center().y - total / 2.0 + button_size / 2.0;

            let y0 = start_y;
            if glass_button::glass_button_ui(
                ui,
                egui::pos2(button_center_x, y0),
                0,
                GlassIcon::Confirm,
            ) && click_ok
            {
                confirmed = true;
            }
            let y1 = start_y + button_size + gap;
            if glass_button::glass_button_ui(
                ui,
                egui::pos2(button_center_x, y1),
                1,
                GlassIcon::Deny,
            ) && click_ok
            {
                denied = true;
            }
        }

        let close_margin = 20.0;
        let close_center = egui::pos2(
            screen.right() - close_margin,
            screen.top() + close_margin,
        );
        if close_button::close_button_ui(ui, close_center) {
            // Hide, don't quit. The pending edit stays in the slot; a newer
            // overwrite replaces it, and a confirm or deny clears it.
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        if let Some(line) = &self.error_line {
            let rect = egui::Rect::from_min_max(
                egui::pos2(screen.left() + margin, screen.bottom() - 46.0),
                egui::pos2(screen.right() - margin, screen.bottom() - 16.0),
            );
            ui.put(
                rect,
                egui::Label::new(egui::RichText::new(line.as_str()).color(ERROR_COLOR)).wrap(),
            );
        }

        if confirmed {
            if let Some(shown) = self.shown.clone() {
                match shown.confirm() {
                    Ok(()) => {
                        app_core::terminal_router::clear_pending(&shown);
                        self.shown = None;
                        self.error_line = None;
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    }
                    Err(reason) => {
                        let sentence = format!(
                            "failed request to edit file {}: {}",
                            shown.path, reason
                        );
                        self.error_line = Some(sentence.clone());
                        let _ = app_core::clipboard::copy_to_clipboard(&sentence);
                    }
                }
            }
        }

        if denied {
            if let Some(shown) = self.shown.clone() {
                match shown.deny() {
                    Ok(()) => {
                        app_core::terminal_router::clear_pending(&shown);
                        self.shown = None;
                        self.error_line = None;
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    }
                    Err(reason) => {
                        let sentence = format!("couldn't copy to the clipboard: {reason}");
                        self.error_line = Some(sentence.clone());
                        let _ = app_core::clipboard::copy_to_clipboard(&sentence);
                    }
                }
            }
        }
    }
}

pub struct CtxForActuallyDrawing<'a> {
    pub ui: &'a mut egui::Ui,
}
