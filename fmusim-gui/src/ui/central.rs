use eframe::egui;

use crate::app::FmusimApp;

pub fn show(app: &mut FmusimApp, ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("log_panel")
        .resizable(true)
        .default_height(220.0)
        .show(ctx, |ui| {
            super::log_panel::show(app, ui);
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        super::plot_panel::show(app, ui);
    });
}
