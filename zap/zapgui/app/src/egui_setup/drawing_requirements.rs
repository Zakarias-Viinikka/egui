use eframe::egui;

use app_core::text_diff::DiffRowProcessedForUi;
use every_page_ever::ReusableComponents::close_button::ui as close_button;
use every_page_ever::ReusableComponents::diff_viewer::ui as diff_viewer;
use every_page_ever::ReusableComponents::glass_button::ui as glass_button;

pub struct NecessaryStructForEgui {
    rows: Vec<DiffRowProcessedForUi>,
}

impl Default for NecessaryStructForEgui {
    fn default() -> Self {
        // Empty at startup. The first overwrite request fills this in.
        Self { rows: Vec::new() }
    }
}

impl eframe::App for NecessaryStructForEgui {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.8]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(rows) = app_core::terminal_router::take_diff() {
            self.rows = rows;
            // Make the window visible before raising it. wmctrl on a hidden
            // window can silently do nothing, so the order matters.
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Visible(true));
            app_core::terminal_router::route_requests::focus_window();
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
    fn literally_draw_the_page(&self, ctx: CtxForActuallyDrawing);
}

impl LiterallyDrawThePage for NecessaryStructForEgui {
    fn literally_draw_the_page(&self, ctx: CtxForActuallyDrawing) {
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

        diff_viewer::diff_viewer_ui(ui, viewer_rect, &self.rows);

        let button_center_x = screen.right() - margin - buttons_col_width / 2.0;
        let total = 3.0 * button_size + 2.0 * gap;
        let start_y = screen.center().y - total / 2.0 + button_size / 2.0;
        for i in 0..3 {
            let y = start_y + i as f32 * (button_size + gap);
            glass_button::glass_button_ui(ui, egui::pos2(button_center_x, y), i);
        }

        let close_margin = 20.0;
        let close_center = egui::pos2(
            screen.right() - close_margin,
            screen.top() + close_margin,
        );
        if close_button::close_button_ui(ui, close_center) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

pub struct CtxForActuallyDrawing<'a> {
    pub ui: &'a mut egui::Ui,
}
