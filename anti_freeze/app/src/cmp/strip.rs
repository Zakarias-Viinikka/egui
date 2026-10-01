use eframe::egui;

use super::helpers;
use crate::egui_setup::global_data::GlobalData;

pub fn draw(painter: &egui::Painter, box_rect: egui::Rect, data: &GlobalData) {
    let strip_h = 32.0;
    let strip_w = box_rect.width() * 0.6;
    let strip_rect = egui::Rect::from_min_size(
        egui::pos2(box_rect.center().x - strip_w / 2.0, box_rect.min.y - strip_h - 10.0),
        egui::vec2(strip_w, strip_h),
    );
    painter.rect_filled(strip_rect, 8.0, egui::Color32::from_rgb(245, 245, 248));

    if let Some(id) = data.selected_id {
        let mut total: u64 = 0;
        let mut name: Option<&str> = None;
        let mut count: usize = 0;
        for p in data.ram_info.iter().filter(|p| p.id == id) {
            total += p.memory_bytes;
            if name.is_none() {
                name = Some(&p.name);
            }
            count += 1;
        }
        let (label, col) = match name {
            Some(n) => (
                format!("{}  x{}  \u{00b7}  {}", n, count, helpers::human_bytes(total)),
                egui::Color32::from_rgb(30, 30, 40),
            ),
            None => (
                String::from("nothing selected"),
                egui::Color32::from_gray(150),
            ),
        };
        painter.text(
            strip_rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(15.0),
            col,
        );
    } else {
        painter.text(
            strip_rect.center(),
            egui::Align2::CENTER_CENTER,
            "hover a process",
            egui::FontId::proportional(14.0),
            egui::Color32::from_gray(150),
        );
    }
}
