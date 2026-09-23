use eframe::egui;

/*
 *** Egui requires a struct that implements eframe::App ***
 *
 * i don't think it cares how you actually drawing the thing, but it needs a struct that implements that
 *
 * ---
 *
 * eframe::App implements ui, which provides the ui signature, which is what's needed to actaully draw
 *
 * ---
 *
 *
 * egui::CentralPanel::default().show(ui, |ui| {
 *  // put code here to use ui to draw
 * });
 *
 * ---
 *
 * it might be unecessary but i put the code that actually
 * draws as a trait on the struct egui requires anyway.
 *
 *
 */

pub struct NecessaryStructForEgui {}

impl eframe::App for NecessaryStructForEgui {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            let ctx = CtxForActuallyDrawing { ui: ui };
            (Self::literally_draw_the_page)(ctx);
        });
    }
}

pub trait LiterallyDrawThePage {
    fn literally_draw_the_page(ctx: CtxForActuallyDrawing);
}

impl LiterallyDrawThePage for NecessaryStructForEgui {
    fn literally_draw_the_page<'a>(ctx: CtxForActuallyDrawing<'a>) {
        ctx.ui.label("Hello");
        ctx.ui.separator();
        if ctx.ui.button("Click me").clicked() {
            // do nothing
        }
    }
}

pub struct CtxForActuallyDrawing<'a> {
    pub ui: &'a mut egui::Ui,
}
