use eframe::egui;

use crate::egui_setup::{
    config,
    drawing_requirements::{self},
    post_init_config,
};

pub fn run() -> eframe::Result<()> {
    let _listener = app_core::terminal_router::listener::spawn(
        app_core::terminal_router::SOCKET_PATH,
    );
    let mut options = config::normal_menu();

    // The PNG is baked into the binary at compile time, so the icon still
    // works if the app is installed somewhere without its source tree.
    if let Some(icon) = window_icon() {
        options.viewport = options.viewport.with_icon(icon);
    }

    let egui_drawing_for_eternity_main = drawing_requirements::NecessaryStructForEgui::default();

    let post_init_options = get_all_post_init_options();

    eframe::run_native(
        "zapgui",
        options,
        // The |cc| { ... } below is a closure. eframe calls it ONE time.
        // It's called after the window exists, before the first frame is drawn.
        // There is no Ui yet. cc.egui_ctx is the Context, and it exists here.
        Box::new(move |cc| {
            // runs ONE time
            app_core::terminal_router::set_ctx(cc.egui_ctx.clone());
            cc.egui_ctx.set_visuals(eframe::egui::Visuals::light());
            draw_once_on_startup(&cc.egui_ctx, post_init_options);
            // runs ONE time. Hands the App to eframe.
            // After this, eframe calls App::ui every frame, forever.
            Ok(Box::new(egui_drawing_for_eternity_main))
        }),
    )
}

fn get_all_post_init_options() -> Vec<Box<dyn FnOnce(&egui::Context)>> {
    let mut post_init_options: Vec<Box<dyn FnOnce(&egui::Context)>> = vec![];
    post_init_options.push(Box::new(post_init_config::startup));

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

fn window_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!("../assets/bird.png");
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}
