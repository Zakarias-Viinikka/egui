use eframe::egui;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn popup_binary_path() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    format!("{}/ProgStuff/egui/popup_thingy/target/debug/popup_demo_app", home)
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_position([0.0, 0.0])
            .with_inner_size([1920.0, 1080.0])
            .with_decorations(false)
            .with_transparent(true),
        ..Default::default()
    };

    eframe::run_native(
        "popup_host",
        options,
        Box::new(|_cc| {
            let focus = Arc::new(Mutex::new(String::from("(unknown)")));
            {
                let focus = focus.clone();
                std::thread::spawn(move || loop {
                    let name = std::process::Command::new("xdotool")
                        .arg("getactivewindow")
                        .arg("getwindowname")
                        .output()
                        .ok()
                        .and_then(|o| String::from_utf8(o.stdout).ok())
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| String::from("(none)"));
                    *focus.lock().unwrap() = name;
                    std::thread::sleep(Duration::from_millis(200));
                });
            }
            Ok(Box::new(HostApp { focus, popup_count: 0 }))
        }),
    )
}

struct HostApp {
    focus: Arc<Mutex<String>>,
    popup_count: usize,
}

impl eframe::App for HostApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.8]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx().request_repaint();

        let screen = ui.ctx().input(|i| i.raw.screen_rect).unwrap_or(ui.available_rect_before_wrap());

        ui.painter()
            .rect_filled(screen, 0.0, egui::Color32::from_rgba_premultiplied(0, 0, 0, 204));

        // X button top right
        let close_center = egui::pos2(screen.right() - 20.0, screen.top() + 20.0);
        let close_rect = egui::Rect::from_center_size(close_center, egui::vec2(26.0, 26.0));
        let close_resp = ui.interact(close_rect, egui::Id::new("host_close"), egui::Sense::click());
        if close_resp.hovered() {
            ui.painter().rect_filled(close_rect, 4.0, egui::Color32::from_gray(60));
        }
        let stroke = egui::Stroke::new(2.0, egui::Color32::from_gray(220));
        let pad = 7.0;
        ui.painter().line_segment(
            [egui::pos2(close_rect.left() + pad, close_rect.top() + pad),
             egui::pos2(close_rect.right() - pad, close_rect.bottom() - pad)],
            stroke,
        );
        ui.painter().line_segment(
            [egui::pos2(close_rect.right() - pad, close_rect.top() + pad),
             egui::pos2(close_rect.left() + pad, close_rect.bottom() - pad)],
            stroke,
        );
        if close_resp.clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // popup trigger button, centered. counter box to the right.
        let btn_center = screen.center();
        let btn = egui::Rect::from_center_size(btn_center, egui::vec2(240.0, 60.0));
        let ctr = egui::Rect::from_min_size(
            egui::pos2(btn.right() + 12.0, btn.center().y - 20.0),
            egui::vec2(60.0, 40.0),
        );

        ui.painter().rect_filled(ctr, 6.0, egui::Color32::from_gray(50));
        ui.painter().text(
            ctr.center(),
            egui::Align2::CENTER_CENTER,
            self.popup_count.to_string(),
            egui::FontId::proportional(20.0),
            egui::Color32::from_gray(230),
        );

        if ui.put(btn, egui::Button::new("Show popup")).clicked() {
            self.popup_count += 1;

            let host_id = std::process::Command::new("xdotool")
                .arg("getactivewindow")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            let path = popup_binary_path();
            let popup_x = (screen.center().x - 160.0).to_string();
            let popup_y = (screen.top() + 10.0).to_string();

            let result = std::process::Command::new(&path)
                .arg("copied")
                .arg(popup_x)
                .arg(popup_y)
                .arg(host_id.clone())
                .spawn();



            let log = match &result {
                Ok(child) => format!("spawned pid={} path={}", child.id(), path),
                Err(e) => format!("spawn FAILED: {} path={}", e, path),
            };
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/popup_host_spawn.log")
            {
                let _ = writeln!(f, "{}", log);
            }
        }

        // focus label bottom
        let focus_name = self.focus.lock().unwrap().clone();
        ui.painter().text(
            egui::pos2(screen.center().x, screen.bottom() - 20.0),
            egui::Align2::CENTER_BOTTOM,
            format!("focus: {}", focus_name),
            egui::FontId::proportional(16.0),
            egui::Color32::from_gray(220),
        );
    }
}
