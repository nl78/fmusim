use eframe::egui;

use crate::app::{FmusimApp, InterfaceKind, SimState};
use crate::logging::LogLevel;

pub fn show(app: &mut FmusimApp, ui: &mut egui::Ui) {
    let running = matches!(app.sim_state, SimState::Running { .. });

    egui::CollapsingHeader::new("Configuration")
        .default_open(true)
        .show(ui, |ui| {
            ui.add_enabled_ui(!running, |ui| {
                egui::Grid::new("cfg_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Stop time [s]");
                        ui.add(
                            egui::DragValue::new(&mut app.config.stop_time)
                                .speed(0.1)
                                .range(0.0..=f64::MAX),
                        );
                        ui.end_row();

                        ui.label("Step size [s]");
                        ui.add(
                            egui::DragValue::new(&mut app.config.step_size)
                                .speed(1e-4)
                                .range(1e-9..=f64::MAX)
                                .max_decimals(9),
                        );
                        ui.end_row();

                        ui.label("Interface");
                        egui::ComboBox::from_id_salt("interface_kind")
                            .selected_text(app.config.interface.label())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut app.config.interface,
                                    InterfaceKind::CoSimulation,
                                    InterfaceKind::CoSimulation.label(),
                                );
                                ui.add_enabled_ui(false, |ui| {
                                    ui.selectable_value(
                                        &mut app.config.interface,
                                        InterfaceKind::ModelExchange,
                                        InterfaceKind::ModelExchange.label(),
                                    );
                                });
                            });
                        ui.end_row();

                        ui.label("FMU logging");
                        ui.checkbox(&mut app.config.logging_on, "loggingOn");
                        ui.end_row();

                        ui.label("Log FMI calls");
                        ui.checkbox(&mut app.config.log_fmi_calls, "trace calls");
                        ui.end_row();

                        ui.label("Log level filter");
                        egui::ComboBox::from_id_salt("log_level_filter")
                            .selected_text(app.config.log_level_filter.label())
                            .show_ui(ui, |ui| {
                                for level in [
                                    LogLevel::Debug,
                                    LogLevel::Info,
                                    LogLevel::Warning,
                                    LogLevel::Error,
                                    LogLevel::Fatal,
                                ] {
                                    ui.selectable_value(
                                        &mut app.config.log_level_filter,
                                        level,
                                        level.label(),
                                    );
                                }
                            });
                        ui.end_row();
                    });
            });
        });
}
