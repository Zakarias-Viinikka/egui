use eframe::egui;

use crate::router; //app::router::{self};

pub fn run() -> eframe::Result<()> {
    eframe::run_native(
        "routing_template",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}

struct App {
    page_enum: router::PageToRouteTo,
}

impl App {
    fn new() -> Self {
        Self {
            page_enum: router::PageToRouteTo::Home,
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            router::literally_draw_the_page(self.page_enum.clone(), ui);
        });
    }
}
