mod components;

use components::scrollable_text_input::ui::scrollable_text_input;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "scrollbar_test",
        options,
        Box::new(|cc| {
            for theme in [eframe::egui::Theme::Light, eframe::egui::Theme::Dark] {
                cc.egui_ctx.style_mut_of(theme, |style| {
                    style.interaction.tooltip_delay = f32::INFINITY;
                });
            }
            Ok(Box::new(App::default()))
        }),
    )
}

#[derive(Default)]
struct App {
    long_text: String,
    short_text: String,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        eframe::egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Scrollable text input demo");

            ui.label("Max height 80 — paste a lot here to see the scrollbar:");
            scrollable_text_input(ui, "long_text", &mut self.long_text, 80.0);

            ui.add_space(20.0);

            ui.label("Max height 200 — same component, bigger limit:");
            scrollable_text_input(ui, "short_text", &mut self.short_text, 200.0);
        });
    }
}
