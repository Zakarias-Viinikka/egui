pub fn set_windowed_fullscreen(ctx: &eframe::egui::Context) {
    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Maximized(true));
}
