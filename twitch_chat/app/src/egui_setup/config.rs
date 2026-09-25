pub fn chat_overlay() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([400.0, 600.0])
            .with_position([10.0, 10.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_window_type(eframe::egui::X11WindowType::Dock)
            .with_resizable(true)
            .with_mouse_passthrough(true),
        ..Default::default()
    }
}
