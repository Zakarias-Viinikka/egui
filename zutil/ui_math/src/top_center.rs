use eframe::egui;

pub fn top_center_of_screen(ctx: &egui::Context, y_offset: f32) -> egui::Pos2 {
    let screen = ctx.viewport_rect();
    egui::pos2(screen.center().x, screen.top() + y_offset)
}

pub fn adjust_for_centered_width(pos: egui::Pos2, width: f32) -> egui::Pos2 {
    egui::pos2(pos.x - width / 2.0, pos.y)
}
