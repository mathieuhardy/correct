mod config;
mod dictionary;
mod ignored;
mod languagetool;
mod ui;
mod watcher;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("correct")
            .with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "correct",
        options,
        Box::new(|cc| Ok(Box::new(ui::app::App::new(cc)))),
    )
}
