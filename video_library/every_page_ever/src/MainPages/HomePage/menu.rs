use super::actions::Actions;
use app_core::kdenlive::KdenliveStatus;
use app_core::state::video_library_state::VideoLibraryState;
use app_core::BwPreset;
use eframe::egui;

const BTN_SIZE: f32 = 15.0;
const LABEL_SIZE: f32 = 14.0;

pub fn draw(ui: &mut egui::Ui, state: &mut VideoLibraryState, actions: &mut Actions) {
    let connected = state.kdenlive.is_connected();
    let pulling = state.kdenlive.pull_job.is_some();
    let pushing = state.kdenlive.push_job.is_some();
    let busy = pulling || pushing;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        ui.spacing_mut().button_padding = egui::vec2(14.0, 8.0);

        if ui
            .button(egui::RichText::new("📂 Open video").size(BTN_SIZE))
            .clicked()
        {
            actions.open_video_picker = true;
        }

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        let pull_label = if pulling { "⏳ Pulling..." } else { "⬇ Pull from Kdenlive" };
        if ui
            .add_enabled(
                connected && !busy,
                egui::Button::new(egui::RichText::new(pull_label).size(BTN_SIZE)),
            )
            .on_hover_text("Import the clip currently selected in Kdenlive")
            .clicked()
        {
            actions.pull_from_kdenlive = true;
        }

        let push_label = if pushing { "⏳ Sending..." } else { "⬆ Add to Kdenlive bin" };
        let has_output = state.last_render_output.is_some() || state.picked_path.is_some();
        if ui
            .add_enabled(
                connected && has_output && !busy,
                egui::Button::new(egui::RichText::new(push_label).size(BTN_SIZE)),
            )
            .on_hover_text("Import the current file into Kdenlive's project bin")
            .clicked()
        {
            actions.push_to_kdenlive = true;
        }

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        let bw_enabled = state.picked_path.is_some() && !state.is_rendering();
        if ui
            .add_enabled(
                bw_enabled,
                egui::Button::new(egui::RichText::new("🎨 Black and White").size(BTN_SIZE)),
            )
            .on_hover_text("Saves next to the current file as <name>_bw.mp4")
            .clicked()
        {
            actions.start_bw_render = true;
        }

        ui.menu_button(egui::RichText::new("⚙").size(BTN_SIZE), |ui| {
            ui.label("Black and White quality");
            ui.separator();
            for preset in BwPreset::all() {
                let selected = state.bw_preset == preset;
                if ui.selectable_label(selected, preset.label()).clicked() {
                    state.bw_preset = preset;
                    ui.close();
                }
            }
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (color, label) = match &state.kdenlive.status {
                KdenliveStatus::Connected { name } => {
                    let pid = name
                        .strip_prefix("org.kde.kdenlive-")
                        .unwrap_or(name.as_str());
                    (
                        egui::Color32::from_rgb(120, 220, 140),
                        format!("Kdenlive (PID {pid})"),
                    )
                }
                KdenliveStatus::NotRunning => (
                    egui::Color32::from_gray(130),
                    "Kdenlive not running".to_string(),
                ),
                KdenliveStatus::Multiple { count, .. } => (
                    egui::Color32::from_rgb(240, 120, 120),
                    format!("{count} Kdenlive instances"),
                ),
                KdenliveStatus::Checking => (
                    egui::Color32::from_gray(150),
                    "Checking...".to_string(),
                ),
                KdenliveStatus::Error(msg) => (
                    egui::Color32::from_rgb(240, 120, 120),
                    format!("Kdenlive: {msg}"),
                ),
            };

            ui.label(egui::RichText::new(&label).size(LABEL_SIZE).color(color));

            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
            ui.painter().circle_filled(rect.center(), 6.0, color);
        });
    });
}