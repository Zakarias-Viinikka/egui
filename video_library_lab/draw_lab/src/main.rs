mod app;
mod canvas;
mod guess;
mod strokes;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "draw_lab",
        options,
        Box::new(|_cc| Ok(Box::new(app::DrawLab::default()))),
    )
}