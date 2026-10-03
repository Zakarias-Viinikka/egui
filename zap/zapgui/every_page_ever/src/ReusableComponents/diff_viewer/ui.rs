use app_core::text_diff::{DiffRowProcessedForUi, RowKind, Side};
use eframe::egui;

const CONTENT_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(0, 0, 0, 230);
const BORDER: egui::Color32 = egui::Color32::from_rgba_premultiplied(70, 70, 70, 90);
const HR_COLOR: egui::Color32 = egui::Color32::from_rgba_premultiplied(90, 90, 90, 55);
const TEXT: egui::Color32 = egui::Color32::from_rgb(220, 220, 220);
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(120, 120, 120);
const BAR_DEL: egui::Color32 = egui::Color32::from_rgb(210, 80, 80);
const BAR_INS: egui::Color32 = egui::Color32::from_rgb(80, 180, 80);
const BAR_EDIT: egui::Color32 = egui::Color32::from_rgb(120, 170, 230);

const CORNER: f32 = 10.0;
const PAD: f32 = 12.0;
const GAP: f32 = 14.0;
const GUTTER_W: f32 = 36.0;
const BAR_W: f32 = 3.0;
const BAR_PAD: f32 = 4.0;
const ROW_H: f32 = 22.0;
const FONT_SIZE: f32 = 14.0;
const TEXT_PAD: f32 = 8.0;

fn bar_color(kind: &RowKind, has_side: bool) -> Option<egui::Color32> {
    if !has_side {
        return None;
    }
    match kind {
        RowKind::Equal => None,
        RowKind::Delete => Some(BAR_DEL),
        RowKind::Insert => Some(BAR_INS),
        RowKind::Change => Some(BAR_EDIT),
    }
}

pub fn diff_viewer_ui(ui: &mut egui::Ui, rect: egui::Rect, blocks: &[DiffRowProcessedForUi]) {
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

    let button_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 8.0, rect.top() - 26.0),
        egui::vec2(60.0, 22.0),
    );
    if ui.put(button_rect, egui::Button::new("copy")).clicked() {
        let s = app_core::text_diff::debug_format(blocks);
        copy_to_clipboard(&s);
    }

    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::LEFT)),
    );

    egui::ScrollArea::vertical()
        .id_salt("diff_scroll")
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            for (bi, block) in blocks.iter().enumerate() {
                let is_change = block.kind == RowKind::Change;
                if is_change {
                    let top_y = ui.cursor().top();
                    hr(ui, inner_left, top_y);
                    hr(ui, inner_right, top_y);
                }

                for row in &block.rows {
                    let (row_rect, _) = ui.allocate_exact_size(
                        egui::vec2(rect.width(), ROW_H),
                        egui::Sense::hover(),
                    );
                    let y_top = row_rect.top();
                    let y_mid = row_rect.center().y;

                    draw_side(
                        ui,
                        inner_left,
                        y_top,
                        y_mid,
                        row.left.as_ref(),
                        &row.kind,
                        row.left.is_some(),
                    );
                    draw_side(
                        ui,
                        inner_right,
                        y_top,
                        y_mid,
                        row.right.as_ref(),
                        &row.kind,
                        row.right.is_some(),
                    );
                }

            }
        });
}

fn hr(ui: &egui::Ui, inner: egui::Rect, y: f32) {
    ui.painter().line_segment(
        [egui::pos2(inner.left(), y), egui::pos2(inner.right(), y)],
        egui::Stroke::new(1.0, HR_COLOR),
    );
}

fn draw_side(
    ui: &egui::Ui,
    inner: egui::Rect,
    y_top: f32,
    y_mid: f32,
    side: Option<&Side>,
    kind: &RowKind,
    has_side: bool,
) {
    let has_content = side
        .map(|s| !s.text.trim().is_empty())
        .unwrap_or(false);
    if let Some(color) = bar_color(kind, has_side && has_content) {
        let bar_rect = egui::Rect::from_min_size(
            egui::pos2(inner.left() + GUTTER_W, y_top + BAR_PAD),
            egui::vec2(BAR_W, ROW_H - BAR_PAD * 2.0),
        );
        ui.painter().rect_filled(bar_rect, 1.0, color);
    }

    if let Some(s) = side {
        if let Some(n) = s.lineno {
            ui.painter().text(
                egui::pos2(inner.left() + GUTTER_W - 6.0, y_mid),
                egui::Align2::RIGHT_CENTER,
                n.to_string(),
                egui::FontId::monospace(FONT_SIZE - 2.0),
                TEXT_DIM,
            );
        }

        ui.painter().text(
            egui::pos2(inner.left() + GUTTER_W + BAR_W + TEXT_PAD, y_mid),
            egui::Align2::LEFT_CENTER,
            &s.text,
            egui::FontId::monospace(FONT_SIZE),
            TEXT,
        );
    }
}

fn copy_to_clipboard(s: &str) {
    use std::io::Write;
    use std::process::{Command, Stdio};
    if let Ok(mut child) = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(s.as_bytes());
        }
        let _ = child.wait();
    }
}
