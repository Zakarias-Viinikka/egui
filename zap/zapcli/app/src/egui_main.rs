use eframe::egui;

use crate::egui_setup::{config, drawing_requirements};

pub fn run() -> eframe::Result<()> {
    if setup_guard_handed_off() {
        return Ok(());
    }

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

/// Try to hand this launch off to a running zapcli. Returns true when it
/// did and the caller should exit. On a fresh launch it binds the small
/// zapcli socket and spawns a listener that raises the window when a
/// second launch sends "focus". On any bind error it returns false and
/// runs without a guard.
fn setup_guard_handed_off() -> bool {
    use std::io::{Read, Write};
    use std::net::Shutdown;
    use std::os::unix::net::{UnixListener, UnixStream};

    const PATH: &str = "/run/user/1000/zapcli.sock";

    if let Ok(mut s) = UnixStream::connect(PATH) {
        let _ = s.write_all(b"focus");
        let _ = s.shutdown(Shutdown::Write);
        return true;
    }

    let _ = std::fs::remove_file(PATH);
    let Ok(listener) = UnixListener::bind(PATH) else {
        return false;
    };
    std::thread::spawn(move || {
        for mut s in listener.incoming().flatten() {
            let mut buf = String::new();
            let _ = s.read_to_string(&mut buf);
            if buf.trim() == "focus" {
                app_core::general_util::instance_guard::request_focus();
            }
        }
    });
    false
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
