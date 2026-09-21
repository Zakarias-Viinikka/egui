use eframe::egui;

/// A vertical ScrollArea capped at max_height, for arbitrary non-input
/// content. Shrinks to fit if the content is short, otherwise grows a
/// scrollbar and stays at max_height.
pub fn scrollable_panel<F: FnOnce(&mut egui::Ui)>(
    ui: &mut egui::Ui,
    id: &str,
    max_height: f32,
    content: F,
) {
    egui::ScrollArea::vertical()
        .id_salt(id)
        .max_height(max_height)
        .auto_shrink([false, true])
        .show(ui, content);
}
