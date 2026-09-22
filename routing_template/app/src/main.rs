use app::egui_main;

fn main() -> eframe::Result<()> {
    egui_main::run()
}
/*fn main() -> eframe::Result<()> {
    eframe::run_native(
        "egui_template",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(App))),
    )
}

struct App;

impl eframe::App for App {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        eframe::egui::CentralPanel::default().show(ui, |ui| {
            //ui.label(notes_core::hello());
        });
    }
}*/
