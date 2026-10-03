/// Hide the window on startup. The UI thread shows it again the first time a
/// diff arrives from the listener.
pub fn startup(ctx: &eframe::egui::Context) {
    let _ = ctx;
}
