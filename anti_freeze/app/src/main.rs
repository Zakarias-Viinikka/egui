use app::egui_main;

fn main() -> eframe::Result<()> {
    if let Err(e) = app_core::watchdog::spawn_guard() {
        eprintln!("{e}");
        if let Err(e2) = app_core::popup::show(&e, 100, 100) {
            eprintln!("{e2}");
        }
    }
    egui_main::run()
}
