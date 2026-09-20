use db_wrapper::mascot::LiveForever;
use eframe::egui;
use error_stuff::{ErrorDetail, current_error};
use protocol::row_col::Row;

pub fn disabled_ui(ui: &mut egui::Ui, db: &LiveForever) {
    ui.vertical_centered(|ui| {
        ui.heading("App disabled");
        ui.add_space(12.0);

        let mut copy_text = String::new();

        match current_error() {
            Some(err) => {
                let detail_line = match &err.detail {
                    ErrorDetail::Db(e) => format!("db error: {}", e),
                    ErrorDetail::Col(msg) => format!("col error: {}", msg),
                };

                copy_text.push_str(&format!("screen: {}\n", err.screen));
                copy_text.push_str(&format!("location: {}\n", err.location));
                copy_text.push_str(&format!("{}\n", detail_line));

                ui.add(
                    egui::Label::new(format!("screen: {}", err.screen)).selectable(true),
                );
                ui.add(
                    egui::Label::new(format!("location: {}", err.location))
                        .selectable(true),
                );
                ui.add_space(8.0);
                ui.add(egui::Label::new(&detail_line).selectable(true));
            }
            None => {
                ui.label("no error recorded");
            }
        }

        ui.add_space(16.0);

        if let Ok(rows) = logging::read::top_logs_for_session(db, logging::session::get(), 10) {
            if !rows.is_empty() {
                ui.label("Recent logs:");
                copy_text.push_str("\nRecent logs:\n");
                for row in &rows {
                    let line = format_log_row(row);
                    ui.add(egui::Label::new(&line).selectable(true));
                    copy_text.push_str(&line);
                    copy_text.push('\n');
                }
            }
        }

        ui.add_space(16.0);
        if ui.button("Copy error").clicked() {
            ui.ctx().copy_text(copy_text);
        }
    });
}

fn format_log_row(row: &Row) -> String {
    let level = row
        .cols
        .get(2)
        .and_then(|c| c.as_str().ok())
        .unwrap_or("?");
    let category = row
        .cols
        .get(3)
        .and_then(|c| c.as_str().ok())
        .unwrap_or("?");
    let message = row
        .cols
        .get(6)
        .and_then(|c| c.as_str().ok())
        .unwrap_or("?");
    format!("[{}] {}: {}", level, category, message)
}
