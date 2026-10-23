use eframe::egui;

use crate::cmp::{background, /* freeze_button, */ /* modal, */ ram_list, strip, tmp_ram_readout};
use crate::egui_setup::global_data::GlobalData;

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

pub struct NecessaryStructForEgui {
    pub global_data: GlobalData,
}

impl eframe::App for NecessaryStructForEgui {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                let ctx = CtxForActuallyDrawing {
                    ui: ui,
                    global_data: &mut self.global_data,
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
        let ui = ctx.ui;

        if !ctx.global_data.did_first_frame {
            ctx.global_data.did_first_frame = true;
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        while let Ok(cmd) = ctx.global_data.hotkey_rx.try_recv() {
            match cmd {
                crate::hotkeys::Cmd::CapOver => {
                    if let Err(e) = app_core::popup::show("RAM cap lifted", 100, 100) {
                        eprintln!("{e}");
                    }
                    crate::ram_monitor::restart(ctx.global_data, ui.ctx());
                }
                crate::hotkeys::Cmd::CapFailed => {
                    crate::ram_monitor::restart(ctx.global_data, ui.ctx());
                }
                crate::hotkeys::Cmd::ToggleVisible => {
                    ctx.global_data.visible = !ctx.global_data.visible;
                    if ctx.global_data.visible {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
                        crate::x11_window::raise_self();
                    } else {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Visible(false));
                        // crate::cmp::freeze_button::unfreeze_all(ctx.global_data);
                        crate::ram_monitor::restart(ctx.global_data, ui.ctx());
                    }
                    ui.ctx().request_repaint();
                }
            }
        }

        let focused = ui.ctx().input(|i| i.focused);
        if ctx.global_data.visible {
            let focused_now = focused && !ctx.global_data.was_focused;
            let elapsed = ctx.global_data.last_refresh.elapsed();
            if focused_now || elapsed >= std::time::Duration::from_secs(5) {
                match app_core::reading_ram::read_top_processes() {
                    Ok(v) => ctx.global_data.ram_info = v,
                    Err(e) => eprintln!("{e}"),
                }
                ctx.global_data.last_refresh = std::time::Instant::now();
            }
            ui.ctx().request_repaint_after(std::time::Duration::from_secs(5));
        }
        ctx.global_data.was_focused = focused;

        let screen = ui.ctx().input(|i| i.content_rect());
        let box_w = screen.width() * 0.6;
        let box_h = screen.height() * 0.6;
        let box_rect = egui::Rect::from_center_size(screen.center(), egui::vec2(box_w, box_h));

        let painter = ui.painter().clone();

        background::draw(&painter, screen, box_rect, ctx.global_data.background_opacity);
        tmp_ram_readout::draw(ui, screen, ctx.global_data);
        // freeze_button::draw(ui, screen, ctx.global_data);
        strip::draw(&painter, box_rect, ctx.global_data);
        ram_list::draw(ui, box_rect, ctx.global_data);
        // modal::draw(ui, &painter, screen, ctx.global_data);
    }
}

pub struct CtxForActuallyDrawing<'a> {
    pub ui: &'a mut egui::Ui,
    pub global_data: &'a mut GlobalData,
}
