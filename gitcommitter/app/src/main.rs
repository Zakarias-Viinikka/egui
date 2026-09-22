use eframe::egui;

fn main() -> eframe::Result<()> {
    let path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: gitcommitter <file>");
            std::process::exit(1);
        }
    };

    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("gitcommitter: could not read {}: {}", path, e);
            std::process::exit(1);
        }
    };

    let text = raw
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    let options = eframe::NativeOptions {
        centered: true,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_decorations(false)
            .with_resizable(true)
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        "gitcommitter",
        options,
        Box::new(|cc| {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = egui::Color32::from_gray(45);
            visuals.extreme_bg_color = egui::Color32::from_gray(55);
            visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
            visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
            visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
            visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
            cc.egui_ctx.set_visuals(visuals);
            let mut style = (*cc.egui_ctx.style_of(egui::Theme::Dark)).clone();
            style.text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::proportional(18.0),
            );
            style.text_styles.insert(
                egui::TextStyle::Monospace,
                egui::FontId::monospace(18.0),
            );
            cc.egui_ctx.set_style_of(egui::Theme::Dark, style);
            Ok(Box::new(App {
                path,
                text,
                got_focus: false,
            }))
        }),
    )
}

struct App {
    path: String,
    text: String,
    got_focus: bool,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(egui::Color32::from_gray(45))
                    .inner_margin(0.0),
            )
            .show(ui, |ui| {
                let id = egui::Id::new("commit_text");

                egui::Frame::default()
                    .fill(egui::Color32::from_gray(55))
                    .stroke(egui::Stroke::NONE)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("commit_scroll")
                            .max_height(ui.available_height())
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.text)
                                        .id(id)
                                        .frame(egui::Frame::NONE)
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(1),
                                );
                            });
                    });

                if !self.got_focus {
                    ui.memory_mut(|m| m.request_focus(id));
                    self.got_focus = true;
                }
            });

        if ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
            let mut out = self.text.clone();
            out.push('\n');
            if let Err(e) = std::fs::write(&self.path, out) {
                eprintln!("gitcommitter: could not write {}: {}", self.path, e);
            }
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
