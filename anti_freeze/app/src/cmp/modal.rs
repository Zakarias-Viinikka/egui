use eframe::egui;

use super::helpers;
use crate::egui_setup::global_data::GlobalData;
use app_core::popup;

pub fn draw(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    screen: egui::Rect,
    data: &mut GlobalData,
) {
    let (target_id, target_name) = match data.confirm_target.clone() {
        Some(t) => t,
        None => return,
    };
    let count = data.ram_info.iter().filter(|p| p.id == target_id).count();

    painter.rect_filled(
        screen,
        0.0,
        egui::Color32::from_rgba_unmultiplied(8, 12, 24, 165),
    );
    ui.allocate_rect(screen, egui::Sense::click());

    let card_w = 460.0;
    let card_h = 260.0;
    let card_rect = egui::Rect::from_center_size(screen.center(), egui::vec2(card_w, card_h));

    painter.rect_filled(
        card_rect.translate(egui::vec2(0.0, 14.0)).expand(24.0),
        28.0,
        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 30),
    );
    painter.rect_filled(
        card_rect.translate(egui::vec2(0.0, 6.0)).expand(10.0),
        20.0,
        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 60),
    );

    painter.rect_filled(card_rect, 16.0, egui::Color32::from_rgb(250, 250, 253));
    painter.rect_stroke(
        card_rect,
        16.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(224, 226, 234)),
        egui::StrokeKind::Inside,
    );

    let content = card_rect.shrink2(egui::vec2(36.0, 28.0));
    let mut do_cancel = false;
    let mut do_confirm = false;

    ui.scope_builder(egui::UiBuilder::new().max_rect(content), |ui| {
        ui.vertical(|ui| {
            let heading_text = format!("End {}?", target_name);
            let heading_font = egui::FontId::proportional(23.0);
            let heading_color = egui::Color32::from_rgb(184, 54, 54);
            let galley =
                ui.painter()
                    .layout_no_wrap(heading_text, heading_font, heading_color);
            let hsize = galley.size();
            let hpos = egui::pos2(content.center().x - hsize.x / 2.0, content.min.y + 8.0);
            let hrect = egui::Rect::from_min_size(hpos, hsize).expand2(egui::vec2(16.0, 9.0));
            ui.painter().rect_filled(
                hrect,
                10.0,
                egui::Color32::from_rgba_unmultiplied(200, 70, 70, 12),
            );
            ui.painter().rect_stroke(
                hrect,
                10.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(184, 54, 54, 80)),
                egui::StrokeKind::Inside,
            );
            ui.painter().galley(hpos, galley, heading_color);

            ui.add_space(hrect.height() + 18.0);

            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "This will forcibly terminate {} process{}.",
                        count,
                        if count == 1 { "" } else { "es" }
                    ))
                    .color(egui::Color32::from_rgb(105, 107, 120))
                    .size(15.0),
                );
            });

            ui.add_space(34.0);

            let btn_w = 110.0;
            let btn_h = 40.0;
            let gap = 14.0;
            let total_w = btn_w * 2.0 + gap;
            let start_x = content.center().x - total_w / 2.0;
            let btn_y = content.max.y - btn_h;

            let cancel_rect =
                egui::Rect::from_min_size(egui::pos2(start_x, btn_y), egui::vec2(btn_w, btn_h));
            let end_rect = egui::Rect::from_min_size(
                egui::pos2(start_x + btn_w + gap, btn_y),
                egui::vec2(btn_w, btn_h),
            );

            let cancel_resp = ui.interact(
                cancel_rect,
                egui::Id::new("modal_cancel"),
                egui::Sense::click(),
            );
            let end_resp =
                ui.interact(end_rect, egui::Id::new("modal_end"), egui::Sense::click());

            helpers::draw_button_shadow(painter, cancel_rect, 10.0, cancel_resp.hovered());
            let cancel_bg = if cancel_resp.hovered() {
                egui::Color32::from_rgb(232, 234, 242)
            } else {
                egui::Color32::from_rgb(242, 243, 248)
            };
            painter.rect_filled(cancel_rect, 10.0, cancel_bg);
            painter.rect_stroke(
                cancel_rect,
                10.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(216, 218, 228)),
                egui::StrokeKind::Inside,
            );
            painter.text(
                cancel_rect.center(),
                egui::Align2::CENTER_CENTER,
                "Cancel",
                egui::FontId::proportional(15.0),
                egui::Color32::from_rgb(60, 62, 78),
            );

            helpers::draw_button_shadow(painter, end_rect, 10.0, end_resp.hovered());
            let end_bg = if end_resp.hovered() {
                egui::Color32::from_rgb(214, 78, 78)
            } else {
                egui::Color32::from_rgb(196, 58, 58)
            };
            painter.rect_filled(end_rect, 10.0, end_bg);
            painter.rect_stroke(
                end_rect,
                10.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(120, 20, 20, 90)),
                egui::StrokeKind::Inside,
            );
            painter.text(
                end_rect.center(),
                egui::Align2::CENTER_CENTER,
                "End",
                egui::FontId::proportional(15.0),
                egui::Color32::from_rgb(255, 250, 250),
            );

            if cancel_resp.clicked() {
                do_cancel = true;
            }
            if end_resp.clicked() {
                do_confirm = true;
            }
        });
    });

    if do_cancel {
        data.confirm_target = None;
    }
    if do_confirm {
        let pids: Vec<u32> = data
            .ram_info
            .iter()
            .filter(|p| p.id == target_id)
            .map(|p| p.pid)
            .collect();
        data.confirm_target = None;
        data.selected_id = None;
        data.fade_id = None;
        data.fade_progress = 0.0;
        match app_core::killing::kill_pids(&pids) {
            Ok(()) => {
                let msg = format!("Ended {} (x{})", target_name, count);
                if let Err(e2) = popup::show(&msg, 100, 100) {
                    eprintln!("{e2}");
                }
                data.visible = false;
                crate::ram_monitor::restart(data, ui.ctx());
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
            Err(e) => {
                eprintln!("{e}");
                if let Err(e2) = popup::show(&e, 100, 100) {
                    eprintln!("{e2}");
                }
            }
        }
    }
}
