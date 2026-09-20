use eframe::egui;

pub struct PopupButton {
    pub label: String,
    pub action: PopupAction,
}

#[derive(Clone)]
pub enum PopupAction {
    None,
    Left,
    Right,
    Close,
    Confirm,
    Cancel,
    CopyText,
}

pub struct PopupParams<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub page_label: Option<String>,
    pub show_nav: bool,
    pub copy_text: Option<String>,
    pub buttons: Vec<PopupButton>,
    pub body: Box<dyn FnOnce(&mut egui::Ui) + 'a>,
}

pub fn popup(ui: &mut egui::Ui, params: PopupParams) -> PopupAction {
    let mut action = PopupAction::None;

    egui::Modal::new(egui::Id::new(params.id)).show(ui.ctx(), |ui| {
        ui.set_width(480.0);

        let copy_text = params.copy_text.clone();

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                if let Some(label) = &params.page_label {
                    ui.heading(label);
                } else {
                    ui.heading(params.title);
                }
            });
            if let Some(text) = copy_text {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Copy").clicked() {
                        ui.ctx().copy_text(text.clone());
                    }
                });
            }
        });
        ui.separator();

        if params.show_nav {
            ui.horizontal(|ui| {
                if ui.add_sized([40.0, 40.0], egui::Button::new("<")).clicked() {
                    action = PopupAction::Left;
                }

                ui.vertical(|ui| {
                    ui.set_width(360.0);
                    (params.body)(ui);
                });

                if ui.add_sized([40.0, 40.0], egui::Button::new(">")).clicked() {
                    action = PopupAction::Right;
                }
            });
        } else {
            (params.body)(ui);
        }

        ui.separator();

        ui.horizontal(|ui| {
            for btn in params.buttons {
                if ui.button(&btn.label).clicked() {
                    action = btn.action.clone();
                }
            }
        });
    });

    action
}
