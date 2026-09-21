use eframe::egui;
use json_parsing::template_markers::{extract_markers, fill_markers};
use std::collections::HashMap;

const FIELD_FONT_SIZE: f32 = 24.0;

pub struct FillTemplateState {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub instructions: String,
    pub example: String,
    pub values: HashMap<String, String>,
    pub focused_index: Option<usize>,
    pub pending_focus: Option<usize>,
}

impl Default for FillTemplateState {
    fn default() -> Self {
        Self {
            id: 0,
            title: String::new(),
            content: String::new(),
            instructions: String::new(),
            example: String::new(),
            values: HashMap::new(),
            focused_index: None,
            pending_focus: None,
        }
    }
}

impl FillTemplateState {
    pub fn reset_values_for_markers(&mut self) {
        let markers = extract_markers(&self.content);
        let mut new_values = HashMap::new();
        for m in markers {
            let v = self.values.get(&m).cloned().unwrap_or_default();
            new_values.insert(m, v);
        }
        self.values = new_values;
        self.focused_index = None;
        self.pending_focus = None;
    }
}

pub enum FillTemplateAction {
    None,
    Back,
    Confirm(String),
}

pub fn fill_template_ui(
    ui: &mut egui::Ui,
    state: &mut FillTemplateState,
) -> FillTemplateAction {
    let mut action = FillTemplateAction::None;

    let screen = ui
        .ctx()
        .input(|i| i.raw.screen_rect)
        .unwrap_or(ui.available_rect_before_wrap());

    ui.painter()
        .rect_filled(screen, 0.0, egui::Color32::from_rgba_premultiplied(0, 0, 0, 255));

    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        return FillTemplateAction::Back;
    }
    if ui.input(|i| i.pointer.secondary_clicked()) {
        return FillTemplateAction::Back;
    }

    let confirm = ui.input(|i| i.key_pressed(egui::Key::Enter));
    let markers = extract_markers(&state.content);

    if !markers.is_empty() {
        let mut dir = 0i32;
        ui.input_mut(|i| {
            if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                dir = 1;
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                dir = -1;
            }
        });

        if state.focused_index.is_none() && state.pending_focus.is_none() {
            state.focused_index = Some(0);
            state.pending_focus = Some(0);
        }

        if dir != 0 {
            let len = markers.len();
            let cur = state.focused_index.unwrap_or(0);
            let next = if dir > 0 {
                (cur + 1) % len
            } else {
                (cur + len - 1) % len
            };
            state.focused_index = Some(next);
            state.pending_focus = Some(next);
        }
    }

    let col_w = screen.width() * 0.8;
    let col_h = screen.height() - 100.0;
    let col_x = screen.center().x - col_w / 2.0;
    let col_y = screen.top() + 50.0;
    let col_rect =
        egui::Rect::from_min_size(egui::pos2(col_x, col_y), egui::vec2(col_w, col_h));

    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(col_rect));
    child.vertical_centered(|ui| {
        egui::ScrollArea::vertical()
            .id_salt("fill_template_body")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(&state.title)
                        .size(34.0)
                        .color(egui::Color32::from_gray(240)),
                );
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(20.0);

                if !state.instructions.is_empty() {
                    ui.label(
                        egui::RichText::new("Instructions")
                            .size(18.0)
                            .strong()
                            .color(egui::Color32::from_gray(170)),
                    );
                    ui.add_space(8.0);
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&state.instructions)
                                .size(17.0)
                                .color(egui::Color32::from_gray(225)),
                        )
                        .selectable(true),
                    );
                    ui.add_space(24.0);
                }

                if !state.example.is_empty() {
                    ui.label(
                        egui::RichText::new("Example")
                            .size(18.0)
                            .strong()
                            .color(egui::Color32::from_gray(170)),
                    );
                    ui.add_space(8.0);
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&state.example)
                                .size(17.0)
                                .color(egui::Color32::from_gray(225)),
                        )
                        .selectable(true),
                    );
                    ui.add_space(24.0);
                }

                if markers.is_empty() {
                    ui.label(
                        egui::RichText::new("no markers in this template")
                            .size(17.0)
                            .color(egui::Color32::from_gray(160)),
                    );
                } else {
                    for (idx, m) in markers.iter().enumerate() {
                        ui.label(
                            egui::RichText::new(m.as_str())
                                .size(17.0)
                                .strong()
                                .color(egui::Color32::from_gray(200)),
                        );
                        ui.add_space(6.0);

                        let field_id = egui::Id::new(("fill_marker", m.as_str()));

                        let response = {
                            let entry = state.values.entry(m.clone()).or_default();

                            let font = egui::FontId::monospace(FIELD_FONT_SIZE);
                            let measure = |txt: &str| -> f32 {
                                ui.painter()
                                    .layout_no_wrap(
                                        txt.to_string(),
                                        font.clone(),
                                        egui::Color32::WHITE,
                                    )
                                    .size()
                                    .x
                            };

                            let bracket_w = measure("[");
                            let text_w = measure(entry).max(2.0);
                            let cursor_pad = 4.0;

                            let row_h = FIELD_FONT_SIZE + 12.0;
                            let total_w = bracket_w + text_w + cursor_pad + bracket_w;

                            let (row_rect, _row_resp) = ui.allocate_exact_size(
                                egui::vec2(total_w, row_h),
                                egui::Sense::hover(),
                            );

                            // brackets
                            ui.painter().text(
                                egui::pos2(row_rect.left(), row_rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                "[",
                                font.clone(),
                                egui::Color32::from_gray(180),
                            );
                            ui.painter().text(
                                egui::pos2(row_rect.right(), row_rect.center().y),
                                egui::Align2::RIGHT_CENTER,
                                "]",
                                font.clone(),
                                egui::Color32::from_gray(180),
                            );

                            // text edit between them
                            let edit_rect = egui::Rect::from_min_max(
                                egui::pos2(row_rect.left() + bracket_w, row_rect.top()),
                                egui::pos2(
                                    row_rect.left() + bracket_w + text_w + cursor_pad,
                                    row_rect.bottom(),
                                ),
                            );

                            ui.scope(|ui| {
                                let mut style = (**ui.style()).clone();
                                style.text_styles.insert(
                                    egui::TextStyle::Body,
                                    egui::FontId::monospace(FIELD_FONT_SIZE),
                                );
                                ui.set_style(style);

                                let v = ui.visuals_mut();
                                v.override_text_color =
                                    Some(egui::Color32::from_gray(245));
                                v.extreme_bg_color = egui::Color32::TRANSPARENT;
                                v.selection.bg_fill =
                                    egui::Color32::from_rgb(70, 100, 150);
                                v.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                                v.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                                v.widgets.active.bg_stroke = egui::Stroke::NONE;

                                ui.put(
                                    edit_rect,
                                    egui::TextEdit::singleline(entry)
                                        .id(field_id)
                                        .frame(egui::Frame::NONE)
                                        .margin(egui::Margin::symmetric(0, 0)),
                                )
                            })
                            .inner
                        };

                        if state.pending_focus == Some(idx) {
                            ui.ctx().memory_mut(|m| m.request_focus(field_id));
                            state.pending_focus = None;
                        }
                        if response.has_focus() {
                            state.focused_index = Some(idx);
                        }

                        ui.add_space(20.0);
                    }
                }

                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new("Preview")
                        .size(18.0)
                        .strong()
                        .color(egui::Color32::from_gray(170)),
                );
                ui.add_space(8.0);
                let preview = fill_markers(&state.content, &state.values);
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&preview)
                            .size(17.0)
                            .color(egui::Color32::from_gray(225)),
                    )
                    .selectable(true),
                );
                ui.add_space(24.0);
            });
    });

    let hint_y = screen.bottom() - 24.0;
    ui.painter().text(
        egui::pos2(screen.left() + 30.0, hint_y),
        egui::Align2::LEFT_CENTER,
        "right click or esc to close",
        egui::FontId::proportional(15.0),
        egui::Color32::from_gray(150),
    );
    ui.painter().text(
        egui::pos2(screen.right() - 30.0, hint_y),
        egui::Align2::RIGHT_CENTER,
        "enter to generate",
        egui::FontId::proportional(15.0),
        egui::Color32::from_gray(180),
    );

    if confirm {
        let filled = fill_markers(&state.content, &state.values);
        action = FillTemplateAction::Confirm(filled);
    }

    action
}
