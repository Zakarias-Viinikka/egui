use std::sync::Arc;

use eframe::egui::{self, mutex::Mutex};

use crate::router;

pub struct NecessaryStructForEgui {
    page_enum: Arc<Mutex<router::PageToRouteTo>>,
}

impl<'a> NecessaryStructForEgui {
    pub fn new() -> Self {
        Self {
            page_enum: Arc::new(Mutex::new(router::PageToRouteTo::Home)),
        }
    }
}

impl eframe::App for NecessaryStructForEgui {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            let ctx = CtxForActuallyDrawing {
                ui: ui,
                page_enum: Arc::clone(&self.page_enum),
            };
            (Self::literally_draw_the_page)(ctx);
        });
    }
}

pub trait LiterallyDrawThePage {
    fn literally_draw_the_page(ctx: CtxForActuallyDrawing);
}

impl LiterallyDrawThePage for NecessaryStructForEgui {
    fn literally_draw_the_page<'a>(ctx: CtxForActuallyDrawing<'a>) {
        let CtxForActuallyDrawing { ui, page_enum } = ctx;
        router::literally_draw_the_page(page_enum, ui);
    }
}

pub struct CtxForActuallyDrawing<'a> {
    pub ui: &'a mut egui::Ui,
    pub page_enum: Arc<Mutex<router::PageToRouteTo>>,
}
