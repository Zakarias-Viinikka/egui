use eframe::egui;

pub fn gradient_rect(
    painter: &egui::Painter,
    rect: egui::Rect,
    top: egui::Color32,
    bottom: egui::Color32,
) {
    let mut mesh = egui::Mesh::default();
    mesh.vertices.push(egui::epaint::Vertex {
        pos: rect.left_top(),
        uv: egui::Pos2::ZERO,
        color: top,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: rect.right_top(),
        uv: egui::Pos2::ZERO,
        color: top,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: rect.left_bottom(),
        uv: egui::Pos2::ZERO,
        color: bottom,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: rect.right_bottom(),
        uv: egui::Pos2::ZERO,
        color: bottom,
    });
    mesh.indices.extend_from_slice(&[0, 1, 2, 1, 2, 3]);
    painter.add(mesh);
}

pub fn human_bytes(b: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let bf = b as f64;
    if bf >= GB {
        format!("{:.2} GB", bf / GB)
    } else if bf >= MB {
        format!("{:.1} MB", bf / MB)
    } else if bf >= KB {
        format!("{:.0} KB", bf / KB)
    } else {
        format!("{} B", b)
    }
}

pub fn draw_button_shadow(
    painter: &egui::Painter,
    rect: egui::Rect,
    corner_radius: f32,
    hovered: bool,
) {
    let layers: i32 = if hovered { 14 } else { 10 };
    let base_alpha = if hovered { 80.0 } else { 50.0 };
    for i in 0..layers {
        let i_f = i as f32;
        let t = i_f / layers as f32;
        let alpha = (base_alpha * (1.0 - t) * (1.0 - t)) as u8;
        if alpha == 0 {
            continue;
        }
        let expand = i_f;
        let offset_y = i_f * 0.8 + 2.0;
        let layer_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left() - expand, rect.top() + offset_y),
            egui::pos2(rect.right() + expand, rect.bottom() + offset_y),
        );
        painter.rect_filled(
            layer_rect,
            corner_radius,
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, alpha),
        );
    }
}
