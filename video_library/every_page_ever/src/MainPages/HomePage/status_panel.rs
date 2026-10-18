use app_core::state::log::LogKind;
use app_core::state::video_library_state::VideoLibraryState;
use eframe::egui;

pub fn draw(ui: &mut egui::Ui, state: &VideoLibraryState) {
    let busy = state.kdenlive.is_busy();
    let render_running = state
        .render_job
        .as_ref()
        .is_some_and(|j| matches!(j.outcome(), app_core::state::render_job::RenderOutcome::Running));

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        if busy || render_running {
            ui.add(egui::Spinner::new().size(14.0));
            let what = if state.kdenlive.pull_job.is_some() {
                "Pulling from Kdenlive..."
            } else if state.kdenlive.push_job.is_some() {
                "Sending to Kdenlive..."
            } else if render_running {
                "Rendering..."
            } else {
                "Working..."
            };
            ui.label(egui::RichText::new(what).color(egui::Color32::from_rgb(120, 200, 255)));
        } else {
            ui.label(egui::RichText::new("● Ready").color(egui::Color32::from_rgb(120, 220, 140)));
        }
    });

    ui.add_space(4.0);
    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for entry in &state.log {
                let color = match entry.kind {
                    LogKind::Info => egui::Color32::from_gray(200),
                    LogKind::Success => egui::Color32::from_rgb(120, 220, 140),
                    LogKind::Error => egui::Color32::from_rgb(240, 120, 120),
                };
                ui.label(egui::RichText::new(&entry.text).color(color).size(13.0));
            }
        });
}
