use eframe::egui;

use crate::egui_setup::{config, drawing_requirements};

pub fn run() -> eframe::Result<()> {
    let mut options = config::normal_menu();

    // The PNG is baked into the binary at compile time, so the icon still
    // works if the app is installed somewhere without its source tree.
    if let Some(icon) = window_icon() {
        options.viewport = options.viewport.with_icon(icon);
    }

    let egui_drawing_for_eternity_main = drawing_requirements::NecessaryStructForEgui::default();

    eframe::run_native(
        "zapcli",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(eframe::egui::Visuals::dark());
            Ok(Box::new(egui_drawing_for_eternity_main))
        }),
    )
}

fn window_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!("../assets/duck.png");
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}
