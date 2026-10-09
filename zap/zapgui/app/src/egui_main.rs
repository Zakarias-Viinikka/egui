use eframe::egui;

use crate::egui_setup::{
    config,
    drawing_requirements::{self},
    post_init_config,
};

pub fn run() -> eframe::Result<()> {
    let hidden = std::env::args().any(|a| a == "--hidden");

    // Manual launch: hand off to any running instance instead of stealing
    // its socket.
    if !hidden && handoff_to_existing() {
        return Ok(());
    }

    let _listener = app_core::terminal_router::listener::spawn(
        app_core::terminal_router::SOCKET_PATH,
    );
    let mut options = config::normal_menu();

    if let Some(icon) = window_icon() {
        options.viewport = options.viewport.with_icon(icon);
    }

    let mut app = drawing_requirements::NecessaryStructForEgui::default();
    if !hidden {
        app.open_menu();
    }

    let post_init_options = get_all_post_init_options();

    eframe::run_native(
        "zapgui",
        options,
        Box::new(move |cc| {
            app_core::terminal_router::set_ctx(cc.egui_ctx.clone());
            cc.egui_ctx.set_visuals(eframe::egui::Visuals::light());
            draw_once_on_startup(&cc.egui_ctx, post_init_options);
            cc.egui_ctx.request_repaint();
            Ok(Box::new(app))
        }),
    )
}

/// Try to hand a manual launch off to an already-running zapgui by asking
/// it to show its menu. Returns true if it did.
fn handoff_to_existing() -> bool {
    use std::io::Write;
    use std::net::Shutdown;
    use std::os::unix::net::UnixStream;

    let Ok(mut stream) = UnixStream::connect(app_core::terminal_router::SOCKET_PATH) else {
        return false;
    };
    if stream.write_all(b"menu").is_err() {
        return false;
    }
    let _ = stream.shutdown(Shutdown::Write);
    let mut reply = String::new();
    let _ = std::io::Read::read_to_string(&mut stream, &mut reply);
    true
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
