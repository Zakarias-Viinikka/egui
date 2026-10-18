use super::ui_state::HomePageUiState;
use app_core::state::video_library_state::VideoLibraryState;
use eframe::egui;

pub fn draw(ui: &mut egui::Ui, state: &mut VideoLibraryState, ui_state: &mut HomePageUiState) {
    if state.picked_path.is_none() {
        ui.centered_and_justified(|ui| { ui.label("No video selected"); });
        return;
    }
    let Some((w, h, rgba)) = state.video_view.current_frame_rgba.as_ref() else {
        ui.centered_and_justified(|ui| { ui.label("Loading video..."); });
        return;
    };

    if ui_state.last_uploaded_frame_id != Some(state.video_view.frame_id) {
        let img = egui::ColorImage::from_rgba_unmultiplied([*w as usize, *h as usize], rgba);
        let texture = ui.ctx().load_texture("video_frame", img, egui::TextureOptions::LINEAR);
        ui_state.video_texture = Some(texture);
        ui_state.last_uploaded_frame_id = Some(state.video_view.frame_id);
    }
    let Some(texture) = &ui_state.video_texture else { return };

    let available = ui.available_size();
    let wf = *w as f32;
    let hf = *h as f32;
    let scale = (available.x / wf).min(available.y / hf).max(0.05);
    let display_size = egui::vec2(wf * scale, hf * scale);

    ui.centered_and_justified(|ui| {
        ui.add(egui::Image::new(egui::load::SizedTexture::new(texture.id(), display_size)));
    });
}
