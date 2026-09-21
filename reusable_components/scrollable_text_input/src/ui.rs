use eframe::egui;

/// A multiline text input that grows to fit its content, but never grows
/// taller than `max_height`. Once the text would exceed that height, the
/// field stops growing and gets a scrollbar instead.
pub fn scrollable_text_input(
    ui: &mut egui::Ui,
    id: &str,
    text: &mut String,
    max_height: f32,
) {
    egui::ScrollArea::vertical()
        .id_salt(id)
        .max_height(max_height)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(text)
                    .desired_width(f32::INFINITY)
                    .desired_rows(1),
            );
        });
}
