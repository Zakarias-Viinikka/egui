use app_core::command_router::{default_folder, settings_file};
use app_core::general_util::path_parsing::home_dir;
use eframe::egui;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 420.0])
            .with_decorations(true),
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "zap settings",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(SettingsApp::default()))
        }),
    )
}

struct SettingsApp {
    folder: PathBuf,
    modal_open: bool,
    draft: String,
    error: Option<String>,
}

impl Default for SettingsApp {
    fn default() -> Self {
        Self {
            folder: default_folder(),
            modal_open: false,
            draft: String::new(),
            error: None,
        }
    }
}

impl eframe::App for SettingsApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Settings");
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Default folder:");
                ui.label(self.folder.display().to_string());
            });
            ui.add_space(4.0);
            if ui.button("Change default folder").clicked() {
                self.draft = self.folder.display().to_string();
                self.error = None;
                self.modal_open = true;
            }
        });

        if self.modal_open {
            let mut still_open = true;
            egui::Window::new("Set default folder")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ui.ctx(), |ui| {
                    ui.label("Folder path:");
                    ui.text_edit_singleline(&mut self.draft);
                    if let Some(err) = &self.error {
                        ui.colored_label(egui::Color32::from_rgb(220, 120, 120), err);
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            let expanded = expand(&self.draft);
                            if expanded.is_dir() {
                                match write_setting(&expanded) {
                                    Ok(()) => {
                                        self.folder = expanded;
                                        still_open = false;
                                    }
                                    Err(e) => {
                                        self.error =
                                            Some(format!("could not save: {e}"));
                                    }
                                }
                            } else {
                                self.error =
                                    Some(format!("not a folder: {}", expanded.display()));
                            }
                        }
                        if ui.button("Back").clicked() {
                            still_open = false;
                        }
                    });
                });
            if !still_open {
                self.modal_open = false;
            }
        }
    }
}

fn expand(input: &str) -> PathBuf {
    let trimmed = input.trim();
    if trimmed == "~" {
        return home_dir();
    }
    if let Some(rest) = trimmed.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    PathBuf::from(trimmed)
}

fn write_setting(folder: &PathBuf) -> std::io::Result<()> {
    let path = settings_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let body = format!("default_folder={}\n", folder.display());
    std::fs::write(path, body)
}
