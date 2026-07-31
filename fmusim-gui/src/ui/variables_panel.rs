use eframe::egui;

use crate::app::{FmusimApp, SimState};

pub fn show(app: &mut FmusimApp, ui: &mut egui::Ui) {
    let running = matches!(app.sim_state, SimState::Running { .. });

    egui::CollapsingHeader::new("Variables")
        .default_open(true)
        .show(ui, |ui| {
            let Some(fmu) = app.loaded.as_ref() else {
                ui.label("Load an FMU to see its variables.");
                return;
            };

            ui.horizontal(|ui| {
                ui.add_enabled_ui(!running, |ui| {
                    if ui.small_button("Select all outputs").clicked() {
                        let names: Vec<String> = fmu
                            .variables
                            .iter()
                            .filter(|v| v.is_output && v.kind.plottable())
                            .map(|v| v.name.clone())
                            .collect();
                        app.selected_outputs = names.clone();
                        for name in names {
                            app.series.entry(name).or_default();
                        }
                    }
                    if ui.small_button("Clear").clicked() {
                        app.selected_outputs.clear();
                    }
                });
            });

            ui.separator();

            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    for variable in &fmu.variables {
                        let plottable = variable.kind.plottable();
                        let mut selected = app.selected_outputs.contains(&variable.name);
                        let response = ui.add_enabled(
                            plottable && !running,
                            egui::Checkbox::new(
                                &mut selected,
                                format!(
                                    "{} ({}, {:?})",
                                    variable.name, variable.causality, variable.kind
                                ),
                            ),
                        );
                        if let Some(desc) = &variable.description {
                            response.on_hover_text(desc);
                        }
                        if plottable && !running {
                            let currently_in = app.selected_outputs.contains(&variable.name);
                            if selected && !currently_in {
                                app.selected_outputs.push(variable.name.clone());
                                app.series.entry(variable.name.clone()).or_default();
                            } else if !selected && currently_in {
                                app.selected_outputs.retain(|n| n != &variable.name);
                            }
                        }
                    }
                });
        });
}
