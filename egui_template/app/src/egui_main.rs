use eframe::egui;

use crate::egui_setup::{
    config,
    drawing_requirements::{self},
    post_init_config,
};

pub fn run() -> eframe::Result<()> {
    let options = config::normal_menu();

    let egui_drawing_main = drawing_requirements::NecessaryStructForEgui {};

    let post_init_options = get_all_post_init_options();

    eframe::run_native(
        "hello",
        options,
        // The |cc| { ... } below is a closure. eframe calls it ONE time.
        // It's called after the window exists, before the first frame is drawn.
        // There is no Ui yet. cc.egui_ctx is the Context, and it exists here.
        Box::new(move |cc| {
            // runs ONE time
            cc.egui_ctx.set_visuals(eframe::egui::Visuals::light());
            draw_once_on_startup(&cc.egui_ctx, post_init_options);
            // runs ONE time. Hands the App to eframe.
            // After this, eframe calls App::ui every frame, forever.
            Ok(Box::new(egui_drawing_main))
        }),
    )
}

fn get_all_post_init_options() -> Vec<Box<dyn FnOnce(&egui::Context)>> {
    let mut post_init_options: Vec<Box<dyn FnOnce(&egui::Context)>> = vec![];
    post_init_options.push(Box::new(post_init_config::set_windowed_fullscreen));

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
