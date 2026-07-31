use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints};

use crate::app::FmusimApp;

pub fn show(app: &mut FmusimApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.heading("Output variables");
        ui.separator();
        ui.checkbox(&mut app.config.follow_plot, "Follow latest");
    });

    if app.selected_outputs.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label("Select at least one variable in the side panel to plot it here.");
        });
        return;
    }

    let mut plot = Plot::new("live_plot")
        .legend(Legend::default())
        .allow_zoom(true)
        .allow_drag(true)
        .x_axis_label("time [s]");

    if let (true, Some(t)) = (app.config.follow_plot, app.latest_sim_time) {
        let window = app.config.stop_time.max(t).max(1e-6);
        plot = plot.include_x(0.0).include_x(window);
    }

    plot.show(ui, |plot_ui| {
        for name in &app.selected_outputs {
            let Some(series) = app.series.get(name) else {
                continue;
            };
            if series.time.is_empty() {
                continue;
            }
            let points: PlotPoints = series
                .time
                .iter()
                .zip(series.value.iter())
                .map(|(t, v)| [*t, *v])
                .collect();
            plot_ui.line(Line::new(points).name(name));
        }
    });
}
