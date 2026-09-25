use std::sync::Arc;

use eframe::egui::{self, mutex::Mutex};
use every_page_ever::MainPages::*;

#[derive(Clone)]
pub enum PageToRouteTo {
    Home,
}

pub fn literally_draw_the_page(page_to_route_to: Arc<Mutex<PageToRouteTo>>, ui: &mut egui::Ui) {
    let page_to_route_to = page_to_route_to.lock();
    match *page_to_route_to {
        PageToRouteTo::Home => HomePage::view::draw_the_page(ui),
    }
}
