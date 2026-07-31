use eframe::egui;

use crate::app::FmusimApp;

pub fn show(app: &mut FmusimApp, ctx: &egui::Context) {
    egui::SidePanel::left("side_panel")
        .resizable(true)
        .default_width(320.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                super::config_panel::show(app, ui);
                ui.separator();
                super::variables_panel::show(app, ui);
            });
        });
}
