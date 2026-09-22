use eframe::egui;
use every_page_ever::MainPages::*;

#[derive(Clone)]
pub enum PageToRouteTo {
    Home,
}

pub fn literally_draw_the_page(page_to_route_to: PageToRouteTo, ui: &mut egui::Ui) {
    match page_to_route_to {
        PageToRouteTo::Home => HomePage::view::draw_the_page(ui),
    }
}
