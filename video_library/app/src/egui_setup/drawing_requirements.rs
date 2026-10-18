use crate::egui_setup::side_panel::{jump_picker_to, render_side_panel, SidePanelState};
use app_core::state::video_library_state::VideoLibraryState;
use db_wrapper::mascot::LiveForever;
use eframe::egui;
use egui_file_dialog::{DialogState, FileDialog};
use every_page_ever::MainPages::HomePage::{ui_state::HomePageUiState, view};
use zutil_db::init::init_db;

pub struct NecessaryStructForEgui {
    pub video_picker: FileDialog,
    pub save_picker: FileDialog,
    pub side_panel: SidePanelState,
    pub db: LiveForever,
    pub video_library: VideoLibraryState,
    pub home_ui: HomePageUiState,
}

impl NecessaryStructForEgui {
    pub fn new() -> Self {
        let db = init_db();
        let starred = zutil_db::helpers::starred_folders::read_all(&db).unwrap_or_else(|e| {
            eprintln!("init::read_all starred_folders failed: {e}");
            Vec::new()
        });

        let video_picker = FileDialog::new()
            .default_size([900.0, 680.0])
            .default_pos([30.0, 50.0])
            .show_path_edit_button(false)
            .show_parent_button(false)
            .show_left_panel(false)
            .as_modal(false);

        let save_picker = FileDialog::new()
            .default_size([900.0, 680.0])
            .default_pos([30.0, 50.0])
            .title("Select where to save to")
            .as_modal(false);

        Self {
            video_picker,
            save_picker,
            side_panel: SidePanelState::new(starred),
            db,
            video_library: VideoLibraryState::new(),
            home_ui: HomePageUiState::new(),
        }
    }
}

impl eframe::App for NecessaryStructForEgui {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let tick_changed = self.video_library.tick();

        let actions = egui::CentralPanel::default()
            .show(ui, |ui| view::draw(ui, &mut self.video_library, &mut self.home_ui))
            .inner;

        let mut job_started = false;

        if actions.open_video_picker {
            self.video_picker.pick_file();
        }

        if actions.pull_from_kdenlive {
            self.video_library.start_pull_from_kdenlive();
            job_started = true;
        }

        if actions.push_to_kdenlive {
            self.video_library.start_push_to_kdenlive();
            job_started = true;
        }

        if actions.open_bw_save_picker {
            if let Some(src) = self.video_library.picked_path.clone() {
                let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
                let name = format!("{}_bw.mp4", stem);
                let old = std::mem::replace(&mut self.save_picker, FileDialog::new());
                self.save_picker = old.default_file_name(&name);
                self.save_picker.save_file();
            }
        }

        if actions.cancel_render {
            if let Some(job) = &self.video_library.render_job {
                job.progress.request_cancel();
            }
        }

        if actions.dismiss_render {
            self.video_library.render_job = None;
        }

        self.video_picker.update(ui.ctx());
        if let Some(path) = self.video_picker.take_picked() {
            match self.video_library.load_video(path) {
                Ok(()) => {}
                Err(e) => self.video_library.last_error = Some(format!("load failed: {e}")),
            }
            ui.ctx().request_repaint();
        }

        self.save_picker.update(ui.ctx());
        if let Some(output) = self.save_picker.take_picked() {
            if let Err(e) = self.video_library.start_bw_render(output) {
                self.video_library.last_error = Some(format!("start render failed: {e}"));
            }
            ui.ctx().request_repaint();
        }

        let video_open = matches!(self.video_picker.state(), DialogState::Open);
        let save_open = matches!(self.save_picker.state(), DialogState::Open);

        if video_open || save_open {
            let Self { video_picker, save_picker, side_panel, db, .. } = self;
            let active = if video_open { video_picker } else { save_picker };
            egui::Window::new("Folders")
                .default_size([400.0, 680.0])
                .default_pos([950.0, 50.0])
                .resizable(true)
                .min_width(260.0)
                .show(ui.ctx(), |ui| {
                    render_side_panel(ui, active, side_panel, db);
                });
        }

        if let Some(path) = self.side_panel.pending_jump.take() {
            if video_open {
                jump_picker_to(&mut self.video_picker, path);
            } else if save_open {
                jump_picker_to(&mut self.save_picker, path);
            }
        }

        let busy = self.video_library.kdenlive.is_busy() || self.video_library.is_rendering();
        let interval_ms = if busy || job_started || tick_changed { 120 } else { 1500 };
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(interval_ms));
    }
}
