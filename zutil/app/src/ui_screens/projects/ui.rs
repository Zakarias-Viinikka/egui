use db_wrapper::mascot::LiveForever;
use eframe::egui;
use error_stuff::unwrap_or_bail;
use zutil_db::helpers::project::read_all_projects;

pub struct ProjectsState {
    pub rows: Vec<ProjectRow>,
    pub show_modal: bool,
    pub editing_id: Option<i64>,
    pub form_title: String,
    pub form_path: String,
    pub form_launch_zed: bool,
    pub form_launch_adstud: bool,
}

pub struct ProjectRow {
    pub id: i64,
    pub title: String,
    pub path: String,
    pub launch_zed: bool,
    pub launch_adstud: bool,
}

impl Default for ProjectsState {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            show_modal: false,
            editing_id: None,
            form_title: String::new(),
            form_path: String::new(),
            form_launch_zed: false,
            form_launch_adstud: false,
        }
    }
}

impl ProjectsState {
    pub fn reload(&mut self, db: &LiveForever) {
        let rows = unwrap_or_bail!(load_rows(db), "projects", "reload");
        self.rows = rows;
    }

    fn open_new(&mut self) {
        self.show_modal = true;
        self.editing_id = None;
        self.form_title.clear();
        self.form_path.clear();
        self.form_launch_zed = false;
        self.form_launch_adstud = false;
    }

    fn open_edit(&mut self, row: &ProjectRow) {
        self.show_modal = true;
        self.editing_id = Some(row.id);
        self.form_title = row.title.clone();
        self.form_path = row.path.clone();
        self.form_launch_zed = row.launch_zed;
        self.form_launch_adstud = row.launch_adstud;
    }
}

pub enum ProjectsAction {
    None,
    Back,
    Create {
        title: String,
        path: String,
        launch_zed: bool,
        launch_adstud: bool,
    },
    Update {
        id: i64,
        title: String,
        path: String,
        launch_zed: bool,
        launch_adstud: bool,
    },
    Delete(i64),
    OpenTerminal { path: String },
    LaunchZed { path: String },
    LaunchAdstud { path: String },
}

pub fn projects_ui(
    ui: &mut egui::Ui,
    db: &LiveForever,
    state: &mut ProjectsState,
) -> ProjectsAction {
    let mut action = ProjectsAction::None;

    ui.horizontal(|ui| {
        if ui.button("Back").clicked() {
            action = ProjectsAction::Back;
        }
        ui.separator();
        ui.heading("Projects");
    });

    ui.separator();

    const BOTTOM_HEIGHT: f32 = 70.0;
    let avail = ui.available_size();
    let scroll_height = (avail.y - BOTTOM_HEIGHT).max(0.0);

    egui::ScrollArea::vertical()
        .max_height(scroll_height)
        .show(ui, |ui| {
            let mut edit_target: Option<(i64, String, String, bool, bool)> = None;
            for row in &state.rows {
                ui.horizontal(|ui| {
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if ui.button("Delete").clicked() {
                                action = ProjectsAction::Delete(row.id);
                            }
                            if ui.button("Edit").clicked() {
                                edit_target = Some((
                                    row.id,
                                    row.title.clone(),
                                    row.path.clone(),
                                    row.launch_zed,
                                    row.launch_adstud,
                                ));
                            }
                            if row.launch_adstud && ui.button("adstud").clicked() {
                                action = ProjectsAction::LaunchAdstud {
                                    path: row.path.clone(),
                                };
                            }
                            if row.launch_zed && ui.button("zed").clicked() {
                                action = ProjectsAction::LaunchZed {
                                    path: row.path.clone(),
                                };
                            }
                            let rest = ui.available_width();
                            let label = format!("{}   {}", row.title, row.path);
                            let resp = ui.add_sized(
                                [rest, 28.0],
                                egui::Button::new(label),
                            );
                            if resp.clicked() {
                                action = ProjectsAction::OpenTerminal {
                                    path: row.path.clone(),
                                };
                            }
                        },
                    );
                });
            }
            if let Some((id, title, path, lz, la)) = edit_target {
                state.show_modal = true;
                state.editing_id = Some(id);
                state.form_title = title;
                state.form_path = path;
                state.form_launch_zed = lz;
                state.form_launch_adstud = la;
            }
        });

    ui.add_space(8.0);
    let width = ui.available_width();
    if ui.add_sized([width, 40.0], egui::Button::new("+")).clicked() {
        state.open_new();
    }

    if state.show_modal {
        egui::Modal::new(egui::Id::new("project_modal")).show(ui.ctx(), |ui| {
            ui.set_width(400.0);
            ui.heading(if state.editing_id.is_some() {
                "Edit project"
            } else {
                "New project"
            });
            ui.separator();
            ui.label("Title");
            ui.text_edit_singleline(&mut state.form_title);
            ui.label("Path");
            ui.text_edit_singleline(&mut state.form_path);
            ui.checkbox(&mut state.form_launch_zed, "Launch zed");
            ui.checkbox(&mut state.form_launch_adstud, "Launch adstud");
            ui.separator();
            ui.horizontal(|ui| {
                let can_confirm =
                    !state.form_title.is_empty() && !state.form_path.is_empty();
                if ui.button("Confirm").clicked() && can_confirm {
                    let title = state.form_title.clone();
                    let path = state.form_path.clone();
                    let lz = state.form_launch_zed;
                    let la = state.form_launch_adstud;
                    action = match state.editing_id {
                        Some(id) => ProjectsAction::Update {
                            id,
                            title,
                            path,
                            launch_zed: lz,
                            launch_adstud: la,
                        },
                        None => ProjectsAction::Create {
                            title,
                            path,
                            launch_zed: lz,
                            launch_adstud: la,
                        },
                    };
                    state.show_modal = false;
                }
                if ui.button("Cancel").clicked() {
                    state.show_modal = false;
                }
            });
        });
    }

    action
}

fn load_rows(db: &LiveForever) -> Result<Vec<ProjectRow>, String> {
    let rows = read_all_projects(db).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        let id = *r.cols[0].as_int().map_err(|e| e)?;
        let title = r.cols[1].as_str().map_err(|e| e)?.to_string();
        let path = r.cols[2].as_str().map_err(|e| e)?.to_string();
        let launch_zed = r
            .cols
            .get(3)
            .and_then(|c| c.as_int().ok())
            .map(|v| *v != 0)
            .unwrap_or(false);
        let launch_adstud = r
            .cols
            .get(4)
            .and_then(|c| c.as_int().ok())
            .map(|v| *v != 0)
            .unwrap_or(false);
        out.push(ProjectRow {
            id,
            title,
            path,
            launch_zed,
            launch_adstud,
        });
    }
    Ok(out)
}
