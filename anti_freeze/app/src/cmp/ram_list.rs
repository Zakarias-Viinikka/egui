use eframe::egui;

use crate::egui_setup::global_data::GlobalData;

pub fn draw(ui: &mut egui::Ui, box_rect: egui::Rect, data: &mut GlobalData) {
    let modal_open = data.confirm_target.is_some();
    let fade_id = data.fade_id;
    let fade_progress = data.fade_progress;
    let now = std::time::Instant::now();
    let hold = std::time::Duration::from_millis(200);
    let mut hovered_id: Option<u64> = None;
    let mut clicked: Option<(u64, String)> = None;

    let inner = box_rect.shrink(10.0);
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let len = data.ram_info.len();
                for (i, p) in data.ram_info.iter().enumerate() {
                    let amount = if fade_id == Some(p.id) {
                        fade_progress
                    } else {
                        0.0
                    };
                    let fg_v = egui::lerp(255.0..=0.0, amount) as u8;
                    let fg = egui::Color32::from_rgb(fg_v, fg_v, fg_v);
                    let bg_a = (255.0 * amount) as u8;
                    let bg = egui::Color32::from_rgba_unmultiplied(170, 200, 255, bg_a);

                    let row = egui::Frame::NONE
                        .fill(bg)
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::symmetric(10, 4))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(&p.name)
                                        .color(fg)
                                        .monospace()
                                        .size(15.0),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let mem_fg = if amount > 0.5 {
                                            fg
                                        } else {
                                            egui::Color32::from_gray(190)
                                        };
                                        ui.label(
                                            egui::RichText::new(p.human_memory())
                                                .color(mem_fg)
                                                .monospace()
                                                .size(13.0),
                                        );
                                    },
                                );
                            });
                        });

                    let row_id = egui::Id::new(("ram_row", p.pid));
                    let row_resp = ui.interact(
                        row.response.rect,
                        row_id,
                        if modal_open {
                            egui::Sense::hover()
                        } else {
                            egui::Sense::click()
                        },
                    );

                    if !modal_open && ui.rect_contains_pointer(row.response.rect) {
                        hovered_id = Some(p.id);
                    }
                    if !modal_open && row_resp.clicked() {
                        clicked = Some((p.id, p.name.clone()));
                    }

                    if i + 1 < len {
                        ui.add_space(2.0);
                    }
                }
            });
    });

    if let Some((id, name)) = clicked {
        data.confirm_target = Some((id, name));
    }

    let next_selected = match hovered_id {
        Some(id) => {
            data.selection_expires_at = Some(now + hold);
            Some(id)
        }
        None => match data.selected_id {
            Some(cur) => match data.selection_expires_at {
                Some(t) if now < t => {
                    let remaining = t - now;
                    ui.ctx().request_repaint_after(remaining);
                    Some(cur)
                }
                _ => None,
            },
            None => None,
        },
    };

    let dt = ui.input(|i| i.stable_dt).min(0.1);
    let speed = 8.0;
    match (data.fade_id, next_selected) {
        (None, None) => {
            data.fade_progress = 0.0;
        }
        (None, Some(id)) => {
            data.fade_id = Some(id);
            data.fade_progress = (speed * dt).min(1.0);
        }
        (Some(old), Some(new)) if old != new => {
            data.fade_id = Some(new);
            data.fade_progress = (speed * dt).min(1.0);
        }
        (Some(_), Some(_)) => {
            data.fade_progress = (data.fade_progress + speed * dt).min(1.0);
        }
        (Some(_), None) => {
            data.fade_progress = (data.fade_progress - speed * dt).max(0.0);
            if data.fade_progress == 0.0 {
                data.fade_id = None;
            }
        }
    }

    let target = if next_selected.is_some() { 1.0 } else { 0.0 };
    if data.fade_progress != target {
        ui.ctx().request_repaint();
    }

    data.selected_id = next_selected;
}
