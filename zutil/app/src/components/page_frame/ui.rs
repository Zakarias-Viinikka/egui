use eframe::egui;
use design::colors::SURFACE;
use design::numbers::{CORNER_MEDIUM, PAD_LARGE, PAGE_INSET_FRACTION};

pub fn page_frame_ui(ui: &mut egui::Ui) -> egui::Rect {
    let screen = ui.available_rect_before_wrap();
    let w = screen.width() * (1.0 - PAGE_INSET_FRACTION * 2.0);
    let h = screen.height() * (1.0 - PAGE_INSET_FRACTION * 2.0);
    let x = screen.left() + (screen.width() - w) / 2.0;
    let y = screen.top() + (screen.height() - h) / 2.0;
    let rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h));

    ui.painter().rect_filled(rect, CORNER_MEDIUM, SURFACE);

    rect.shrink(PAD_LARGE)
}

pub fn page_frame_opaque_ui(ui: &mut egui::Ui) -> egui::Rect {
    let screen = ui.available_rect_before_wrap();
    let w = screen.width() * (1.0 - PAGE_INSET_FRACTION * 2.0);
    let h = screen.height() * (1.0 - PAGE_INSET_FRACTION * 2.0);
    let x = screen.left() + (screen.width() - w) / 2.0;
    let y = screen.top() + (screen.height() - h) / 2.0;
    let rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h));

    ui.painter().rect_filled(rect, CORNER_MEDIUM, design::colors::SURFACE_OPAQUE);

    rect.shrink(PAD_LARGE)
}
