pub fn normal_menu() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_decorations(true),
        centered: true,
        ..Default::default()
    }
}
