use eframe::egui;

use crate::app::{FmusimApp, SimState};

pub fn show(app: &mut FmusimApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if ui.button("Load FMU…").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("FMU", &["fmu"])
                    .pick_file()
            {
                app.load_fmu(&path);
            }

            match &app.loaded {
                Some(fmu) => {
                    ui.label(fmu.path.display().to_string());
                    ui.separator();
                    ui.label(format!("{} · {}", fmu.model_name, fmu.version.label()));
                }
                None => {
                    ui.label("(no FMU loaded)");
                }
            }

            ui.separator();

            let running = matches!(app.sim_state, SimState::Running { .. });
            let can_start = app.can_start();

            ui.add_enabled_ui(!running && can_start, |ui| {
                if ui.button("▶ Start").clicked() {
                    app.start_simulation();
                }
            });
            ui.add_enabled_ui(running, |ui| {
                if ui.button("■ Stop").clicked() {
                    app.cancel_simulation();
                }
            });

            ui.separator();
            show_status(app, ui);
        });

        if let Some(err) = &app.load_error {
            ui.colored_label(egui::Color32::LIGHT_RED, err);
        }
    });
}

fn show_status(app: &FmusimApp, ui: &mut egui::Ui) {
    match &app.sim_state {
        SimState::Idle => {
            ui.label("Idle");
        }
        SimState::Running {
            stop_time,
            started_at,
        } => {
            let t = app.latest_sim_time.unwrap_or(0.0);
            let progress = if *stop_time > 0.0 {
                (t / stop_time).clamp(0.0, 1.0) as f32
            } else {
                0.0
            };
            ui.add(
                egui::ProgressBar::new(progress)
                    .desired_width(200.0)
                    .text(format!("t = {:.3} / {:.3} s", t, stop_time)),
            );
            let elapsed = started_at.elapsed().as_secs_f64();
            ui.label(format!("elapsed {:.1}s", elapsed));
        }
        SimState::Finished { message, ok } => {
            let color = if *ok {
                egui::Color32::LIGHT_GREEN
            } else {
                egui::Color32::LIGHT_RED
            };
            ui.colored_label(color, message);
        }
    }
}
