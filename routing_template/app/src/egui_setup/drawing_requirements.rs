use eframe::egui;

use crate::router;

pub struct NecessaryStructForEgui<'a> {
    page_enum: &'a mut router::PageToRouteTo,
}

impl<'a> NecessaryStructForEgui<'a> {
    pub fn new() -> Self {
        Self {
            page_enum: mut router::PageToRouteTo::Home,
        }
    }
}

impl eframe::App for NecessaryStructForEgui {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            let ctx = CtxForActuallyDrawing {
                ui: ui,
                page_enum: self.page_enum.clone(),
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
    pub page_enum: router::PageToRouteTo,
}
