use std::sync::Arc;

use app_core::text_diff::{DiffRowProcessedForUi, MoveRole, MoveSpan, RowKind};
use eframe::egui;
use egui::text::{LayoutJob, TextWrapping};
use egui::{Galley, TextFormat};
use std::ops::Range;

use super::syntax::{Run, SyntaxTable};

const CONTENT_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(0, 0, 0, 230);
const BORDER: egui::Color32 = egui::Color32::from_rgba_premultiplied(70, 70, 70, 90);
const HR_COLOR: egui::Color32 = egui::Color32::from_rgba_premultiplied(90, 90, 90, 55);
const TEXT: egui::Color32 = egui::Color32::from_rgb(220, 220, 220);
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(120, 120, 120);
const TEXT_DIM_MOVED: egui::Color32 = egui::Color32::from_rgb(100, 100, 100);
const BAR_DEL: egui::Color32 = egui::Color32::from_rgb(210, 80, 80);
const BAR_INS: egui::Color32 = egui::Color32::from_rgb(80, 180, 80);
const BAR_MOVE: egui::Color32 = egui::Color32::from_rgb(170, 120, 220);
// Same blue as verb_accent for replace, overwrite and append.
const BAR_EDIT: egui::Color32 = egui::Color32::from_rgb(100, 160, 220);
// Translucent so the syntax color underneath stays readable.
const HL_EDIT: egui::Color32 = egui::Color32::from_rgba_premultiplied(31, 50, 69, 80);
const HL_DEL: egui::Color32 = egui::Color32::from_rgba_premultiplied(66, 25, 25, 80);
const HL_INS: egui::Color32 = egui::Color32::from_rgba_premultiplied(25, 56, 25, 80);
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

/// How moves should be drawn. Classic is today's behaviour: bars and
/// strips on both sides. Dimmed hides the strip and the bar on the
/// from side and draws its text in dim grey, for snippet views.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveDisplay {
    Classic,
    Dimmed,
}

/// Inputs the viewer needs beyond the row blocks. Defaults are what an
/// old snippet-only view looked like: no moves, shared scroll, no
/// scroll-to-change.
pub struct DiffViewerParams<'a> {
    pub move_spans: &'a [MoveSpan],
    pub scroll_id: egui::Id,
    pub scroll_to_change_once: bool,
    /// When set, the right ("after") pane gets a colored glow. Used to
    /// flag execute / cargo-run requests at a glance.
    pub right_accent: Option<egui::Color32>,
    /// How moves are drawn. Chain review snippet view passes Dimmed;
    /// full-file view and surface reviews pass Classic.
    pub moves: MoveDisplay,
    /// Syntax colors for the left ("before") and right ("after") text,
    /// looked up by each row's line number. None draws plain TEXT.
    pub left_colors: Option<&'a SyntaxTable>,
    pub right_colors: Option<&'a SyntaxTable>,
}

impl Default for DiffViewerParams<'_> {
    fn default() -> Self {
        Self {
            move_spans: &[],
            scroll_id: egui::Id::new("diff_scroll"),
            scroll_to_change_once: false,
            right_accent: None,
            moves: MoveDisplay::Classic,
            left_colors: None,
            right_colors: None,
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
    row_kind: RowKind,
    edit: bool,
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

/// One section per run. With no runs, one plain section in `color`.
fn build_job(
    text: &str,
    font: &egui::FontId,
    color: egui::Color32,
    runs: &[Run],
    highlights: &[Range<usize>],
    hl_bg: egui::Color32,
) -> LayoutJob {
    let plain = || {
        LayoutJob::single_section(
            text.to_string(),
            TextFormat {
                font_id: font.clone(),
                color,
                ..Default::default()
            },
        )
    };
    if runs.is_empty() && highlights.is_empty() {
        return plain();
    }
    let segs = segments_for_line(text.len(), runs, highlights);
    if segs.is_empty() {
        return plain();
    }
    let fits = segs.iter().all(|s| {
        s.end <= text.len() && text.is_char_boundary(s.start) && text.is_char_boundary(s.end)
    });
    if !fits {
        eprintln!(
            "zap: diff_viewer::build_job: syntax runs do not fit the line text, drawing it plain"
        );
        return plain();
    }
    let mut job = LayoutJob::default();
    for seg in &segs {
        job.append(
            &text[seg.start..seg.end],
            0.0,
            TextFormat {
                font_id: font.clone(),
                color: seg.color,
                italics: seg.italic,
                background: if is_highlighted(seg, highlights) {
                    hl_bg
                } else {
                    egui::Color32::TRANSPARENT
                },
                ..Default::default()
            },
        );
    }
    job
}

fn layout_one(
    ui: &egui::Ui,
    text: &str,
    font: &egui::FontId,
    color: egui::Color32,
    wrap_width: f32,
    runs: &[Run],
    highlights: &[Range<usize>],
    hl_bg: egui::Color32,
) -> Arc<Galley> {
    let mut job = build_job(text, font, color, runs, highlights, hl_bg);
    job.wrap = TextWrapping {
        max_width: wrap_width,
        break_anywhere: true,
        ..Default::default()
    };
    ui.fonts_mut(|f| f.layout_job(job))
}

fn bar_color_for(
    kind: &RowKind,
    which: Which,
    in_move: bool,
    moves: MoveDisplay,
    edit: bool,
) -> Option<egui::Color32> {
    if in_move {
        if matches!(moves, MoveDisplay::Dimmed)
            && matches!((kind, which), (RowKind::Delete, Which::Left))
        {
            return None;
        }
        return Some(BAR_MOVE);
    }
    match (kind, which) {
        (RowKind::Equal, _) => None,
        (RowKind::Delete, Which::Left) => Some(BAR_DEL),
        (RowKind::Insert, Which::Right) => Some(BAR_INS),
        (RowKind::Change, _) if edit => Some(BAR_EDIT),
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

fn paint_right_accent(ui: &egui::Ui, rect: egui::Rect, color: egui::Color32) {
    for (w, a) in [(14.0f32, 16u8), (8.0, 40), (3.0, 90)] {
        let c = egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), a);
        ui.painter().rect_stroke(
            rect,
            CORNER + 2.0,
            egui::Stroke::new(w, c),
            egui::StrokeKind::Outside,
        );
    }
    ui.painter().rect_stroke(
        rect,
        CORNER,
        egui::Stroke::new(2.0, color),
        egui::StrokeKind::Inside,
    );
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
    if let Some(accent) = params.right_accent {
        paint_right_accent(ui, right_box, accent);
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
            let left_lineno = row.left.as_ref().and_then(|s| s.lineno);
            let right_lineno = row.right.as_ref().and_then(|s| s.lineno);
            let left_in_move = in_move(params.move_spans, MoveRole::From, left_lineno);
            let right_in_move = in_move(params.move_spans, MoveRole::To, right_lineno);

            let left_dim = matches!(params.moves, MoveDisplay::Dimmed)
                && row.kind == RowKind::Delete
                && left_in_move;
            let left_color = if left_dim { TEXT_DIM_MOVED } else { TEXT };
            let right_color = TEXT;

            let is_change = row.kind == RowKind::Change;
            let (hl_left, hl_right) = if !is_change {
                (egui::Color32::TRANSPARENT, egui::Color32::TRANSPARENT)
            } else if row.edit {
                (HL_EDIT, HL_EDIT)
            } else {
                (HL_DEL, HL_INS)
            };

            let left = row.left.as_ref().map(|s| {
                let runs: &[Run] = if left_dim {
                    &[]
                } else {
                    runs_for(params.left_colors, s.lineno)
                };
                let spans: &[Range<usize>] = if is_change { &s.changed } else { &[] };
                layout_one(ui, &s.text, &font, left_color, wrap_left, runs, spans, hl_left)
            });
            let right = row.right.as_ref().map(|s| {
                let spans: &[Range<usize>] = if is_change { &s.changed } else { &[] };
                layout_one(
                    ui,
                    &s.text,
                    &font,
                    right_color,
                    wrap_right,
                    runs_for(params.right_colors, s.lineno),
                    spans,
                    hl_right,
                )
            });

            let lh = left.as_ref().map(|g| g.size().y).unwrap_or(0.0);
            let rh = right.as_ref().map(|g| g.size().y).unwrap_or(0.0);
            let mut h = lh.max(rh).max(ROW_MIN_H) + ROW_BOTTOM_PAD;

            let tag_left = if matches!(params.moves, MoveDisplay::Classic) {
                span_starting_at(params.move_spans, MoveRole::From, left_lineno)
                    .map(|s| s.label.clone())
            } else {
                None
            };
            let tag_right =
                span_starting_at(params.move_spans, MoveRole::To, right_lineno)
                    .map(|s| s.label.clone());
            if tag_left.is_some() || tag_right.is_some() {
                h += TAG_H;
            }

            laid.push(LaidRow {
                left,
                right,
                left_in_move,
                right_in_move,
                tag_left,
                tag_right,
                left_lineno,
                right_lineno,
                height: h,
                block_start: first,
                kind: block.kind.clone(),
                row_kind: row.kind.clone(),
                edit: row.edit,
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

    let moves_mode = params.moves;
    egui::ScrollArea::vertical()
        .id_salt(params.scroll_id)
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            let mut first_change_rect: Option<egui::Rect> = None;
            let mut label_ys: Vec<f32> = Vec::with_capacity(laid.len());
            for lr in laid.iter() {
                let (row_rect, _) = ui.allocate_exact_size(
                    egui::vec2(rect.width(), lr.height),
                    egui::Sense::hover(),
                );
                let mut y = row_rect.top();

                if lr.block_start && lr.kind != RowKind::Equal {
                    if first_change_rect.is_none() {
                        first_change_rect = Some(row_rect);
                    }
                    hr(ui, inner_left, y);
                    hr(ui, inner_right, y);
                }

                if let Some(label) = &lr.tag_left {
                    paint_strip(ui, inner_left, y, label);
                }
                if let Some(label) = &lr.tag_right {
                    paint_strip(ui, inner_right, y, label);
                }
                if lr.tag_left.is_some() || lr.tag_right.is_some() {
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
                    &lr.row_kind,
                    lr.left.is_some(),
                    moves_mode,
                    lr.edit,
                );
                draw_side_paint(
                    ui,
                    inner_right,
                    y,
                    side_h,
                    Which::Right,
                    lr.right_lineno,
                    lr.right_in_move,
                    &lr.row_kind,
                    lr.right.is_some(),
                    moves_mode,
                    lr.edit,
                );

                // Wrap markers: a small arrow at each continuation row,
                // sitting in the gap between the bar and the text.
                paint_wrap_marks(ui, text_x_left - 10.0, y, lr.left.as_ref());
                paint_wrap_marks(ui, text_x_right - 10.0, y, lr.right.as_ref());

                label_ys.push(y);
            }

            // Place each pane's labels together so selection stays within its pane.
            for (lr, y) in laid.iter().zip(label_ys.iter()) {
                place_label(ui, lr.left.as_ref(), text_x_left, *y, wrap_left);
            }
            for (lr, y) in laid.iter().zip(label_ys.iter()) {
                place_label(ui, lr.right.as_ref(), text_x_right, *y, wrap_right);
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
    moves: MoveDisplay,
    edit: bool,
) {
    if let Some(color) = bar_color_for(kind, which, in_move, moves, edit) {
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

fn runs_for<'a>(table: Option<&'a SyntaxTable>, lineno: Option<u32>) -> &'a [Run] {
    match (table, lineno) {
        (Some(t), Some(n)) => t.line(n),
        _ => &[],
    }
}

/// A stretch of one line with one color. Byte offsets into the line text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub start: usize,
    pub end: usize,
    pub color: egui::Color32,
    pub italic: bool,
}

/// Cut a line of `len` bytes into segments that cover it with no gaps.
/// Cuts fall on every run edge and every edge of a highlighted range.
/// Color and italic come from the run under the segment; uncovered bytes
/// are TEXT and upright. The highlighted ranges only add cuts here; the
/// caller decides what a highlighted segment looks like.
pub fn segments_for_line(len: usize, runs: &[Run], highlights: &[Range<usize>]) -> Vec<Segment> {
    if len == 0 {
        return Vec::new();
    }
    let mut cuts: Vec<usize> = vec![0, len];
    for r in runs {
        cuts.push(r.start.min(len));
        cuts.push(r.end.min(len));
    }
    for h in highlights {
        cuts.push(h.start.min(len));
        cuts.push(h.end.min(len));
    }
    cuts.sort_unstable();
    cuts.dedup();
    cuts.windows(2)
        .map(|w| {
            let (start, end) = (w[0], w[1]);
            let run = runs.iter().find(|r| r.start <= start && start < r.end);
            Segment {
                start,
                end,
                color: run.map(|r| r.color).unwrap_or(TEXT),
                italic: run.map(|r| r.italic).unwrap_or(false),
            }
        })
        .collect()
}

/// True when the whole segment lies inside one highlighted range. Cuts
/// fall on every highlight edge, so a segment is either inside or outside.
pub fn is_highlighted(seg: &Segment, highlights: &[Range<usize>]) -> bool {
    highlights
        .iter()
        .any(|h| h.start <= seg.start && seg.end <= h.end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(start: usize, end: usize, color: egui::Color32, italic: bool) -> Run {
        Run {
            start,
            end,
            color,
            italic,
        }
    }

    fn assert_covers(segs: &[Segment], len: usize) {
        assert_eq!(segs.first().map(|s| s.start), Some(0));
        assert_eq!(segs.last().map(|s| s.end), Some(len));
        for w in segs.windows(2) {
            assert_eq!(w[0].end, w[1].start, "gap or overlap");
        }
        assert!(segs.iter().all(|s| s.start < s.end));
    }

    #[test]
    fn covers_the_whole_line_with_no_gaps() {
        let red = egui::Color32::RED;
        let blue = egui::Color32::BLUE;
        let runs = [run(0, 3, red, false), run(5, 8, blue, true)];
        let segs = segments_for_line(10, &runs, &[]);
        assert_covers(&segs, 10);
        assert_eq!(segs.len(), 4);
        assert_eq!(segs[1].color, TEXT);
        assert!(!segs[1].italic);
        assert_eq!(segs[2].color, blue);
        assert!(segs[2].italic);
    }

    #[test]
    fn splits_at_run_edges() {
        let red = egui::Color32::RED;
        let green = egui::Color32::GREEN;
        let runs = [run(0, 4, red, false), run(4, 9, green, false)];
        let segs = segments_for_line(9, &runs, &[]);
        assert_covers(&segs, 9);
        assert_eq!(segs.len(), 2);
        assert_eq!((segs[0].start, segs[0].end), (0, 4));
        assert_eq!((segs[1].start, segs[1].end), (4, 9));
    }

    #[test]
    fn splits_at_highlight_edges() {
        let red = egui::Color32::RED;
        let runs = [run(0, 10, red, false)];
        let segs = segments_for_line(10, &runs, &[2..6]);
        assert_covers(&segs, 10);
        assert_eq!(segs.len(), 3);
        assert_eq!((segs[1].start, segs[1].end), (2, 6));
        assert_eq!(segs[1].color, red);
    }

    #[test]
    fn splits_at_the_union_of_run_and_span_edges() {
        let red = egui::Color32::RED;
        let green = egui::Color32::GREEN;
        let runs = [run(0, 4, red, false), run(4, 9, green, false)];
        let spans = [2..6];
        let segs = segments_for_line(9, &runs, &spans);
        assert_covers(&segs, 9);
        let edges: Vec<(usize, usize)> = segs.iter().map(|s| (s.start, s.end)).collect();
        assert_eq!(edges, vec![(0, 2), (2, 4), (4, 6), (6, 9)]);
        let flags: Vec<bool> = segs.iter().map(|s| is_highlighted(s, &spans)).collect();
        assert_eq!(flags, vec![false, true, true, false]);
        assert_eq!(segs[1].color, red);
        assert_eq!(segs[2].color, green);
    }

    #[test]
    fn empty_line_and_no_runs() {
        assert!(segments_for_line(0, &[], &[]).is_empty());
        let segs = segments_for_line(5, &[], &[]);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].color, TEXT);
    }
}
