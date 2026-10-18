#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("RelicSift")
            .with_inner_size([1380.0, 880.0])
            .with_min_inner_size([980.0, 650.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "RelicSift",
        options,
        Box::new(|cc| Ok(Box::new(relicsift::app::RelicSift::new(cc)))),
    )
}
