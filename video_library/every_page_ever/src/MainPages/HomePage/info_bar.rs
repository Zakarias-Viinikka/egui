use app_core::state::video_library_state::VideoLibraryState;
use eframe::egui;
use std::time::Duration;

pub fn draw(ui: &mut egui::Ui, state: &VideoLibraryState) {
    let Some(path) = state.picked_path.as_ref() else {
        ui.label(egui::RichText::new("No video loaded").weak());
        return;
    };

    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("?");
    let duration = state.timeline.duration;
    let res = match state.video_view.current_frame_rgba.as_ref() {
        Some((w, h, _)) => format!("{w}×{h}"),
        None => "—".to_string(),
    };

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        ui.label(egui::RichText::new(name).strong());
        ui.label(egui::RichText::new(format!("⏱ {}", format_duration(duration))).weak());
        ui.label(egui::RichText::new(format!("🖼 {res}")).weak());

        if let Some(pulled) = state.kdenlive.pulled.as_ref() {
            let info = &pulled.info;
            ui.separator();
            ui.label(egui::RichText::new("🔗 from Kdenlive").color(egui::Color32::from_rgb(120, 200, 255)));
            ui.label(egui::RichText::new(format!("track {}", info.track_id)).weak());
            ui.label(egui::RichText::new(format!("pos {}", info.position)).weak());
        }
    });
}

fn format_duration(d: Duration) -> String {
    let total = d.as_secs();
    let m = total / 60;
    let s = total % 60;
    format!("{m}:{s:02}")
}
