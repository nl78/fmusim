use eframe::egui;

use crate::app::{FmusimApp, SimState};

pub fn show(app: &mut FmusimApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.heading("Logs");
        ui.separator();
        ui.checkbox(&mut app.config.auto_scroll_logs, "Auto-scroll");
        ui.separator();
        ui.label(format!("{} entries", app.logs.len()));
        if ui.small_button("Clear").clicked() {
            app.logs.clear();
        }
    });
    ui.separator();

    let level_filter = app.config.log_level_filter;
    let started_at = match app.sim_state {
        SimState::Running { started_at, .. } => Some(started_at),
        _ => None,
    };

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .stick_to_bottom(app.config.auto_scroll_logs)
        .show(ui, |ui| {
            for entry in app.logs.iter().filter(|e| e.level >= level_filter) {
                ui.horizontal(|ui| {
                    if let Some(t0) = started_at {
                        let dt = entry.at.saturating_duration_since(t0).as_secs_f64();
                        ui.label(format!("+{:.3}s", dt));
                    }
                    ui.colored_label(entry.level.color(), format!("[{}]", entry.level.label()));
                    ui.label(format!("[{}]", entry.category));
                    ui.label(&entry.message);
                });
            }
        });
}
