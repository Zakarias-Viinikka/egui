use std::sync::mpsc::channel;

use eframe::egui;

use crate::egui_setup::{config, drawing_requirements::App};
use crate::hotkeys::{self, Cmd};
use crate::x11_overlay;

pub fn run() -> eframe::Result<()> {
    let (cmd_tx, cmd_rx) = channel::<Cmd>();
    hotkeys::spawn_hotkey_thread(cmd_tx);
    x11_overlay::apply();

    let options = config::chat_overlay();

    eframe::run_native(
        "chat",
        options,
        Box::new(move |cc| {
            let post_init_options = get_all_post_init_options();
            draw_once_on_startup(&cc.egui_ctx, post_init_options);
            Ok(Box::new(App::new(cmd_rx)))
        }),
    )
}

fn get_all_post_init_options() -> Vec<Box<dyn FnOnce(&egui::Context)>> {
    let post_init_options: Vec<Box<dyn FnOnce(&egui::Context)>> = vec![];
    post_init_options
}

fn draw_once_on_startup(
    ctx: &egui::Context,
    post_init_options: Vec<Box<dyn FnOnce(&egui::Context)>>,
) {
    for option in post_init_options {
        option(ctx);
    }
}
