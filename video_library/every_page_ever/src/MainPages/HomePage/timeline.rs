use super::actions::Actions;
use super::ui_state::HomePageUiState;
use app_core::state::video_library_state::VideoLibraryState;
use app_core::thumbnails::cache;
use eframe::egui;
use std::path::Path;
use std::time::Duration;

const TIMELINE_HEIGHT: f32 = 100.0;

pub fn draw(ui: &mut egui::Ui, state: &mut VideoLibraryState, ui_state: &mut HomePageUiState, actions: &mut Actions) {
    let _ = actions;
    let Some(video_path) = state.picked_path.clone() else { return };
    let duration = state.timeline.duration;
    if duration.is_zero() { return; }

    ensure_thumbnails_loaded(ui, &video_path, ui_state);

    if ui_state.thumbnail_textures.iter().any(|t| t.is_none()) {
        ui.ctx().request_repaint_after(Duration::from_millis(200));
    }

    let desired_size = egui::vec2(ui.available_width(), TIMELINE_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());

    ui.painter().rect_filled(rect, egui::CornerRadius::ZERO, ui.visuals().extreme_bg_color);

    let n = cache::THUMB_COUNT;
    if n > 0 {
        let slot_w = rect.width() / n as f32;
        for i in 0..n {
            if let Some(Some(texture)) = ui_state.thumbnail_textures.get(i) {
                let x0 = rect.left() + i as f32 * slot_w;
                let x1 = x0 + slot_w;
                let thumb_rect = egui::Rect::from_min_max(egui::pos2(x0, rect.top()), egui::pos2(x1, rect.bottom()));
                ui.painter().image(
                    texture.id(),
                    thumb_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
    }

    if response.clicked() || response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            let frac = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            let new_us = (duration.as_micros() as f64 * frac as f64) as u64;
            let new_pos = Duration::from_micros(new_us);
            state.timeline.position = new_pos;
            if let Err(e) = state.video_view.scrub_to(new_pos) {
                state.last_error = Some(format!("scrub failed: {e}"));
            }
        }
    }

    let total_us = duration.as_micros().max(1) as f64;
    let pos_us = state.timeline.position.as_micros() as f64;
    let playhead_frac = (pos_us / total_us).clamp(0.0, 1.0) as f32;
    let x = rect.left() + playhead_frac * rect.width();
    ui.painter().line_segment(
        [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
        egui::Stroke::new(2.0, egui::Color32::RED),
    );
}

fn ensure_thumbnails_loaded(ui: &egui::Ui, video_path: &Path, ui_state: &mut HomePageUiState) {
    let n = cache::THUMB_COUNT;
    if ui_state.thumbnail_textures.len() != n {
        ui_state.thumbnail_textures = vec![None; n];
    }
    for i in 0..n {
        if ui_state.thumbnail_textures[i].is_some() { continue; }
        let path = cache::thumb_path(video_path, i);
        if !path.exists() { continue; }
        if let Some((w, h, rgba)) = cache::load_thumb_rgba(&path) {
            let color = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
            let texture = ui.ctx().load_texture(format!("thumb_{i}"), color, egui::TextureOptions::LINEAR);
            ui_state.thumbnail_textures[i] = Some(texture);
        }
    }
}