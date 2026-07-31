use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Instant;

use crossbeam_channel::Sender;
use fmi_rs::model_description::FMIMajorVersion;
use tempfile::TempDir;

use crate::app::{FmiVersion, StartValue};
use crate::logging::{GuiLoggerFmi2, GuiLoggerFmi3, LogEntry};
use crate::messages::{SampleMsg, StatusMsg};
use crate::stream::{GuiObserverFmi2, GuiObserverFmi3};

pub struct SimJobConfig {
    pub fmu_path: PathBuf,
    pub selected_outputs: Vec<String>,
    pub stop_time: f64,
    pub output_interval: f64,
    pub logging_on: bool,
    pub log_fmi_calls: bool,
    pub start_values: Vec<StartValue>,
}

pub struct SimJob {
    #[allow(dead_code)]
    pub handle: JoinHandle<()>,
    pub cancel: Arc<AtomicBool>,
    pub started_at: Instant,
}

pub fn spawn(
    config: SimJobConfig,
    version_hint: FmiVersion,
    log_tx: Sender<LogEntry>,
    sample_tx: Sender<SampleMsg>,
    status_tx: Sender<StatusMsg>,
) -> SimJob {
    let cancel = Arc::new(AtomicBool::new(false));
    let started_at = Instant::now();
    let _ = status_tx.send(StatusMsg::Started {
        stop_time: config.stop_time,
        started_at,
    });

    let cancel_for_thread = cancel.clone();
    let handle = thread::spawn(move || {
        let result = run(config, version_hint, log_tx, sample_tx, cancel_for_thread);
        let _ = status_tx.send(StatusMsg::Finished(result));
    });

    SimJob {
        handle,
        cancel,
        started_at,
    }
}

fn run(
    config: SimJobConfig,
    version_hint: FmiVersion,
    log_tx: Sender<LogEntry>,
    sample_tx: Sender<SampleMsg>,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    let unzipdir = TempDir::new().map_err(|e| format!("Failed to create temp directory: {e}"))?;
    fmi_rs::zip::extract_zip_archive(&config.fmu_path, &unzipdir)
        .map_err(|e| format!("Failed to extract FMU: {e}"))?;

    let xml_path = unzipdir.path().join("modelDescription.xml");
    let detected = fmi_rs::model_description::peek_fmi_major_version(&xml_path)
        .map_err(|e| format!("Failed to read modelDescription.xml: {e}"))?;

    // We trust the version detected from the archive (defensive against stale hints).
    let _ = version_hint;

    match detected {
        FMIMajorVersion::V3 => run_v3(&config, &unzipdir, &xml_path, log_tx, sample_tx, cancel),
        FMIMajorVersion::V2 => run_v2(&config, &unzipdir, &xml_path, log_tx, sample_tx, cancel),
    }
}

fn run_v3(
    config: &SimJobConfig,
    unzipdir: &TempDir,
    xml_path: &std::path::Path,
    log_tx: Sender<LogEntry>,
    sample_tx: Sender<SampleMsg>,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    use fmi_rs::model_description::fmi3::{Causality, ModelDescription};
    use fmi_rs::sim::fmi3::{SimulationSettings, Trajectories, recorder::Recorder};

    let md = ModelDescription::from_path(xml_path)
        .map_err(|e| format!("Failed to parse modelDescription.xml: {e}"))?;

    let outputs: Vec<&fmi_rs::model_description::fmi3::ModelVariable> = if config
        .selected_outputs
        .is_empty()
    {
        md.modelVariables
            .iter()
            .filter(|v| v.causality == Causality::Output)
            .collect()
    } else {
        let mut resolved = Vec::with_capacity(config.selected_outputs.len());
        for name in &config.selected_outputs {
            let var = md
                .modelVariables
                .iter()
                .find(|v| &v.name == name)
                .ok_or_else(|| format!("Unknown output variable: {name}"))?;
            resolved.push(var);
        }
        resolved
    };

    let settings = SimulationSettings {
        unzipdir: unzipdir.path(),
        model_description: &md,
        start_time: 0.0,
        stop_time: config.stop_time,
        logging_on: config.logging_on,
        set_stop_time: true,
        output_interval: config.output_interval,
        tolerance: None,
        start_values: config
            .start_values
            .iter()
            .map(|sv| (sv.name.clone(), sv.value.clone()))
            .collect(),
        log_fmi_calls: config.log_fmi_calls,
        input_file: None,
        early_return_allowed: false,
        event_mode_used: false,
        log_file: None,
        initial_fmu_state_file: None,
        final_fmu_state_file: None,
        cancel: Some(cancel),
    };

    let mut trajectories = Trajectories::new(&md, outputs);
    let mut recorder =
        Recorder::with_observer(&mut trajectories, Box::new(GuiObserverFmi3::new(sample_tx)));

    let logger = Box::new(GuiLoggerFmi3::new(log_tx));

    fmi_rs::sim::fmi3::cs::simulate_with_logger(&settings, None, &mut recorder, logger)
        .map_err(|e| e.to_string())
}

fn run_v2(
    config: &SimJobConfig,
    unzipdir: &TempDir,
    xml_path: &std::path::Path,
    log_tx: Sender<LogEntry>,
    sample_tx: Sender<SampleMsg>,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    use fmi_rs::model_description::fmi2::{Causality, ModelDescription};
    use fmi_rs::sim::fmi2::{SimulationSettings, Trajectories, recorder::Recorder};

    let md = ModelDescription::from_path(xml_path)
        .map_err(|e| format!("Failed to parse modelDescription.xml: {e}"))?;

    let outputs: Vec<&fmi_rs::model_description::fmi2::ScalarVariable> = if config
        .selected_outputs
        .is_empty()
    {
        md.modelVariables
            .iter()
            .filter(|v| v.causality == Causality::Output)
            .collect()
    } else {
        let mut resolved = Vec::with_capacity(config.selected_outputs.len());
        for name in &config.selected_outputs {
            let var = md
                .modelVariables
                .iter()
                .find(|v| &v.name == name)
                .ok_or_else(|| format!("Unknown output variable: {name}"))?;
            resolved.push(var);
        }
        resolved
    };

    let settings = SimulationSettings {
        unzipdir: unzipdir.path(),
        model_description: &md,
        start_time: 0.0,
        stop_time: config.stop_time,
        logging_on: config.logging_on,
        set_stop_time: true,
        output_interval: config.output_interval,
        tolerance: None,
        start_values: config
            .start_values
            .iter()
            .map(|sv| (sv.name.clone(), sv.value.clone()))
            .collect(),
        log_fmi_calls: config.log_fmi_calls,
        input_file: None,
        early_return_allowed: false,
        event_mode_used: false,
        log_file: None,
        initial_fmu_state_file: None,
        final_fmu_state_file: None,
        cancel: Some(cancel),
    };

    let mut trajectories = Trajectories::new(&md, outputs);
    let mut recorder =
        Recorder::with_observer(&mut trajectories, Box::new(GuiObserverFmi2::new(sample_tx)));

    let logger = Box::new(GuiLoggerFmi2::new(log_tx));

    fmi_rs::sim::fmi2::cs::simulate_with_logger(&settings, None, &mut recorder, logger)
        .map_err(|e| e.to_string())
}
