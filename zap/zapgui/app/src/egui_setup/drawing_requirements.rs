use eframe::egui;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use app_core::chain::{popup, report, writer, Chain, StepStatus, Verb};
use app_core::terminal_router::surface_review::SurfaceReview;
use every_page_ever::ReusableComponents::close_button::ui as close_button;
use every_page_ever::ReusableComponents::diff_viewer::ui as diff_viewer;
use every_page_ever::ReusableComponents::glass_button::ui::{
    self as glass_button, BlockState, Icon as GlassIcon,
};

const CLICK_GUARD: Duration = Duration::from_millis(500);
const ERROR_COLOR: egui::Color32 = egui::Color32::from_rgb(210, 80, 80);
const CAPTION_COLOR: egui::Color32 = egui::Color32::from_rgb(220, 220, 220);
const CAPTION_DENIED_COLOR: egui::Color32 = egui::Color32::from_rgb(210, 80, 80);
const SURFACE_BANNER_BG: egui::Color32 = egui::Color32::from_rgb(60, 42, 12);
const SURFACE_BANNER_BORDER: egui::Color32 = egui::Color32::from_rgb(230, 160, 40);
const SURFACE_BANNER_TEXT: egui::Color32 = egui::Color32::from_rgb(255, 200, 110);

use app_core::terminal_router::surface_review::BannerKind;

fn banner_title(kind: BannerKind) -> &'static str {
    match kind {
        BannerKind::CargoRun => "RUN REQUEST",
        BannerKind::Execute => "EXECUTE REQUEST",
        BannerKind::Delete => "DELETE REQUEST",
        BannerKind::Rename => "RENAME REQUEST",
    }
}

fn banner_colors(kind: BannerKind) -> (egui::Color32, egui::Color32, egui::Color32) {
    match kind {
        BannerKind::CargoRun => (SURFACE_BANNER_BG, SURFACE_BANNER_BORDER, SURFACE_BANNER_TEXT),
        BannerKind::Execute => (
            egui::Color32::from_rgb(48, 20, 56),
            egui::Color32::from_rgb(200, 90, 220),
            egui::Color32::from_rgb(230, 170, 240),
        ),
        BannerKind::Delete => (
            egui::Color32::from_rgb(60, 16, 16),
            egui::Color32::from_rgb(220, 70, 70),
            egui::Color32::from_rgb(255, 180, 180),
        ),
        BannerKind::Rename => (
            egui::Color32::from_rgb(14, 48, 48),
            egui::Color32::from_rgb(70, 190, 190),
            egui::Color32::from_rgb(180, 240, 240),
        ),
    }
}

fn banner_hint(kind: BannerKind) -> &'static str {
    match kind {
        BannerKind::CargoRun => "confirm = run it, deny = don't",
        BannerKind::Execute => "confirm = run it, deny = don't",
        BannerKind::Delete => "confirm = delete it, deny = don't",
        BannerKind::Rename => "confirm = rename it, deny = don't",
    }
}

fn draw_banner(ui: &egui::Ui, rect: egui::Rect, kind: BannerKind) {
    let (bg, border, text) = banner_colors(kind);
    ui.painter().rect(
        rect,
        8.0,
        bg,
        egui::Stroke::new(2.0, border),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 16.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        banner_title(kind),
        egui::FontId::proportional(22.0),
        text,
    );
    ui.painter().text(
        egui::pos2(rect.right() - 16.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        banner_hint(kind),
        egui::FontId::proportional(14.0),
        text,
    );
}

pub struct NecessaryStructForEgui {
    shown: Option<Arc<Chain>>,
    shown_at: Instant,
    error_line: Option<String>,
    shown_step: usize,
    denied: Vec<bool>,
    visited: Vec<bool>,
    shown_surface: Option<Arc<SurfaceReview>>,
    surface_step: usize,
    menu_open: bool,
    prev_menu_open: bool,
    menu_folders: Vec<PathBuf>,
    menu_draft: String,
    menu_error: Option<String>,
}

impl Default for NecessaryStructForEgui {
    fn default() -> Self {
        Self {
            shown: None,
            shown_at: Instant::now(),
            error_line: None,
            shown_step: 0,
            denied: Vec::new(),
            visited: Vec::new(),
            shown_surface: None,
            surface_step: 0,
            menu_open: false,
            prev_menu_open: false,
            menu_folders: Vec::new(),
            menu_draft: String::new(),
            menu_error: None,
        }
    }
}

impl NecessaryStructForEgui {
    pub fn open_menu(&mut self) {
        self.menu_open = true;
        self.menu_folders = app_core::chain::whitelist::load();
        self.menu_draft.clear();
        self.menu_error = None;
    }
}

impl eframe::App for NecessaryStructForEgui {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.8]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if app_core::terminal_router::take_menu_request() {
            self.open_menu();
            // Even if the menu was already open, make sure the window is
            // visible and raised. wmctrl on a hidden window does nothing,
            // so Visible(true) has to go first.
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Visible(true));
            app_core::terminal_router::route_requests::focus_window();
        }

        let current_surface = app_core::terminal_router::current_surface_review();
        let surface_changed = match (&self.shown_surface, &current_surface) {
            (Some(a), Some(b)) => !Arc::ptr_eq(a, b),
            (None, None) => false,
            _ => true,
        };
        if surface_changed {
            self.shown_surface = current_surface;
            self.surface_step = 0;
            if self.shown_surface.is_some() {
                self.menu_open = false;
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Visible(true));
                app_core::terminal_router::route_requests::focus_window();
            }
        }

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
            self.shown_step = 0;
            self.denied = self
                .shown
                .as_ref()
                .map(|c| vec![false; c.steps.len()])
                .unwrap_or_default();
            self.visited = self
                .shown
                .as_ref()
                .map(|c| vec![false; c.steps.len()])
                .unwrap_or_default();
            if self.shown.is_some() {
                self.menu_open = false;
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Visible(true));
                app_core::terminal_router::route_requests::focus_window();
            }
        }

        if self.menu_open != self.prev_menu_open {
            let cmd = if self.menu_open {
                egui::ViewportCommand::Visible(true)
            } else {
                egui::ViewportCommand::Visible(false)
            };
            ui.ctx().send_viewport_cmd(cmd);
            self.prev_menu_open = self.menu_open;
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

        if self.shown_surface.is_some() {
            self.draw_surface_review(ui, screen);
            return;
        }
        if self.shown.is_none() && self.menu_open {
            self.draw_menu(ui, screen);
            return;
        }

        let margin = 40.0;
        let banner_h = 52.0;
        let caption_h = 28.0;
        let buttons_h = 64.0;
        let blocks_col_w = 56.0;
        let gap = 12.0;
        let button_size = 44.0;
        let block_size = 32.0;

        let chain = self.shown.clone();
        let step_count = chain.as_ref().map(|c| c.steps.len()).unwrap_or(0);
        let show_blocks = step_count > 1;
        let right_pad = if show_blocks { blocks_col_w + gap } else { 0.0 };

        let banner_kind = chain
            .as_ref()
            .and_then(|c| c.steps.get(self.shown_step))
            .and_then(|s| match s.step.verb {
                Verb::Remove => Some(BannerKind::Delete),
                Verb::Move => Some(BannerKind::Rename),
                _ => None,
            });
        let banner_offset = if banner_kind.is_some() { banner_h + gap } else { 0.0 };

        if let Some(kind) = banner_kind {
            let banner_rect = egui::Rect::from_min_max(
                egui::pos2(screen.left() + margin, screen.top() + margin),
                egui::pos2(screen.right() - margin, screen.top() + margin + banner_h),
            );
            draw_banner(ui, banner_rect, kind);
        }

        let caption_top = screen.top() + margin + banner_offset;
        let caption_rect = egui::Rect::from_min_max(
            egui::pos2(screen.left() + margin, caption_top),
            egui::pos2(screen.right() - margin, caption_top + caption_h),
        );

        let viewer_rect = egui::Rect::from_min_max(
            egui::pos2(
                screen.left() + margin,
                caption_top + caption_h + gap,
            ),
            egui::pos2(
                screen.right() - margin - right_pad,
                screen.bottom() - margin - buttons_h,
            ),
        );

        if let Some(c) = chain.as_ref() {
            if let Some(step) = c.steps.get(self.shown_step) {
                let denied = self.denied.get(self.shown_step).copied().unwrap_or(false);
                let verb = match step.step.verb {
                    Verb::Create => "create".to_string(),
                    Verb::Replace => "replace".to_string(),
                    Verb::Overwrite => "overwrite".to_string(),
                    Verb::Remove => "delete".to_string(),
                    Verb::Move => {
                        let dest = step
                            .dest
                            .as_ref()
                            .map(|d| d.display().to_string())
                            .unwrap_or_default();
                        let base = format!("rename {} \u{2192} {dest}", step.path.display());
                        match app_core::run_trust::trust_note(&step.path) {
                            Some(note) => format!("{base} \u{00b7} {note}"),
                            None => base,
                        }
                    }
                    Verb::Mkdir => "mkdir".to_string(),
                };
                let n = self.shown_step + 1;
                let m = step_count;
                let text = if denied {
                    format!("step {n} of {m} \u{00b7} {verb} {} \u{00b7} denied", step.path.display())
                } else {
                    format!("step {n} of {m} \u{00b7} {verb} {}", step.path.display())
                };
                let color = if denied { CAPTION_DENIED_COLOR } else { CAPTION_COLOR };
                ui.painter().text(
                    egui::pos2(caption_rect.left(), caption_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    text,
                    egui::FontId::monospace(13.0),
                    color,
                );
            }
        }

        let empty_rows: Vec<app_core::text_diff::DiffRowProcessedForUi> = Vec::new();
        let blocks: &[app_core::text_diff::DiffRowProcessedForUi] = chain
            .as_ref()
            .and_then(|c| c.steps.get(self.shown_step))
            .map(|s| s.rows.as_slice())
            .unwrap_or(&empty_rows);
        diff_viewer::diff_viewer_ui_no_copy(ui, viewer_rect, blocks);

        let mut clicked_step: Option<usize> = None;
        if show_blocks {
            let col_x = viewer_rect.right() + gap + blocks_col_w / 2.0;
            let total = step_count as f32 * block_size
                + (step_count.saturating_sub(1)) as f32 * gap;
            let start_y = viewer_rect.center().y - total / 2.0 + block_size / 2.0;
            for i in 0..step_count {
                let state = if self.denied.get(i).copied().unwrap_or(false) {
                    BlockState::Denied
                } else if i == self.shown_step {
                    BlockState::Selected
                } else {
                    BlockState::Normal
                };
                let y = start_y + i as f32 * (block_size + gap);
                if glass_button::glass_step_block_ui(ui, egui::pos2(col_x, y), i, state) {
                    clicked_step = Some(i);
                }
            }
        }
        if let Some(i) = clicked_step {
            self.shown_step = i;
            self.shown_at = Instant::now();
        }
        if let Some(slot) = self.visited.get_mut(self.shown_step) {
            *slot = true;
        }

        let close_margin = 20.0;
        let close_center = egui::pos2(screen.right() - close_margin, screen.top() + close_margin);
        if close_button::close_button_ui(ui, close_center) {
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

        let click_ok = self.shown_at.elapsed() >= CLICK_GUARD;
        let mut confirmed = false;
        let mut denied_click = false;
        if chain.is_some() {
            let total_w = 2.0 * button_size + gap;
            let start_x = viewer_rect.center().x - total_w / 2.0 + button_size / 2.0;
            let btn_y = screen.bottom() - margin - buttons_h / 2.0;
            if glass_button::glass_button_ui(
                ui,
                egui::pos2(start_x, btn_y),
                0,
                GlassIcon::Confirm,
            ) && click_ok
            {
                confirmed = true;
            }
            if glass_button::glass_button_ui(
                ui,
                egui::pos2(start_x + button_size + gap, btn_y),
                1,
                GlassIcon::Deny,
            ) && click_ok
            {
                denied_click = true;
            }
        }

        if denied_click {
            if let Some(c) = chain.clone() {
                if let Some(slot) = self.denied.get_mut(self.shown_step) {
                    *slot = true;
                }
                self.shown_at = Instant::now();
                let next = self
                    .denied
                    .iter()
                    .enumerate()
                    .skip(self.shown_step + 1)
                    .find(|(_, d)| !**d)
                    .map(|(i, _)| i)
                    .or_else(|| {
                        self.denied
                            .iter()
                            .enumerate()
                            .take(self.shown_step)
                            .find(|(_, d)| !**d)
                            .map(|(i, _)| i)
                    });
                match next {
                    Some(i) => {
                        self.shown_step = i;
                    }
                    None => {
                        let statuses = vec![StepStatus::Denied; c.steps.len()];
                        let rep = report::build(&c, &statuses, None);
                        let _ = app_core::clipboard::copy_to_clipboard(&rep);
                        popup::show("All denied");
                        app_core::terminal_router::clear_pending(&c);
                        self.shown = None;
                        self.error_line = None;
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    }
                }
            }
        }

        if confirmed {
            if let Some(c) = chain.clone() {
                let wanted: Vec<bool> = self.denied.iter().map(|d| !*d).collect();
                // A Remove or Move step the user never looked at blocks the
                // whole confirm. Jump to it and say so.
                let unvisited_destructive = c
                    .steps
                    .iter()
                    .enumerate()
                    .position(|(i, s)| {
                        wanted.get(i).copied().unwrap_or(true)
                            && matches!(s.step.verb, Verb::Remove | Verb::Move)
                            && !self.visited.get(i).copied().unwrap_or(false)
                    });
                if let Some(i) = unvisited_destructive {
                    self.shown_step = i;
                    self.shown_at = Instant::now();
                    self.error_line = Some(format!(
                        "look at step {} before confirming; it deletes or renames a file",
                        i + 1
                    ));
                    return;
                }
                match writer::apply(&c, &wanted) {
                    Ok(()) => {
                        refresh_trust_after_chain_confirm(&c);
                        let statuses: Vec<StepStatus> = wanted
                            .iter()
                            .map(|w| {
                                if *w {
                                    StepStatus::Applied
                                } else {
                                    StepStatus::Denied
                                }
                            })
                            .collect();
                        let rep = report::build(&c, &statuses, None);
                        let _ = app_core::clipboard::copy_to_clipboard(&rep);
                        let applied = wanted.iter().filter(|w| **w).count();
                        popup::show(&format!("Applied {} step(s)", applied));
                        app_core::terminal_router::clear_pending(&c);
                        self.shown = None;
                        self.error_line = None;
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    }
                    Err(reason) => {
                        let statuses: Vec<StepStatus> = wanted
                            .iter()
                            .map(|w| {
                                if *w {
                                    StepStatus::Pending
                                } else {
                                    StepStatus::Denied
                                }
                            })
                            .collect();
                        let rep = report::build(&c, &statuses, Some(reason.as_str()));
                        self.error_line = Some(rep.clone());
                        let _ = app_core::clipboard::copy_to_clipboard(&rep);
                        popup::show("Edit failed");
                    }
                }
            }
        }
    }
}

impl NecessaryStructForEgui {
    fn draw_surface_review(&mut self, ui: &mut egui::Ui, screen: egui::Rect) {
        let Some(review) = self.shown_surface.clone() else {
            return;
        };

        let margin = 40.0;
        let banner_h = 52.0;
        let caption_h = 28.0;
        let buttons_h = 64.0;
        let blocks_col_w = 56.0;
        let gap = 12.0;
        let button_size = 44.0;
        let block_size = 32.0;

        let file_count = review.files.len();
        let show_blocks = file_count > 1;
        let right_pad = if show_blocks { blocks_col_w + gap } else { 0.0 };

        let banner_rect = egui::Rect::from_min_max(
            egui::pos2(screen.left() + margin, screen.top() + margin),
            egui::pos2(screen.right() - margin, screen.top() + margin + banner_h),
        );
        draw_banner(ui, banner_rect, review.kind);

        let caption_top = screen.top() + margin + banner_h + gap;
        let caption_rect = egui::Rect::from_min_max(
            egui::pos2(screen.left() + margin, caption_top),
            egui::pos2(screen.right() - margin, caption_top + caption_h),
        );

        let viewer_rect = egui::Rect::from_min_max(
            egui::pos2(
                screen.left() + margin,
                caption_top + caption_h + gap,
            ),
            egui::pos2(
                screen.right() - margin - right_pad,
                screen.bottom() - margin - buttons_h,
            ),
        );

        if let Some(f) = review.files.get(self.surface_step) {
            let n = self.surface_step + 1;
            let text = format!(
                "{} \u{00b7} {n} of {file_count} \u{00b7} {}",
                review.caption,
                f.path.display()
            );
            ui.painter().text(
                egui::pos2(caption_rect.left(), caption_rect.center().y),
                egui::Align2::LEFT_CENTER,
                text,
                egui::FontId::monospace(12.0),
                CAPTION_COLOR,
            );
        }

        let empty_rows: Vec<app_core::text_diff::DiffRowProcessedForUi> = Vec::new();
        let blocks: &[app_core::text_diff::DiffRowProcessedForUi] = review
            .files
            .get(self.surface_step)
            .map(|f| f.rows.as_slice())
            .unwrap_or(&empty_rows);
        diff_viewer::diff_viewer_ui_no_copy(ui, viewer_rect, blocks);

        if show_blocks {
            let col_x = viewer_rect.right() + gap + blocks_col_w / 2.0;
            let total = file_count as f32 * block_size
                + (file_count.saturating_sub(1)) as f32 * gap;
            let start_y = viewer_rect.center().y - total / 2.0 + block_size / 2.0;
            for i in 0..file_count {
                let state = if i == self.surface_step {
                    BlockState::Selected
                } else {
                    BlockState::Normal
                };
                let y = start_y + i as f32 * (block_size + gap);
                if glass_button::glass_step_block_ui(ui, egui::pos2(col_x, y), 1000 + i, state) {
                    self.surface_step = i;
                }
            }
        }

        let close_margin = 20.0;
        let close_center = egui::pos2(screen.right() - close_margin, screen.top() + close_margin);
        if close_button::close_button_ui(ui, close_center) {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        let total_w = 2.0 * button_size + gap;
        let start_x = viewer_rect.center().x - total_w / 2.0 + button_size / 2.0;
        let btn_y = screen.bottom() - margin - buttons_h / 2.0;
        let mut confirm_clicked = false;
        let mut deny_clicked = false;
        if glass_button::glass_button_ui(ui, egui::pos2(start_x, btn_y), 0, GlassIcon::Confirm) {
            confirm_clicked = true;
        }
        if glass_button::glass_button_ui(
            ui,
            egui::pos2(start_x + button_size + gap, btn_y),
            1,
            GlassIcon::Deny,
        ) {
            deny_clicked = true;
        }

        if confirm_clicked {
            review.respond(true);
            app_core::terminal_router::clear_surface_review(&review);
            self.shown_surface = None;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Visible(false));
        } else if deny_clicked {
            review.respond(false);
            app_core::terminal_router::clear_surface_review(&review);
            self.shown_surface = None;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
    }

    fn draw_menu(&mut self, ui: &mut egui::Ui, screen: egui::Rect) {
        let close_margin = 20.0;
        let close_center = egui::pos2(screen.right() - close_margin, screen.top() + close_margin);
        if close_button::close_button_ui(ui, close_center) {
            self.menu_open = false;
        }

        let rect = egui::Rect::from_min_max(
            egui::pos2(screen.left() + 60.0, screen.top() + 60.0),
            egui::pos2(screen.right() - 60.0, screen.bottom() - 60.0),
        );

        ui.painter().rect(
            rect.expand(24.0),
            12.0,
            egui::Color32::from_rgb(24, 24, 28),
            egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
            egui::StrokeKind::Inside,
        );

        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
        );
        let ui = &mut child;

        ui.heading("Whitelisted folders");
        ui.add_space(6.0);
        ui.label("Files inside these folders are applied without confirmation.");
        ui.add_space(14.0);

        let mut to_remove: Option<usize> = None;
        let mut to_add: Option<PathBuf> = None;
        if self.menu_folders.is_empty() {
            ui.label(egui::RichText::new("(none)").italics());
        }
        for (i, folder) in self.menu_folders.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(folder.display().to_string());
                if ui.button("remove").clicked() {
                    to_remove = Some(i);
                }
            });
        }

        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.label("add:");
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.menu_draft)
                    .desired_width(500.0)
                    .hint_text("/absolute/path/to/folder"),
            );
            let submitted = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.button("add").clicked() || submitted {
                let raw = self.menu_draft.trim().to_string();
                if !raw.is_empty() {
                    to_add = Some(PathBuf::from(raw));
                }
            }
        });

        if let Some(err) = &self.menu_error {
            ui.add_space(10.0);
            ui.colored_label(ERROR_COLOR, err);
        }

        if let Some(i) = to_remove {
            self.menu_folders.remove(i);
            match app_core::chain::whitelist::save(&self.menu_folders) {
                Ok(()) => self.menu_error = None,
                Err(e) => self.menu_error = Some(e),
            }
        }

        if let Some(path) = to_add {
            match app_core::chain::whitelist::validate_add(&path) {
                Ok(canon) => {
                    if self.menu_folders.contains(&canon) {
                        self.menu_error = Some("already in the list".to_string());
                    } else {
                        self.menu_folders.push(canon);
                        match app_core::chain::whitelist::save(&self.menu_folders) {
                            Ok(()) => {
                                self.menu_error = None;
                                self.menu_draft.clear();
                            }
                            Err(e) => self.menu_error = Some(e),
                        }
                    }
                }
                Err(e) => self.menu_error = Some(e),
            }
        }
    }
}

pub struct CtxForActuallyDrawing<'a> {
    pub ui: &'a mut egui::Ui,
}

/// After a chain is confirmed in review, update the trust record for any
/// already-trusted project whose surface files the chain wrote.
fn refresh_trust_after_chain_confirm(chain: &Chain) {
    use std::collections::HashSet;

    let has_move_or_remove = chain
        .steps
        .iter()
        .any(|s| matches!(s.step.verb, Verb::Move | Verb::Remove));

    // Rename / remove bookkeeping for Move and Remove steps.
    for step in &chain.steps {
        match step.step.verb {
            Verb::Move => {
                if let Some(dest) = &step.dest {
                    app_core::run_trust::rename(&step.path, dest);
                    let from_root = app_core::surface::project_root(&step.path);
                    let to_root = app_core::surface::project_root(dest);
                    match (from_root, to_root) {
                        (Some(fr), Some(tr)) if fr == tr => {
                            app_core::trust::rename_in_record(&fr, &step.path, dest);
                        }
                        (Some(fr), _) => {
                            app_core::trust::remove_from_record(&fr, &step.path);
                        }
                        _ => {}
                    }
                }
            }
            Verb::Remove => {
                app_core::run_trust::remove_all_at_path(&step.path);
                if let Some(root) = app_core::surface::project_root(&step.path) {
                    app_core::trust::remove_from_record(&root, &step.path);
                }
            }
            _ => {}
        }
    }

    // A chain with any Move or Remove skips the content refresh, because
    // moved content never appeared as a diff.
    if has_move_or_remove {
        return;
    }

    let mut roots: HashSet<PathBuf> = HashSet::new();
    for snap in &chain.snapshots {
        if let Some(root) = app_core::surface::project_root(&snap.path) {
            roots.insert(root);
        }
    }

    for root in roots {
        let Some(mut record) = app_core::trust::load(&root) else {
            continue;
        };
        let written: Vec<(PathBuf, String)> = chain
            .snapshots
            .iter()
            .filter(|s| app_core::surface::project_root(&s.path).as_deref() == Some(root.as_path()))
            .filter_map(|s| {
                std::fs::read_to_string(&s.path)
                    .ok()
                    .map(|text| (s.path.clone(), text))
            })
            .collect();
        if app_core::trust::refresh_after_chain_confirm(&mut record, &written) {
            let _ = app_core::trust::save(&record);
        }
    }
}
