use crate::app::DrawLab;
use crate::strokes::Tool;
use eframe::egui;

pub fn draw(ui: &mut egui::Ui, app: &mut DrawLab, rect: egui::Rect, response: &egui::Response) {
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::ZERO, egui::Color32::BLACK);

    let (fw, fh) = match app.video.current_frame_rgba.as_ref() {
        Some((w, h, _)) => (*w, *h),
        None => {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "no frame",
                egui::FontId::default(),
                egui::Color32::GRAY,
            );
            return;
        }
    };

    if app.last_frame_id != Some(app.video.frame_id) {
        if let Some((w, h, rgba)) = app.video.current_frame_rgba.as_ref() {
            let img = egui::ColorImage::from_rgba_unmultiplied(
                [*w as usize, *h as usize],
                rgba,
            );
            app.texture =
                Some(ui.ctx().load_texture("frame", img, egui::TextureOptions::LINEAR));
            app.last_frame_id = Some(app.video.frame_id);
        }
    }

    let img_rect = fit_rect(rect, fw as f32, fh as f32);

    if let Some(tex) = &app.texture {
        ui.painter().image(
            tex.id(),
            img_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }

    if let Some(mask_tex) = &app.mask_texture {
        ui.painter().image(
            mask_tex.id(),
            img_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }

    for stroke in app.strokes.iter() {
        let color = match stroke.tool {
            Tool::Brush => egui::Color32::from_rgb(0, 255, 0),
            Tool::Eraser => egui::Color32::from_rgb(255, 60, 60),
        };
        let thickness = app.brush_size.max(1.0);
        if stroke.points.len() == 1 {
            let p = map_point(stroke.points[0], img_rect);
            ui.painter().circle_filled(p, thickness * 0.5, color);
        } else {
            let points: Vec<egui::Pos2> =
                stroke.points.iter().map(|p| map_point(*p, img_rect)).collect();
            ui.painter()
                .add(egui::Shape::line(points, egui::Stroke::new(thickness, color)));
        }
    }

    if let Some((x0, y0, x1, y1)) = app.guess_box {
        let p0 = map_point((x0, y0), img_rect);
        let p1 = map_point((x1, y1), img_rect);
        let guess_rect = egui::Rect::from_min_max(p0, p1);
        ui.painter().rect_stroke(
            guess_rect,
            egui::CornerRadius::ZERO,
            egui::Stroke::new(2.0, egui::Color32::YELLOW),
            egui::StrokeKind::Outside,
        );
    }

    if response.drag_started() {
        if let Some(pos) = response.interact_pointer_pos() {
            if img_rect.contains(pos) {
                let norm = normalize(pos, img_rect);
                app.strokes.begin(app.tool, norm);
                app.guess_box = None;
            }
        }
    } else if response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            if img_rect.contains(pos) {
                app.strokes.extend(normalize(pos, img_rect));
                app.guess_box = None;
            }
        }
    }
}

fn fit_rect(bounds: egui::Rect, fw: f32, fh: f32) -> egui::Rect {
    let scale = (bounds.width() / fw).min(bounds.height() / fh);
    let w = fw * scale;
    let h = fh * scale;
    egui::Rect::from_center_size(bounds.center(), egui::vec2(w, h))
}

fn normalize(p: egui::Pos2, rect: egui::Rect) -> (f32, f32) {
    (
        (p.x - rect.left()) / rect.width(),
        (p.y - rect.top()) / rect.height(),
    )
}

fn map_point(p: (f32, f32), rect: egui::Rect) -> egui::Pos2 {
    egui::pos2(
        rect.left() + p.0 * rect.width(),
        rect.top() + p.1 * rect.height(),
    )
}
