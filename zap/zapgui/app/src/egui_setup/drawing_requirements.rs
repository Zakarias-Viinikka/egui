use eframe::egui;
use egui::text::{LayoutJob, TextWrapping};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use app_core::chain::{popup, report, writer, Chain, ChainOutcome, StepStatus, Verb};
use app_core::terminal_router::surface_review::SurfaceReview;
use every_page_ever::ReusableComponents::close_button::ui as close_button;
use every_page_ever::ReusableComponents::diff_viewer::ui as diff_viewer;
use every_page_ever::ReusableComponents::glass_button::ui::{
    self as glass_button, BlockState, Icon as GlassIcon,
};

const CLICK_GUARD: Duration = Duration::from_millis(150);
const ERROR_COLOR: egui::Color32 = egui::Color32::from_rgb(210, 80, 80);
const CAPTION_COLOR: egui::Color32 = egui::Color32::from_rgb(220, 220, 220);
const CAPTION_MARKED_COLOR: egui::Color32 = egui::Color32::from_rgb(230, 180, 90);
const BOUNDARY_GREEN: egui::Color32 = egui::Color32::from_rgb(100, 200, 120);
const BOUNDARY_AMBER: egui::Color32 = egui::Color32::from_rgb(230, 180, 90);
const BOUNDARY_ORANGE: egui::Color32 = egui::Color32::from_rgb(240, 110, 40);
const CHIP_PAD_X: f32 = 8.0;
const CHIP_PAD_Y: f32 = 3.0;
const CLOSE_TIP: &str = "Hide this window. The waiting chain stays waiting.";
const PILL_TIP: &str = "Switch between the changed lines and the whole file.";
const SURFACE_BANNER_BG: egui::Color32 = egui::Color32::from_rgb(60, 42, 12);
const SURFACE_BANNER_BORDER: egui::Color32 = egui::Color32::from_rgb(230, 160, 40);
const SURFACE_BANNER_TEXT: egui::Color32 = egui::Color32::from_rgb(255, 200, 110);
const CAPTION_MIN_H: f32 = 28.0;
const PILL_RESERVE: f32 = 110.0;

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

fn boundary_chip_text(boundary: &std::path::Path) -> String {
    if boundary == std::path::Path::new("/") {
        "edits span the whole disk".to_string()
    } else {
        format!("confined to {}/", boundary.display())
    }
}

fn boundary_chip_color(boundary: &std::path::Path) -> egui::Color32 {
    if boundary == std::path::Path::new("/") {
        return BOUNDARY_ORANGE;
    }
    let normal = boundary
        .components()
        .filter(|c| matches!(c, std::path::Component::Normal(_)))
        .count();
    if normal <= 2 {
        BOUNDARY_AMBER
    } else {
        BOUNDARY_GREEN
    }
}

fn chip_galley(
    ui: &egui::Ui,
    text: &str,
    color: egui::Color32,
    max_w: f32,
) -> Arc<egui::Galley> {
    let font = egui::FontId::monospace(11.0);
    let avail = (max_w - CHIP_PAD_X * 2.0).max(24.0);
    let full = ui
        .painter()
        .layout_no_wrap(text.to_string(), font.clone(), color);
    if full.size().x <= avail {
        return full;
    }
    let chars: Vec<char> = text.chars().collect();
    for k in 1..chars.len() {
        let candidate: String = std::iter::once('\u{2026}')
            .chain(chars[k..].iter().copied())
            .collect();
        let g = ui
            .painter()
            .layout_no_wrap(candidate, font.clone(), color);
        if g.size().x <= avail {
            return g;
        }
    }
    ui.painter()
        .layout_no_wrap("\u{2026}".to_string(), font, color)
}

fn verb_word(v: Verb) -> &'static str {
    match v {
        Verb::Create => "create",
        Verb::Replace => "replace",
        Verb::Overwrite => "overwrite",
        Verb::Append => "append",
        Verb::Remove => "delete",
        Verb::RemoveDir => "rmdir",
        Verb::Cargo => "cargo",
        Verb::Run => "run",
        Verb::Move => "rename",
        Verb::Mkdir => "mkdir",
    }
}

fn verb_accent(v: Verb) -> egui::Color32 {
    match v {
        Verb::Create | Verb::Mkdir => egui::Color32::from_rgb(100, 200, 120),
        Verb::Replace | Verb::Overwrite | Verb::Append => egui::Color32::from_rgb(100, 160, 220),
        Verb::Remove | Verb::RemoveDir => egui::Color32::from_rgb(210, 80, 80),
        Verb::Move => egui::Color32::from_rgb(90, 200, 220),
        Verb::Cargo => egui::Color32::from_rgb(230, 160, 40),
        Verb::Run => egui::Color32::from_rgb(200, 90, 220),
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

/// Rects shared by the chain page and the surface review page. Built once
/// per frame so the caption can grow when a path wraps.
struct PageLayout {
    banner: Option<egui::Rect>,
    caption: egui::Rect,
    viewer: egui::Rect,
    button_y: f32,
    button_size: f32,
    block_size: f32,
    block_col_x: f32,
    gap: f32,
    close_center: egui::Pos2,
    error_rect: egui::Rect,
    pill_center: egui::Pos2,
}

impl PageLayout {
    fn new(screen: egui::Rect, has_banner: bool, has_blocks: bool, caption_h: f32) -> Self {
        let margin = 40.0;
        let banner_h = 52.0;
        let buttons_h = 64.0;
        let blocks_col_w: f32 = 56.0;
        let gap = 12.0;
        let button_size = 44.0;
        let block_size = 32.0;

        let right_pad = if has_blocks { blocks_col_w + gap } else { 0.0 };
        let banner_offset = if has_banner { banner_h + gap } else { 0.0 };

        let banner = if has_banner {
            Some(egui::Rect::from_min_max(
                egui::pos2(screen.left() + margin, screen.top() + margin),
                egui::pos2(screen.right() - margin, screen.top() + margin + banner_h),
            ))
        } else {
            None
        };

        let caption_top = screen.top() + margin + banner_offset;
        let caption = egui::Rect::from_min_max(
            egui::pos2(screen.left() + margin, caption_top),
            egui::pos2(screen.right() - margin - right_pad, caption_top + caption_h),
        );

        let viewer = egui::Rect::from_min_max(
            egui::pos2(screen.left() + margin, caption_top + caption_h + gap),
            egui::pos2(
                screen.right() - margin - right_pad,
                screen.bottom() - margin - buttons_h,
            ),
        );

        let block_col_x = viewer.right() + gap + blocks_col_w / 2.0;
        let button_y = screen.bottom() - margin - buttons_h / 2.0;
        let close_center = egui::pos2(screen.right() - 20.0, screen.top() + 20.0);
        let error_rect = egui::Rect::from_min_max(
            egui::pos2(screen.left() + margin, screen.bottom() - 46.0),
            egui::pos2(screen.right() - margin, screen.bottom() - 16.0),
        );
        let pill_center = egui::pos2(caption.right() - 50.0, caption.center().y);

        Self {
            banner,
            caption,
            viewer,
            button_y,
            button_size,
            block_size,
            block_col_x,
            gap,
            close_center,
            error_rect,
            pill_center,
        }
    }
}

fn caption_wrap_width(screen: egui::Rect, has_blocks: bool) -> f32 {
    let margin = 40.0;
    let blocks_col_w = 56.0;
    let gap = 12.0;
    let right_pad = if has_blocks { blocks_col_w + gap } else { 0.0 };
    ((screen.right() - margin - right_pad) - (screen.left() + margin) - PILL_RESERVE).max(120.0)
}

fn caption_area(screen: egui::Rect, has_blocks: bool) -> (f32, f32) {
    let margin = 40.0;
    let blocks_col_w = 56.0;
    let gap = 12.0;
    let right_pad = if has_blocks { blocks_col_w + gap } else { 0.0 };
    (screen.left() + margin, screen.right() - margin - right_pad)
}

fn pill_width(ui: &egui::Ui, label: &str) -> f32 {
    let font = egui::FontId::monospace(12.0);
    let text_w = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, egui::Color32::WHITE)
        .size()
        .x;
    text_w + 20.0
}

fn layout_caption(
    ui: &egui::Ui,
    text: &str,
    color: egui::Color32,
    wrap_width: f32,
) -> Arc<egui::Galley> {
    let mut job = LayoutJob::single_section(
        text.to_string(),
        egui::TextFormat {
            font_id: egui::FontId::monospace(13.0),
            color,
            ..Default::default()
        },
    );
    job.wrap = TextWrapping {
        max_width: wrap_width,
        ..Default::default()
    };
    ui.fonts_mut(|f| f.layout_job(job))
}


pub struct NecessaryStructForEgui {
    shown: Option<Arc<Chain>>,
    shown_at: Instant,
    error_line: Option<String>,
    shown_step: usize,
    marked: Vec<bool>,
    visited: Vec<bool>,
    verified: Vec<bool>,
    shown_surface: Option<Arc<SurfaceReview>>,
    surface_step: usize,
    menu_open: bool,
    prev_menu_open: bool,
    menu_folders: Vec<PathBuf>,
    menu_draft: String,
    menu_error: Option<String>,
    /// When true, the viewer shows every line of the step instead of the
    /// snippet around the change.
    full_file: bool,
    /// Set for one frame when the pill is clicked, so the viewer scrolls
    /// to the first change after switching modes.
    scroll_to_change_pending: bool,
    surface_shown_at: Instant,
    did_init_style: bool,
}

impl Default for NecessaryStructForEgui {
    fn default() -> Self {
        Self {
            shown: None,
            shown_at: Instant::now(),
            error_line: None,
            shown_step: 0,
            marked: Vec::new(),
            visited: Vec::new(),
            verified: Vec::new(),
            shown_surface: None,
            surface_step: 0,
            menu_open: false,
            prev_menu_open: false,
            menu_folders: Vec::new(),
            menu_draft: String::new(),
            menu_error: None,
            full_file: false,
            scroll_to_change_pending: false,
            surface_shown_at: Instant::now(),
            did_init_style: false,
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
        if !self.did_init_style {
            self.did_init_style = true;
            ui.ctx()
                .all_styles_mut(|style| style.interaction.tooltip_delay = 0.15);
        }

        if app_core::terminal_router::take_menu_request() {
            self.open_menu();
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Visible(true));
            app_core::terminal_router::route_requests::focus_window();
        }

        if app_core::terminal_router::take_show_request() {
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
            let was_shown = self.shown_surface.is_some();
            let now_none = current_surface.is_none();
            self.shown_surface = current_surface;
            self.surface_step = 0;
            self.surface_shown_at = Instant::now();
            self.full_file = false;
            if was_shown && now_none {
                // The review was withdrawn by its own handler (cancel,
                // timeout) without the UI having cleared it. Hide.
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
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
            // If the slot went from Some to None without the UI having
            // cleared it, the chain-run handler withdrew it. Hide.
            let was_shown = self.shown.is_some();
            let now_none = current.is_none();
            self.shown = current;
            if was_shown && now_none {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
            self.shown_at = Instant::now();
            self.error_line = None;
            self.shown_step = 0;
            self.full_file = false;
            self.marked = self
                .shown
                .as_ref()
                .map(|c| vec![false; c.steps.len()])
                .unwrap_or_default();
            self.visited = self
                .shown
                .as_ref()
                .map(|c| vec![false; c.steps.len()])
                .unwrap_or_default();
            self.verified = self
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

        let chain = self.shown.clone();
        let step_count = chain.as_ref().map(|c| c.steps.len()).unwrap_or(0);
        let show_blocks = step_count > 1;

        let banner_kind = chain
            .as_ref()
            .and_then(|c| c.steps.get(self.shown_step))
            .and_then(|s| match s.step.verb {
                Verb::Remove => Some(BannerKind::Delete),
                Verb::RemoveDir => Some(BannerKind::Delete),
                Verb::Move => Some(BannerKind::Rename),
                Verb::Cargo => Some(BannerKind::CargoRun),
                Verb::Run => Some(BannerKind::Execute),
                _ => None,
            });
        let has_banner = banner_kind.is_some();

        let pill_label = if self.full_file {
            "changes only"
        } else {
            "full file"
        };
        let (cap_left, cap_right) = caption_area(screen, show_blocks);
        let cap_area_w = cap_right - cap_left;
        let pill_w = pill_width(ui, pill_label);
        let pill_left = cap_right - 50.0 - pill_w / 2.0;
        let chip_right = pill_left - 8.0;
        let chip_max_w = cap_area_w * 0.45;
        let chip = chain.as_ref().map(|c| {
            let text = boundary_chip_text(&c.boundary);
            let color = boundary_chip_color(&c.boundary);
            let g = chip_galley(ui, &text, color, chip_max_w);
            (g, color)
        });
        let chip_w = chip
            .as_ref()
            .map(|(g, _)| g.size().x + CHIP_PAD_X * 2.0)
            .unwrap_or(0.0);
        let wrap_width = if chip.is_some() {
            (chip_right - chip_w - 12.0 - cap_left).max(120.0)
        } else {
            caption_wrap_width(screen, show_blocks)
        };

        let (caption_galley, caption_color) = if let Some(c) = chain.as_ref() {
            if let Some(step) = c.steps.get(self.shown_step) {
                let marked = self.marked.get(self.shown_step).copied().unwrap_or(false);
                let verified = self.verified.get(self.shown_step).copied().unwrap_or(false);
                let verb = match step.step.verb {
                    Verb::Create => "create".to_string(),
                    Verb::Replace => "replace".to_string(),
                    Verb::Overwrite => "overwrite".to_string(),
                    Verb::Append => "append".to_string(),
                    Verb::Remove => "delete".to_string(),
                    Verb::RemoveDir => "rmdir".to_string(),
                    Verb::Cargo => "cargo".to_string(),
                    Verb::Run => "run".to_string(),
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
                let suffix = if marked {
                    " \u{00b7} marked for retry"
                } else if verified {
                    " \u{00b7} verified"
                } else {
                    ""
                };
                let text = format!(
                    "step {n} of {m} \u{00b7} {verb} {}{suffix}",
                    step.path.display()
                );
                let color = if marked {
                    CAPTION_MARKED_COLOR
                } else {
                    CAPTION_COLOR
                };
                let g = layout_caption(ui, &text, color, wrap_width);
                (Some(g), color)
            } else {
                (None, CAPTION_COLOR)
            }
        } else {
            (None, CAPTION_COLOR)
        };

        let measured_caption_h = caption_galley
            .as_ref()
            .map(|g| g.size().y)
            .unwrap_or(CAPTION_MIN_H)
            .max(CAPTION_MIN_H);

        let layout = PageLayout::new(screen, has_banner, show_blocks, measured_caption_h);

        if let (Some(kind), Some(banner_rect)) = (banner_kind, layout.banner) {
            draw_banner(ui, banner_rect, kind);
        }

        let caption_accent = chain
            .as_ref()
            .and_then(|c| c.steps.get(self.shown_step))
            .map(|s| verb_accent(s.step.verb));

        if let Some(accent) = caption_accent {
            let bar = egui::Rect::from_min_size(
                egui::pos2(layout.caption.left() - 8.0, layout.caption.top() + 4.0),
                egui::vec2(3.0, (measured_caption_h - 8.0).max(4.0)),
            );
            ui.painter().rect_filled(bar, 1.0, accent);
        }

        if let Some(g) = caption_galley {
            ui.painter()
                .galley(layout.caption.left_top(), g, caption_color);
        }

        if let Some((g, color)) = chip {
            let size = g.size() + egui::vec2(CHIP_PAD_X * 2.0, CHIP_PAD_Y * 2.0);
            let rect = egui::Rect::from_min_size(
                egui::pos2(
                    chip_right - size.x,
                    layout.caption.center().y - size.y / 2.0,
                ),
                size,
            );
            ui.painter().rect(
                rect,
                size.y / 2.0,
                egui::Color32::TRANSPARENT,
                egui::Stroke::new(1.0, color),
                egui::StrokeKind::Inside,
            );
            ui.painter()
                .galley(rect.min + egui::vec2(CHIP_PAD_X, CHIP_PAD_Y), g, color);
        }

        if chain.is_some() {
            if glass_button::pill_ui(
                ui,
                layout.pill_center,
                "full_file_toggle",
                pill_label,
                self.full_file,
                PILL_TIP,
            ) {
                self.full_file = !self.full_file;
                self.scroll_to_change_pending = true;
                self.shown_at = Instant::now();
            }
        }

        let owned_blocks;
        let blocks: &[app_core::text_diff::DiffRowProcessedForUi] = if self.full_file {
            if let Some(c) = chain.as_ref() {
                if let Some(step) = c.steps.get(self.shown_step) {
                    let before = step.before.as_deref().unwrap_or("");
                    owned_blocks = app_core::text_diff::process(
                        &app_core::text_diff::pair_lines_full(before, &step.after),
                    );
                    owned_blocks.as_slice()
                } else {
                    &[]
                }
            } else {
                &[]
            }
        } else {
            chain
                .as_ref()
                .and_then(|c| c.steps.get(self.shown_step))
                .map(|s| s.rows.as_slice())
                .unwrap_or(&[])
        };

        let move_spans: &[app_core::text_diff::MoveSpan] = chain
            .as_ref()
            .and_then(|c| c.steps.get(self.shown_step))
            .map(|s| s.move_spans.as_slice())
            .unwrap_or(&[]);

        let scroll_once = self.scroll_to_change_pending;
        self.scroll_to_change_pending = false;
        let right_accent = chain
            .as_ref()
            .and_then(|c| c.steps.get(self.shown_step))
            .and_then(|s| match s.step.verb {
                Verb::Run | Verb::Cargo => Some(egui::Color32::from_rgb(200, 90, 220)),
                _ => None,
            });

        let moves_mode = if self.full_file {
            diff_viewer::MoveDisplay::Classic
        } else {
            diff_viewer::MoveDisplay::Dimmed
        };
        diff_viewer::diff_viewer_ui_no_copy(
            ui,
            layout.viewer,
            blocks,
            diff_viewer::DiffViewerParams {
                move_spans,
                scroll_id: egui::Id::new(("diff_scroll", self.shown_step, self.full_file)),
                scroll_to_change_once: scroll_once,
                right_accent,
                moves: moves_mode,
            },
        );

        let mut clicked_step: Option<usize> = None;
        if show_blocks {
            let total = step_count as f32 * layout.block_size
                + (step_count.saturating_sub(1)) as f32 * layout.gap;
            let start_y = layout.viewer.center().y - total / 2.0 + layout.block_size / 2.0;
            for i in 0..step_count {
                let marked = self.marked.get(i).copied().unwrap_or(false);
                let shown = i == self.shown_step;
                let state = match (marked, shown) {
                    (true, true) => BlockState::MarkedSelected,
                    (true, false) => BlockState::Marked,
                    (false, true) => BlockState::Selected,
                    (false, false) => BlockState::Normal,
                };
                let accent = chain
                    .as_ref()
                    .and_then(|c| c.steps.get(i))
                    .map(|s| verb_accent(s.step.verb));
                let step_ref = chain.as_ref().and_then(|c| c.steps.get(i));
                let mut tip = if let Some(s) = step_ref {
                    let file_name = s
                        .path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| s.path.display().to_string());
                    format!(
                        "Step {} of {}: {} {}",
                        i + 1,
                        step_count,
                        verb_word(s.step.verb),
                        file_name
                    )
                } else {
                    format!("Step {} of {}", i + 1, step_count)
                };
                if marked {
                    tip.push_str(", marked");
                }
                if self.verified.get(i).copied().unwrap_or(false) {
                    tip.push_str(", verified");
                }
                let y = start_y + i as f32 * (layout.block_size + layout.gap);
                if glass_button::glass_step_block_ui(
                    ui,
                    egui::pos2(layout.block_col_x, y),
                    i,
                    state,
                    accent,
                    &tip,
                ) {
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

        if close_button::close_button_ui(ui, layout.close_center, CLOSE_TIP) {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        if let Some(line) = &self.error_line {
            ui.put(
                layout.error_rect,
                egui::Label::new(egui::RichText::new(line.as_str()).color(ERROR_COLOR)).wrap(),
            );
        }

        let click_ok = self.shown_at.elapsed() >= CLICK_GUARD;
        let shown_is_run = chain
            .as_ref()
            .and_then(|c| c.steps.get(self.shown_step))
            .map(|s| matches!(s.step.verb, Verb::Cargo | Verb::Run))
            .unwrap_or(false);
        let mut confirmed = false;
        let mut mark_clicked = false;
        let mut deny_all_clicked = false;
        let mut verify_clicked = false;
        if chain.is_some() {
            let total_w = 4.0 * layout.button_size + 3.0 * layout.gap;
            let start_x =
                layout.viewer.center().x - total_w / 2.0 + layout.button_size / 2.0;
            let step_x = layout.button_size + layout.gap;
            if glass_button::glass_button_ui(
                ui,
                egui::pos2(start_x, layout.button_y),
                0,
                GlassIcon::Verify,
                "Tick this once you have read the step. Run and cargo steps cannot be confirmed until verified.",
            ) {
                verify_clicked = true;
            }
            if glass_button::glass_button_ui(
                ui,
                egui::pos2(start_x + step_x, layout.button_y),
                1,
                GlassIcon::Mark,
                "Leave this step out. Confirm skips marked steps and keeps them for retry.",
            ) {
                mark_clicked = true;
            }
            let approve_icon = if shown_is_run {
                GlassIcon::AllowRun
            } else {
                GlassIcon::Confirm
            };
            let approve_tip = if shown_is_run {
                "Write the steps, then run the command in this step."
            } else {
                "Write every step that is not marked."
            };
            if glass_button::glass_button_ui(
                ui,
                egui::pos2(start_x + 2.0 * step_x, layout.button_y),
                2,
                approve_icon,
                approve_tip,
            ) && click_ok
            {
                confirmed = true;
            }
            if glass_button::glass_button_ui(
                ui,
                egui::pos2(start_x + 3.0 * step_x, layout.button_y),
                3,
                GlassIcon::Deny,
                "Write nothing. All steps stay available for retry.",
            ) && click_ok
            {
                deny_all_clicked = true;
            }
        }

        if verify_clicked {
            if let Some(slot) = self.verified.get_mut(self.shown_step) {
                *slot = !*slot;
                self.shown_at = Instant::now();
            }
        }

        if mark_clicked {
            if let Some(slot) = self.marked.get_mut(self.shown_step) {
                *slot = !*slot;
                self.shown_at = Instant::now();
            }
        }

        if deny_all_clicked {
            if let Some(c) = chain.clone() {
                let statuses: Vec<StepStatus> = self
                    .marked
                    .iter()
                    .map(|m| {
                        if *m {
                            StepStatus::Marked
                        } else {
                            StepStatus::Denied
                        }
                    })
                    .collect();
                app_core::terminal_router::retry::save_from_chain(&c, &statuses);
                let rep = report::build(&c, &statuses, None);
                popup::show("Denied all");
                let _ = app_core::clipboard::copy_to_clipboard(&rep);
                app_core::terminal_router::clear_pending(&c);
                c.finish(ChainOutcome::AllDenied);
                self.shown = None;
                self.error_line = None;
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
        }

        if confirmed {
            if let Some(c) = chain.clone() {
                let wanted: Vec<bool> = self.marked.iter().map(|m| !*m).collect();
                let unapproved_special = c.steps.iter().enumerate().position(|(i, s)| {
                    let wanted = wanted.get(i).copied().unwrap_or(true);
                    if !wanted {
                        return false;
                    }
                    let is_special = matches!(
                        s.step.verb,
                        Verb::Remove | Verb::Move | Verb::RemoveDir
                    );
                    let is_run = matches!(s.step.verb, Verb::Cargo | Verb::Run);
                    let verified = self.verified.get(i).copied().unwrap_or(false);
                    if is_run {
                        !verified
                    } else if is_special {
                        !self.visited.get(i).copied().unwrap_or(false)
                    } else {
                        false
                    }
                });
                if let Some(i) = unapproved_special {
                    self.shown_step = i;
                    self.shown_at = Instant::now();
                    let verb_word = match c.steps[i].step.verb {
                        Verb::Cargo | Verb::Run => "run a command",
                        _ => "delete or rename a file",
                    };
                    self.error_line = Some(format!(
                        "step {} needs Verify before confirming; it can {verb_word}",
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
                                    StepStatus::Marked
                                }
                            })
                            .collect();
                        app_core::terminal_router::retry::save_from_chain(&c, &statuses);
                        let rep = report::build(&c, &statuses, None);
                        let applied = wanted.iter().filter(|w| **w).count();
                        popup::show(&format!("Applied {} step(s)", applied));
                        let _ = app_core::clipboard::copy_to_clipboard(&rep);
                        app_core::terminal_router::clear_pending(&c);
                        c.finish(ChainOutcome::Applied(wanted.clone()));
                        self.shown = None;
                        self.error_line = None;
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    }
                    Err(reason) => {
                        let retry_statuses: Vec<StepStatus> = wanted
                            .iter()
                            .map(|w| {
                                if *w {
                                    StepStatus::Failed
                                } else {
                                    StepStatus::Denied
                                }
                            })
                            .collect();
                        app_core::terminal_router::retry::save_from_chain(&c, &retry_statuses);
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
                        popup::show("Edit failed");
                        let _ = app_core::clipboard::copy_to_clipboard(&rep);
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

        let file_count = review.files.len();
        let show_blocks = file_count > 1;

        let wrap_width = caption_wrap_width(screen, show_blocks);

        let (caption_galley, caption_color) = if let Some(f) = review.files.get(self.surface_step)
        {
            let n = self.surface_step + 1;
            let text = format!(
                "{} \u{00b7} {n} of {file_count} \u{00b7} {}",
                review.caption,
                f.path.display()
            );
            let g = layout_caption(ui, &text, CAPTION_COLOR, wrap_width);
            (Some(g), CAPTION_COLOR)
        } else {
            (None, CAPTION_COLOR)
        };

        let measured_caption_h = caption_galley
            .as_ref()
            .map(|g| g.size().y)
            .unwrap_or(CAPTION_MIN_H)
            .max(CAPTION_MIN_H);

        let layout = PageLayout::new(screen, true, show_blocks, measured_caption_h);

        if let Some(banner_rect) = layout.banner {
            draw_banner(ui, banner_rect, review.kind);
        }

        if let Some(g) = caption_galley {
            ui.painter()
                .galley(layout.caption.left_top(), g, caption_color);
        }

        let label = if self.full_file {
            "changes only"
        } else {
            "full file"
        };
        if glass_button::pill_ui(
            ui,
            layout.pill_center,
            "full_file_toggle",
            label,
            self.full_file,
            PILL_TIP,
        ) {
            self.full_file = !self.full_file;
            self.scroll_to_change_pending = true;
            self.surface_shown_at = Instant::now();
        }

        let owned_blocks;
        let blocks: &[app_core::text_diff::DiffRowProcessedForUi] = if self.full_file {
            if let Some(f) = review.files.get(self.surface_step) {
                owned_blocks = app_core::text_diff::process(
                    &app_core::text_diff::pair_lines_full(&f.old_text, &f.new_text),
                );
                owned_blocks.as_slice()
            } else {
                &[]
            }
        } else {
            review
                .files
                .get(self.surface_step)
                .map(|f| f.rows.as_slice())
                .unwrap_or(&[])
        };

        let scroll_once = self.scroll_to_change_pending;
        self.scroll_to_change_pending = false;
        let right_accent = match review.kind {
            BannerKind::Execute | BannerKind::CargoRun => {
                Some(egui::Color32::from_rgb(200, 90, 220))
            }
            _ => None,
        };

        diff_viewer::diff_viewer_ui_no_copy(
            ui,
            layout.viewer,
            blocks,
            diff_viewer::DiffViewerParams {
                move_spans: &[],
                scroll_id: egui::Id::new((
                    "diff_scroll",
                    self.surface_step,
                    self.full_file,
                )),
                scroll_to_change_once: scroll_once,
                right_accent,
                moves: diff_viewer::MoveDisplay::Classic,
            },
        );

        if show_blocks {
            let total = file_count as f32 * layout.block_size
                + (file_count.saturating_sub(1)) as f32 * layout.gap;
            let start_y = layout.viewer.center().y - total / 2.0 + layout.block_size / 2.0;
            for i in 0..file_count {
                let state = if i == self.surface_step {
                    BlockState::Selected
                } else {
                    BlockState::Normal
                };
                let file_name = review
                    .files
                    .get(i)
                    .map(|f| {
                        f.path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| f.path.display().to_string())
                    })
                    .unwrap_or_default();
                let tip = format!("File {} of {}: {}", i + 1, file_count, file_name);
                let y = start_y + i as f32 * (layout.block_size + layout.gap);
                if glass_button::glass_step_block_ui(
                    ui,
                    egui::pos2(layout.block_col_x, y),
                    1000 + i,
                    state,
                    None,
                    &tip,
                ) {
                    self.surface_step = i;
                    self.surface_shown_at = Instant::now();
                }
            }
        }

        if close_button::close_button_ui(ui, layout.close_center, CLOSE_TIP) {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        let total_w = 2.0 * layout.button_size + layout.gap;
        let start_x = layout.viewer.center().x - total_w / 2.0 + layout.button_size / 2.0;
        let mut confirm_clicked = false;
        let mut deny_clicked = false;
        let surface_click_ok = self.surface_shown_at.elapsed() >= CLICK_GUARD;
        if glass_button::glass_button_ui(
            ui,
            egui::pos2(start_x, layout.button_y),
            0,
            GlassIcon::Confirm,
            "Write every step that is not marked.",
        ) && surface_click_ok
        {
            confirm_clicked = true;
        }
        if glass_button::glass_button_ui(
            ui,
            egui::pos2(start_x + layout.button_size + layout.gap, layout.button_y),
            1,
            GlassIcon::Deny,
            "Write nothing. All steps stay available for retry.",
        ) && surface_click_ok
        {
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
        if close_button::close_button_ui(ui, close_center, CLOSE_TIP) {
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

fn refresh_trust_after_chain_confirm(chain: &Chain) {
    use std::collections::HashSet;

    let has_move_or_remove = chain
        .steps
        .iter()
        .any(|s| matches!(s.step.verb, Verb::Move | Verb::Remove));

    for step in &chain.steps {
        match step.step.verb {
            Verb::Move => {
                if let Some(dest) = &step.dest {
                    app_core::run_trust::rename(&step.path, dest);
                    app_core::terminal_router::file_trust::remove_approval(&step.path);
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
            Verb::Remove | Verb::RemoveDir => {
                app_core::run_trust::remove_all_at_path(&step.path);
                app_core::terminal_router::file_trust::remove_approval(&step.path);
                if let Some(root) = app_core::surface::project_root(&step.path) {
                    app_core::trust::remove_from_record(&root, &step.path);
                }
            }
            _ => {}
        }
    }

    // Approvals for .sh / .toml files. The user just confirmed this write,
    // so save the file's content hash so a later edit that lands at the
    // same state won't re-prompt.
    let approved: Vec<PathBuf> = chain
        .steps
        .iter()
        .filter(|s| {
            !matches!(s.step.verb, Verb::Remove | Verb::Move | Verb::RemoveDir)
        })
        .map(|s| s.path.clone())
        .filter(|p| app_core::terminal_router::file_trust::is_sensitive(p))
        .collect();
    let _ = app_core::terminal_router::file_trust::record_approved(&approved);

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