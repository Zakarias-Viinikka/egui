use super::actions::Actions;
use app_core::state::render_job::RenderOutcome;
use app_core::state::video_library_state::VideoLibraryState;
use eframe::egui;

pub fn draw(ctx: &egui::Context, state: &VideoLibraryState, actions: &mut Actions) {
    let Some(job) = &state.render_job else { return };
    let fraction = job.fraction();
    let outcome = job.outcome();

    egui::Window::new("Rendering")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.label("Black and White");
            ui.add(egui::ProgressBar::new(fraction).show_percentage());
            match outcome {
                RenderOutcome::Running => {
                    if ui.button("Cancel").clicked() { actions.cancel_render = true; }
                }
                RenderOutcome::Done => {
                    ui.label("Done");
                    if ui.button("Close").clicked() { actions.dismiss_render = true; }
                }
                RenderOutcome::Cancelled => {
                    ui.label("Cancelled");
                    if ui.button("Close").clicked() { actions.dismiss_render = true; }
                }
                RenderOutcome::Failed(msg) => {
                    ui.colored_label(ui.visuals().error_fg_color, format!("Failed: {msg}"));
                    if ui.button("Close").clicked() { actions.dismiss_render = true; }
                }
            }
        });
}
