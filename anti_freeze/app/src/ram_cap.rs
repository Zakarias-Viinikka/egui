use std::sync::mpsc::Sender;

use eframe::egui;

use app_core::ram_cap::Decision;

use crate::hotkeys::Cmd;

fn tell_ui(tx: &Sender<Cmd>, ctx: &egui::Context, cmd: Cmd) {
    if let Err(e) = tx.send(cmd) {
        eprintln!("err_msg: send to ui failed: {e} | method: tell_ui | file: app/src/ram_cap.rs");
        return;
    }
    ctx.request_repaint();
}

fn report(e: &str) {
    eprintln!("{e}");
    if let Err(e2) = app_core::popup::show(e, 100, 100) {
        eprintln!("{e2}");
    }
}

pub fn on_trip(tx: Sender<Cmd>, ctx: egui::Context) {
    match app_core::ram_cap::on_trip() {
        Ok(Decision::Normal) => tell_ui(&tx, &ctx, Cmd::RamHigh),
        Ok(Decision::AlreadyCapped) => {}
        Ok(Decision::Capped) => {
            if let Err(e) = app_core::popup::show("RAM cap engaged", 100, 100) {
                eprintln!("{e}");
            }
            std::thread::spawn(move || {
                std::thread::sleep(app_core::ram_cap::CAP_DURATION);
                if let Err(e) = app_core::ram_cap::end_cap() {
                    report(&e);
                }
                tell_ui(&tx, &ctx, Cmd::CapOver);
            });
        }
        Err(e) => {
            report(&e);
            tell_ui(&tx, &ctx, Cmd::RamHigh);
        }
    }
}
