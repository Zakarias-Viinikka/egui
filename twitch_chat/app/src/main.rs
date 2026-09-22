mod app;
mod hotkeys;
mod twitch;
mod x11_overlay;

use std::sync::mpsc::channel;

use eframe::egui;

use app::App;
use hotkeys::{spawn_hotkey_thread, Cmd};

fn main() -> eframe::Result<()> {
    let (cmd_tx, cmd_rx) = channel::<Cmd>();
    spawn_hotkey_thread(cmd_tx);
    x11_overlay::apply();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 600.0])
            .with_position([10.0, 10.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_window_type(egui::X11WindowType::Dock)
            .with_resizable(true)
            .with_mouse_passthrough(true),
        ..Default::default()
    };

    eframe::run_native(
        "chat",
        options,
        Box::new(|_cc| Ok(Box::new(App::new(cmd_rx)))),
    )
}
