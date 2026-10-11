use std::sync::Arc;

use app_core::text_diff::{DiffRowProcessedForUi, MoveRole, MoveSpan, RowKind};
use eframe::egui;
use egui::text::{LayoutJob, TextWrapping};
use egui::{Galley, TextFormat};

const CONTENT_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(0, 0, 0, 230);
const BORDER: egui::Color32 = egui::Color32::from_rgba_premultiplied(70, 70, 70, 90);
const HR_COLOR: egui::Color32 = egui::Color32::from_rgba_premultiplied(90, 90, 90, 55);
const TEXT: egui::Color32 = egui::Color32::from_rgb(220, 220, 220);
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(120, 120, 120);
const BAR_DEL: egui::Color32 = egui::Color32::from_rgb(210, 80, 80);
const BAR_INS: egui::Color32 = egui::Color32::from_rgb(80, 180, 80);
const BAR_MOVE: egui::Color32 = egui::Color32::from_rgb(170, 120, 220);
const STRIP_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(60, 40, 90, 170);
const STRIP_TEXT: egui::Color32 = egui::Color32::from_rgb(210, 180, 240);

const CORNER: f32 = 10.0;
const PAD: f32 = 12.0;
const GAP: f32 = 14.0;
const GUTTER_W: f32 = 36.0;
const BAR_W: f32 = 3.0;
const BAR_PAD: f32 = 4.0;
const ROW_MIN_H: f32 = 22.0;
const FONT_SIZE: f32 = 14.0;
const TEXT_PAD: f32 = 22.0;
const SCROLLBAR_RESERVE: f32 = 14.0;
const TAG_H: f32 = 16.0;
const ROW_BOTTOM_PAD: f32 = 4.0;

/// Inputs the viewer needs beyond the row blocks. Defaults are what an
/// old snippet-only view looked like: no moves, shared scroll, no
/// scroll-to-change.
pub struct DiffViewerParams<'a> {
    pub move_spans: &'a [MoveSpan],
    pub scroll_id: egui::Id,
    pub scroll_to_change_once: bool,
}

impl Default for DiffViewerParams<'_> {
    fn default() -> Self {
        Self {
            move_spans: &[],
            scroll_id: egui::Id::new("diff_scroll"),
            scroll_to_change_once: false,
        }
    }
}

#[derive(Clone, Copy)]
enum Which {
    Left,
    Right,
}

struct LaidRow {
    left: Option<Arc<Galley>>,
    right: Option<Arc<Galley>>,
    left_in_move: bool,
    right_in_move: bool,
    tag_left: Option<String>,
    tag_right: Option<String>,
    left_lineno: Option<u32>,
    right_lineno: Option<u32>,
    height: f32,
    block_start: bool,
    kind: RowKind,
}

pub fn diff_viewer_ui(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    blocks: &[DiffRowProcessedForUi],
    params: DiffViewerParams<'_>,
) {
    diff_viewer_body(ui, rect, blocks, true, params);
}

pub fn diff_viewer_ui_no_copy(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    blocks: &[DiffRowProcessedForUi],
    params: DiffViewerParams<'_>,
) {
    diff_viewer_body(ui, rect, blocks, false, params);
}

fn layout_one(
    ui: &egui::Ui,
    text: &str,
    font: &egui::FontId,
    color: egui::Color32,
    wrap_width: f32,
) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(
        text.to_string(),
        TextFormat {
            font_id: font.clone(),
            color,
            ..Default::default()
        },
    );
    job.wrap = TextWrapping {
        max_width: wrap_width,
        break_anywhere: true,
        ..Default::default()
    };
    ui.fonts_mut(|f| f.layout_job(job))
}

fn bar_color_for(kind: &RowKind, which: Which, in_move: bool) -> Option<egui::Color32> {
    if in_move {
        return Some(BAR_MOVE);
    }
    match (kind, which) {
        (RowKind::Equal, _) => None,
        (RowKind::Delete, Which::Left) => Some(BAR_DEL),
        (RowKind::Insert, Which::Right) => Some(BAR_INS),
        (RowKind::Change, Which::Left) => Some(BAR_DEL),
        (RowKind::Change, Which::Right) => Some(BAR_INS),
        _ => None,
    }
}

fn in_move(spans: &[MoveSpan], role: MoveRole, lineno: Option<u32>) -> bool {
    let Some(n) = lineno else { return false };
    spans
        .iter()
        .any(|s| s.role == role && n >= s.first_line && n <= s.last_line)
}

fn span_starting_at<'a>(
    spans: &'a [MoveSpan],
    role: MoveRole,
    lineno: Option<u32>,
) -> Option<&'a MoveSpan> {
    let n = lineno?;
    spans
        .iter()
        .find(|s| s.role == role && s.first_line == n)
}

fn diff_viewer_body(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    blocks: &[DiffRowProcessedForUi],
    show_copy: bool,
    params: DiffViewerParams<'_>,
) {
    let mid_x = rect.center().x;
    let left_box = egui::Rect::from_min_max(rect.min, egui::pos2(mid_x - GAP / 2.0, rect.max.y));
    let right_box =
        egui::Rect::from_min_max(egui::pos2(mid_x + GAP / 2.0, rect.min.y), rect.max);

    for r in [left_box, right_box] {
        ui.painter().rect(
            r,
            CORNER,
            CONTENT_BG,
            egui::Stroke::new(1.0, BORDER),
            egui::StrokeKind::Inside,
        );
    }

    let inner_left = left_box.shrink(PAD);
    let inner_right = right_box.shrink(PAD);
    let text_x_left = inner_left.left() + GUTTER_W + BAR_W + TEXT_PAD;
    let text_x_right = inner_right.left() + GUTTER_W + BAR_W + TEXT_PAD;
    let wrap_left =
        (inner_left.width() - GUTTER_W - BAR_W - TEXT_PAD - SCROLLBAR_RESERVE).max(40.0);
    let wrap_right =
        (inner_right.width() - GUTTER_W - BAR_W - TEXT_PAD - SCROLLBAR_RESERVE).max(40.0);

    if show_copy {
        let button_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 8.0, rect.top() - 26.0),
            egui::vec2(60.0, 22.0),
        );
        if ui.put(button_rect, egui::Button::new("copy")).clicked() {
            let s = app_core::text_diff::debug_format(blocks);
            let _ = app_core::clipboard::copy_to_clipboard(&s);
        }
    }

    // Pass 0: pre-layout every row's galleys and compute row heights.
    let font = egui::FontId::monospace(FONT_SIZE);
    let mut laid: Vec<LaidRow> = Vec::new();
    for block in blocks {
        let mut first = true;
        for row in &block.rows {
            let left = row
                .left
                .as_ref()
                .map(|s| layout_one(ui, &s.text, &font, TEXT, wrap_left));
            let right = row
                .right
                .as_ref()
                .map(|s| layout_one(ui, &s.text, &font, TEXT, wrap_right));

            let lh = left.as_ref().map(|g| g.size().y).unwrap_or(0.0);
            let rh = right.as_ref().map(|g| g.size().y).unwrap_or(0.0);
            let mut h = lh.max(rh).max(ROW_MIN_H) + ROW_BOTTOM_PAD;

            let left_lineno = row.left.as_ref().and_then(|s| s.lineno);
            let right_lineno = row.right.as_ref().and_then(|s| s.lineno);
            let tag_left =
                span_starting_at(params.move_spans, MoveRole::From, left_lineno).map(|s| {
                    let mut l = s.label.clone();
                    if s.edited {
                        l.push_str(" (edited)");
                    }
                    l
                });
            let tag_right =
                span_starting_at(params.move_spans, MoveRole::To, right_lineno).map(|s| {
                    let mut l = s.label.clone();
                    if s.edited {
                        l.push_str(" (edited)");
                    }
                    l
                });
            if tag_left.is_some() || tag_right.is_some() {
                h += TAG_H;
            }

            laid.push(LaidRow {
                left,
                right,
                left_in_move: in_move(params.move_spans, MoveRole::From, left_lineno),
                right_in_move: in_move(params.move_spans, MoveRole::To, right_lineno),
                tag_left,
                tag_right,
                left_lineno,
                right_lineno,
                height: h,
                block_start: first,
                kind: block.kind.clone(),
            });
            first = false;
        }
    }

    // Pass 1: allocate each row. Paint bars, line numbers, and any
    // move-strip label. Text is added in pass 2 as selectable Labels.
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::LEFT)),
    );

    egui::ScrollArea::vertical()
        .id_salt(params.scroll_id)
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            let mut first_change_rect: Option<egui::Rect> = None;
            for lr in laid.iter() {
                let (row_rect, _) = ui.allocate_exact_size(
                    egui::vec2(rect.width(), lr.height),
                    egui::Sense::hover(),
                );
                let mut y = row_rect.top();

                if lr.block_start && lr.kind == RowKind::Change {
                    if first_change_rect.is_none() {
                        first_change_rect = Some(row_rect);
                    }
                    hr(ui, inner_left, y);
                    hr(ui, inner_right, y);
                }

                if let Some(label) = &lr.tag_left {
                    paint_strip(ui, inner_left, y, label);
                    y += TAG_H;
                }
                if let Some(label) = &lr.tag_right {
                    paint_strip(ui, inner_right, y, label);
                    y += TAG_H;
                }

                let side_h = row_rect.bottom() - y;

                draw_side_paint(
                    ui,
                    inner_left,
                    y,
                    side_h,
                    Which::Left,
                    lr.left_lineno,
                    lr.left_in_move,
                    &lr.kind,
                    lr.left.is_some(),
                );
                draw_side_paint(
                    ui,
                    inner_right,
                    y,
                    side_h,
                    Which::Right,
                    lr.right_lineno,
                    lr.right_in_move,
                    &lr.kind,
                    lr.right.is_some(),
                );

                // Wrap markers: a small arrow at each continuation row,
                // sitting in the gap between the bar and the text.
                paint_wrap_marks(ui, text_x_left - 10.0, y, lr.left.as_ref());
                paint_wrap_marks(ui, text_x_right - 10.0, y, lr.right.as_ref());

                // Pass 2: place the text as selectable labels.
                place_label(ui, lr.left.as_ref(), text_x_left, y, wrap_left);
                place_label(ui, lr.right.as_ref(), text_x_right, y, wrap_right);
            }

            if params.scroll_to_change_once {
                if let Some(r) = first_change_rect {
                    // Park the change about three rows below the top so
                    // its context is visible too.
                    let offset = 3.0 * ROW_MIN_H;
                    let shifted = egui::Rect::from_min_size(
                        egui::pos2(r.left(), r.top() - offset),
                        egui::vec2(r.width(), r.height()),
                    );
                    ui.scroll_to_rect(shifted, Some(egui::Align::TOP));
                }
            }
        });
}

/// A marker in the gutter for each continuation row of a wrapped galley.
/// Row 0 gets nothing; each subsequent visual row gets a bright arrow and
/// a short accent bar so it reads even at a glance.
fn paint_wrap_marks(ui: &egui::Ui, x: f32, y: f32, galley: Option<&Arc<Galley>>) {
    let Some(g) = galley else { return };
    if g.rows.len() < 2 {
        return;
    }
    let accent = egui::Color32::from_rgb(230, 180, 90);
    let arrow = egui::Color32::from_rgb(255, 210, 120);
    for row in g.rows.iter().skip(1) {
        let row_top = y + row.rect().top();
        let row_h = row.rect().height();
        let mid = row_top + row_h / 2.0;
        // Accent bar leftmost, then the arrow to its right, both inside
        // the widened pad so they never crowd the change bar or the text.
        ui.painter().rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(x - 9.0, row_top + 2.0),
                egui::vec2(2.0, (row_h - 4.0).max(2.0)),
            ),
            0.0,
            accent,
        );
        ui.painter().text(
            egui::pos2(x + 2.0, mid),
            egui::Align2::LEFT_CENTER,
            "\u{21aa}",
            egui::FontId::monospace(FONT_SIZE - 1.0),
            arrow,
        );
    }
}

/// Place a pre-laid galley as a selectable label at (x, y). Used in pass 2.
/// The galley is passed in already laid out so its wrap width was chosen
/// during pass 0 and the row height already accounts for it.
fn place_label(
    ui: &mut egui::Ui,
    galley: Option<&Arc<Galley>>,
    x: f32,
    y: f32,
    wrap_width: f32,
) {
    let Some(g) = galley else { return };
    let size = g.size();
    let rect = egui::Rect::from_min_size(egui::pos2(x, y), size);
    let _ = wrap_width;
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::LEFT)),
        |ui| {
            ui.add(
                egui::Label::new(g.clone())
                    .selectable(true)
                    .wrap_mode(egui::TextWrapMode::Wrap),
            );
        },
    );
}

fn paint_strip(ui: &egui::Ui, inner: egui::Rect, y: f32, label: &str) {
    let rect = egui::Rect::from_min_size(
        egui::pos2(inner.left(), y),
        egui::vec2(inner.width(), TAG_H),
    );
    ui.painter().rect_filled(rect, 2.0, STRIP_BG);
    ui.painter().text(
        egui::pos2(inner.left() + GUTTER_W + BAR_W + TEXT_PAD, y + TAG_H / 2.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::monospace(FONT_SIZE - 3.0),
        STRIP_TEXT,
    );
}

fn hr(ui: &egui::Ui, inner: egui::Rect, y: f32) {
    ui.painter().line_segment(
        [egui::pos2(inner.left(), y), egui::pos2(inner.right(), y)],
        egui::Stroke::new(1.0, HR_COLOR),
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_side_paint(
    ui: &egui::Ui,
    inner: egui::Rect,
    y_top: f32,
    row_h: f32,
    which: Which,
    lineno: Option<u32>,
    in_move: bool,
    kind: &RowKind,
    has_side: bool,
) {
    if let Some(color) = bar_color_for(kind, which, in_move) {
        if has_side {
            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(inner.left() + GUTTER_W, y_top + BAR_PAD),
                egui::vec2(BAR_W, (row_h - BAR_PAD * 2.0).max(BAR_W)),
            );
            ui.painter().rect_filled(bar_rect, 1.0, color);
        }
    }

    if let Some(n) = lineno {
        ui.painter().text(
            egui::pos2(
                inner.left() + GUTTER_W - 6.0,
                y_top + ROW_MIN_H / 2.0,
            ),
            egui::Align2::RIGHT_CENTER,
            n.to_string(),
            egui::FontId::monospace(FONT_SIZE - 2.0),
            TEXT_DIM,
        );
    }
}