use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

use eframe::egui;

use crate::egui_setup::global_data::GlobalData;
use crate::hotkeys::Cmd;

pub fn start(stop: Arc<AtomicBool>, tx: Sender<Cmd>, ctx: egui::Context) {
    app_core::ram_monitor::spawn(stop, move || {
        crate::ram_cap::on_trip(tx, ctx);
    });
}

pub fn restart(data: &mut GlobalData, ctx: &egui::Context) {
    data.ram_monitor_stop.store(true, Ordering::SeqCst);
    let stop = Arc::new(AtomicBool::new(false));
    data.ram_monitor_stop = stop.clone();
    start(stop, data.cmd_tx.clone(), ctx.clone());
}
