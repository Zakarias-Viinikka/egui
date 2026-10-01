use eframe::egui;

use super::helpers;

pub fn draw(painter: &egui::Painter, screen: egui::Rect, box_rect: egui::Rect, opacity: f32) {
    let alpha = (opacity * 255.0).round() as u8;
    let blue_top = egui::Color32::from_rgba_unmultiplied(80, 150, 230, alpha);
    let blue_bottom = egui::Color32::from_rgba_unmultiplied(160, 210, 255, alpha);
    let black = egui::Color32::from_rgba_unmultiplied(0, 0, 0, alpha);
    let border = egui::Color32::from_rgb(0, 0, 0);

    let top = egui::Rect::from_min_max(screen.min, egui::pos2(screen.max.x, box_rect.min.y));
    let bottom = egui::Rect::from_min_max(egui::pos2(screen.min.x, box_rect.max.y), screen.max);
    let left = egui::Rect::from_min_max(
        egui::pos2(screen.min.x, box_rect.min.y),
        egui::pos2(box_rect.min.x, box_rect.max.y),
    );
    let right = egui::Rect::from_min_max(
        egui::pos2(box_rect.max.x, box_rect.min.y),
        egui::pos2(screen.max.x, box_rect.max.y),
    );

    let sh = screen.height().max(1.0);
    let color_at = |y: f32| -> egui::Color32 {
        let t = ((y - screen.min.y) / sh).clamp(0.0, 1.0);
        let r = egui::lerp(blue_top.r() as f32..=blue_bottom.r() as f32, t) as u8;
        let g = egui::lerp(blue_top.g() as f32..=blue_bottom.g() as f32, t) as u8;
        let b = egui::lerp(blue_top.b() as f32..=blue_bottom.b() as f32, t) as u8;
        let a = egui::lerp(blue_top.a() as f32..=blue_bottom.a() as f32, t) as u8;
        egui::Color32::from_rgba_unmultiplied(r, g, b, a)
    };

    helpers::gradient_rect(painter, top, color_at(top.min.y), color_at(top.max.y));
    helpers::gradient_rect(painter, bottom, color_at(bottom.min.y), color_at(bottom.max.y));
    helpers::gradient_rect(painter, left, color_at(left.min.y), color_at(left.max.y));
    helpers::gradient_rect(painter, right, color_at(right.min.y), color_at(right.max.y));

    painter.rect_filled(box_rect, 0.0, black);
    painter.rect_stroke(
        box_rect,
        0.0,
        egui::Stroke::new(2.0, border),
        egui::StrokeKind::Inside,
    );
}
