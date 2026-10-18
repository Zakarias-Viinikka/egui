use app_core::state::log::LogKind;
use app_core::state::render_job::RenderOutcome;
use app_core::state::video_library_state::VideoLibraryState;
use eframe::egui;

pub fn draw(ui: &mut egui::Ui, state: &VideoLibraryState) {
    let pulling = state.kdenlive.pull_job.is_some();
    let pushing = state.kdenlive.push_job.is_some();
    let render_frac: Option<f32> = state.render_job.as_ref().and_then(|j| {
        if matches!(j.outcome(), RenderOutcome::Running) {
            Some(j.fraction())
        } else {
            None
        }
    });

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        if let Some(frac) = render_frac {
            ui.add(egui::Spinner::new().size(14.0));
            ui.label(
                egui::RichText::new(format!("Rendering B&W... {:.0}%", frac * 100.0))
                    .color(egui::Color32::from_rgb(120, 200, 255)),
            );
            ui.add(
                egui::ProgressBar::new(frac)
                    .desired_width(220.0)
                    .show_percentage(),
            );
        } else if pulling {
            ui.add(egui::Spinner::new().size(14.0));
            ui.label(
                egui::RichText::new("Pulling from Kdenlive...")
                    .color(egui::Color32::from_rgb(120, 200, 255)),
            );
        } else if pushing {
            ui.add(egui::Spinner::new().size(14.0));
            ui.label(
                egui::RichText::new("Adding to Kdenlive bin...")
                    .color(egui::Color32::from_rgb(120, 200, 255)),
            );
        } else {
            ui.label(egui::RichText::new("● Ready").color(egui::Color32::from_rgb(120, 220, 140)));
        }

        if let Some(err) = state.last_error_message.clone() {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(egui::RichText::new("📋 Copy last error").size(13.0))
                    .on_hover_text(&err)
                    .clicked()
                {
                    ui.ctx().copy_text(err);
                }
            });
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