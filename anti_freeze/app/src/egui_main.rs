use eframe::egui;

use crate::egui_setup::{
    config,
    drawing_requirements::{self},
    global_data,
    post_init_config,
};
use app_core::reading_ram::tmp_tester::TmpStats;

pub fn run() -> eframe::Result<()> {
    let options = config::fullscreen_no_menu();

    let initial_ram = match app_core::reading_ram::read_top_processes() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            Vec::new()
        }
    };

    let (hotkey_tx, hotkey_rx) = std::sync::mpsc::channel::<crate::hotkeys::Cmd>();
    let tmp_stats = std::sync::Arc::new(std::sync::Mutex::new(TmpStats::default()));
    let tmp_stats_for_struct = tmp_stats.clone();
    let monitor_stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let monitor_stop_for_start = monitor_stop.clone();
    let monitor_tx = hotkey_tx.clone();

    let egui_drawing_for_eternity_main = drawing_requirements::NecessaryStructForEgui {
        global_data: global_data::GlobalData {
            background_opacity: global_data::BACKGROUND_OPACITY,
            ram_info: initial_ram,
            selected_id: None,
            selection_expires_at: None,
            fade_id: None,
            fade_progress: 0.0,
            confirm_target: None,
            hotkey_rx: hotkey_rx,
            visible: cfg!(debug_assertions),
            tmp_stats: tmp_stats_for_struct,
            frozen_pids: Vec::new(),
            cmd_tx: hotkey_tx.clone(),
            ram_monitor_stop: monitor_stop,
        },
    };

    let post_init_options = get_all_post_init_options();

    eframe::run_native(
        "anti_freeze",
        options,
        Box::new(move |cc| {
            crate::hotkeys::spawn_hotkey_thread(hotkey_tx, cc.egui_ctx.clone());
            crate::x11_window::set_skip_taskbar();
            crate::ram_monitor::start(monitor_stop_for_start, monitor_tx, cc.egui_ctx.clone());
            let repaint_ctx = cc.egui_ctx.clone();
            app_core::reading_ram::tmp_tester::spawn_stats_watcher(tmp_stats.clone(), move || {
                repaint_ctx.request_repaint();
            });
            cc.egui_ctx.set_visuals(eframe::egui::Visuals::light());
            draw_once_on_startup(&cc.egui_ctx, post_init_options);
            Ok(Box::new(egui_drawing_for_eternity_main))
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
