use eframe::egui;

use crate::egui_setup::global_data::GlobalData;

pub fn draw(ui: &mut egui::Ui, screen: egui::Rect, data: &mut GlobalData) {
    let frozen = !data.frozen_pids.is_empty();
    let label = if frozen { "Unfreeze" } else { "Freeze" };

    let w = 130.0;
    let h = 32.0;
    let rect = egui::Rect::from_min_size(
        egui::pos2(screen.max.x - w - 12.0, screen.min.y + 12.0),
        egui::vec2(w, h),
    );

    let resp = ui.interact(rect, egui::Id::new("freeze_btn"), egui::Sense::click());

    let bg = if frozen {
        if resp.hovered() {
            egui::Color32::from_rgb(220, 130, 60)
        } else {
            egui::Color32::from_rgb(198, 112, 48)
        }
    } else if resp.hovered() {
        egui::Color32::from_rgb(70, 130, 210)
    } else {
        egui::Color32::from_rgb(90, 150, 230)
    };

    let painter = ui.painter();
    painter.rect_filled(rect, 8.0, bg);
    painter.rect_stroke(
        rect,
        8.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 40)),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(14.0),
        egui::Color32::WHITE,
    );

    if resp.clicked() {
        if frozen {
            let pids = std::mem::take(&mut data.frozen_pids);
            if let Err(e) = app_core::killing::unfreeze_pids(&pids) {
                eprintln!("{e}");
                if let Err(e2) = app_core::popup::show(&e, 100, 100) {
                    eprintln!("{e2}");
                }
                data.frozen_pids = pids;
            } else {
                crate::x11_window::raise_self();
                if let Err(e) = app_core::watchdog::clear_frozen() {
                    eprintln!("{e}");
                }
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        } else {
            match app_core::killing::freeze_other_processes() {
                Ok(pids) => {
                    if let Err(e) = app_core::watchdog::record_frozen(&pids) {
                        eprintln!("{e}");
                        if let Err(e2) = app_core::popup::show(&e, 100, 100) {
                            eprintln!("{e2}");
                        }
                    }
                    data.frozen_pids = pids;
                }
                Err(e) => {
                    eprintln!("{e}");
                    if let Err(e2) = app_core::popup::show(&e, 100, 100) {
                        eprintln!("{e2}");
                    }
                }
            }
        }
    }
}

pub fn freeze_all(data: &mut GlobalData) {
    if !data.frozen_pids.is_empty() {
        return;
    }
    match app_core::killing::freeze_other_processes() {
        Ok(pids) => {
            if let Err(e) = app_core::watchdog::record_frozen(&pids) {
                eprintln!("{e}");
                if let Err(e2) = app_core::popup::show(&e, 100, 100) {
                    eprintln!("{e2}");
                }
            }
            data.frozen_pids = pids;
        }
        Err(e) => {
            eprintln!("{e}");
            if let Err(e2) = app_core::popup::show(&e, 100, 100) {
                eprintln!("{e2}");
            }
        }
    }
}
