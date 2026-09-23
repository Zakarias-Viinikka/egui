use std::sync::Arc;

use crate::draw_me::DrawMe;
use crate::smart_router::every_page::PageToRouteTo;
use eframe::egui::{self, mutex::Mutex};

pub fn literally_draw_the_page(page_to_route_to: Arc<Mutex<PageToRouteTo>>, ui: &mut egui::Ui) {
    let page_to_route_to = page_to_route_to.lock();
    match &*page_to_route_to {
        PageToRouteTo::Home(page) => page.draw_me(ui),
    }
}
