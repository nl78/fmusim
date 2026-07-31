#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

use eframe::egui;

mod app;
mod logging;
mod messages;
mod stream;
mod ui;
mod worker;

use app::FmusimApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("fmusim GUI"),
        ..Default::default()
    };
    eframe::run_native(
        "fmusim GUI",
        options,
        Box::new(|_cc| Ok(Box::new(FmusimApp::default()))),
    )
}
