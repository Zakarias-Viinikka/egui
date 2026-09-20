mod components;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_position([0.0, 0.0])
            .with_inner_size([1920.0, 1080.0])
            .with_decorations(false)
            .with_transparent(true),
        ..Default::default()
    };

    eframe::run_native(
        "egui_template",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}

#[derive(Default)]
struct App {
    main_content: components::main_content::ui::MainContentState,
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &eframe::egui::Visuals) -> [f32; 4] {
        crate::components::background::ui::background_color()
    }

    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        crate::components::background::ui::background_ui(ui, |ui| {
            crate::components::main_content::ui::main_content_ui(ui, &mut self.main_content);
        });
    }
}
