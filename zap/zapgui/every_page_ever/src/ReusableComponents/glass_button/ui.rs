use eframe::egui;

const SIZE: f32 = 44.0;
const CORNER: f32 = 10.0;
// from_rgba_premultiplied(r*a/255, g*a/255, b*a/255, a)
const FILL: egui::Color32 = egui::Color32::from_rgba_premultiplied(12, 12, 12, 110);
const FILL_HOVER: egui::Color32 = egui::Color32::from_rgba_premultiplied(37, 37, 37, 160);
const BORDER: egui::Color32 = egui::Color32::from_rgba_premultiplied(50, 50, 50, 50);
const BORDER_HOVER: egui::Color32 = egui::Color32::from_rgba_premultiplied(110, 110, 110, 110);
const GREEN: egui::Color32 = egui::Color32::from_rgb(80, 180, 80);
const RED: egui::Color32 = egui::Color32::from_rgb(210, 80, 80);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Confirm,
    Deny,
}

pub fn glass_button_ui(
    ui: &mut egui::Ui,
    center: egui::Pos2,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    icon: Icon,
) -> bool {
    let rect = egui::Rect::from_center_size(center, egui::vec2(SIZE, SIZE));
    let id = egui::Id::new("glass_button").with(id_salt);
    let response = ui.interact(rect, id, egui::Sense::click());

    let (fill, border) = if response.hovered() {
        (FILL_HOVER, BORDER_HOVER)
    } else {
        (FILL, BORDER)
    };

    ui.painter().rect(
        rect,
        CORNER,
        fill,
        egui::Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );

    let c = rect.center();
    let half = 8.0;
    let stroke = egui::Stroke::new(2.0, match icon {
        Icon::Confirm => GREEN,
        Icon::Deny => RED,
    });
    match icon {
        Icon::Confirm => {
            let a = egui::pos2(c.x - half, c.y);
            let b = egui::pos2(c.x - half * 0.3, c.y + half * 0.7);
            let d = egui::pos2(c.x + half, c.y - half * 0.7);
            ui.painter().line_segment([a, b], stroke);
            ui.painter().line_segment([b, d], stroke);
        }
        Icon::Deny => {
            let a = egui::pos2(c.x - half, c.y - half);
            let b = egui::pos2(c.x + half, c.y + half);
            let cc = egui::pos2(c.x + half, c.y - half);
            let d = egui::pos2(c.x - half, c.y + half);
            ui.painter().line_segment([a, b], stroke);
            ui.painter().line_segment([cc, d], stroke);
        }
    }

    let label = match icon {
        Icon::Confirm => "confirm",
        Icon::Deny => "deny",
    };
    response.on_hover_text(label).clicked()
}

/// A small glass-style pill: a rounded rectangle with a label, sized to
/// its text. Used for the full-file toggle and any other capsule control.
pub fn pill_ui(
    ui: &mut egui::Ui,
    center: egui::Pos2,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    label: &str,
    active: bool,
) -> bool {
    let font = egui::FontId::monospace(12.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, egui::Color32::WHITE);
    let pad_x = 10.0;
    let pad_y = 4.0;
    let size = egui::vec2(
        galley.size().x + pad_x * 2.0,
        galley.size().y + pad_y * 2.0,
    );
    let rect = egui::Rect::from_center_size(center, size);
    let id = egui::Id::new("glass_pill").with(id_salt);
    let response = ui.interact(rect, id, egui::Sense::click());

    let (fill, border) = if active {
        (FILL_SELECTED, BORDER_SELECTED)
    } else if response.hovered() {
        (FILL_HOVER, BORDER_HOVER)
    } else {
        (FILL, BORDER)
    };

    ui.painter().rect(
        rect,
        size.y / 2.0,
        fill,
        egui::Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );
    ui.painter().galley(
        rect.center() - galley.size() / 2.0,
        galley,
        egui::Color32::from_rgb(220, 220, 220),
    );

    response.clicked()
}

const FILL_SELECTED: egui::Color32 = egui::Color32::from_rgba_premultiplied(60, 60, 60, 200);
const BORDER_SELECTED: egui::Color32 = egui::Color32::from_rgba_premultiplied(170, 170, 170, 200);
const FILL_DENIED: egui::Color32 = egui::Color32::from_rgba_premultiplied(91, 35, 35, 110);
const BORDER_DENIED: egui::Color32 = egui::Color32::from_rgba_premultiplied(181, 69, 69, 220);
const FILL_DENIED_SELECTED: egui::Color32 = egui::Color32::from_rgba_premultiplied(110, 42, 42, 130);
const BORDER_DENIED_SELECTED: egui::Color32 = egui::Color32::from_rgba_premultiplied(230, 88, 88, 255);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockState {
    Normal,
    Selected,
    Denied,
    DeniedSelected,
}

pub fn glass_step_block_ui(
    ui: &mut egui::Ui,
    center: egui::Pos2,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    state: BlockState,
) -> bool {
    let rect = egui::Rect::from_center_size(center, egui::vec2(SIZE, SIZE));
    let id = egui::Id::new("glass_step_block").with(id_salt);
    let response = ui.interact(rect, id, egui::Sense::click());

    let (fill, border) = match state {
        BlockState::Normal => (FILL, BORDER),
        BlockState::Selected => (FILL_SELECTED, BORDER_SELECTED),
        BlockState::Denied => (FILL_DENIED, BORDER_DENIED),
        BlockState::DeniedSelected => (FILL_DENIED_SELECTED, BORDER_DENIED_SELECTED),
    };

    ui.painter().rect(
        rect,
        CORNER,
        fill,
        egui::Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );

    response.clicked()
}
