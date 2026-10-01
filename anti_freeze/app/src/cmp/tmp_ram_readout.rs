use eframe::egui;

use crate::egui_setup::global_data::GlobalData;

pub fn draw(ui: &mut egui::Ui, screen: egui::Rect, data: &GlobalData) {
    let stats = match data.tmp_stats.lock() {
        Ok(g) => *g,
        Err(e) => {
            eprintln!("err_msg: tmp_stats lock poisoned: {e} | method: draw | file: app/src/cmp/tmp_ram_readout.rs");
            return;
        }
    };

    let text = format!(
        "RAM {} MB   CPU {}%   GPU {}%",
        stats.ram_mb, stats.cpu_pct, stats.gpu_pct
    );

    let pill_w = 320.0;
    let pill_h = 30.0;
    let pill_rect = egui::Rect::from_min_size(
        egui::pos2(screen.min.x + 12.0, screen.min.y + 12.0),
        egui::vec2(pill_w, pill_h),
    );

    let painter = ui.painter();
    painter.rect_filled(pill_rect, 8.0, egui::Color32::from_rgb(250, 250, 253));
    painter.rect_stroke(
        pill_rect,
        8.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(220, 222, 230)),
        egui::StrokeKind::Inside,
    );
    painter.text(
        pill_rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::monospace(13.0),
        egui::Color32::from_rgb(40, 40, 50),
    );
}
