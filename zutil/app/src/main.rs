mod globals;
mod app;
mod components;
mod ui_screens;

use zutil_db::init::init_db;

fn main() -> eframe::Result<()> {
    let db = init_db();
    app::run(db)
}
