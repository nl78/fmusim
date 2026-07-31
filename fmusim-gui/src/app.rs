use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::time::Instant;

use crossbeam_channel::{Receiver, unbounded};
use fmi_rs::model_description::FMIMajorVersion;

use crate::logging::{LogEntry, LogLevel};
use crate::messages::{SampleMsg, StatusMsg};
use crate::worker::{self, SimJob, SimJobConfig};

const MAX_LOG_ENTRIES: usize = 10_000;
const MAX_SERIES_POINTS: usize = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FmiVersion {
    V2,
    V3,
}

impl FmiVersion {
    pub fn label(self) -> &'static str {
        match self {
            FmiVersion::V2 => "FMI 2.0",
            FmiVersion::V3 => "FMI 3.0",
        }
    }
}

impl From<FMIMajorVersion> for FmiVersion {
    fn from(v: FMIMajorVersion) -> Self {
        match v {
            FMIMajorVersion::V2 => FmiVersion::V2,
            FMIMajorVersion::V3 => FmiVersion::V3,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VariableKind {
    Float,
    Integer,
    Boolean,
    Enumeration,
    String,
    Binary,
}

impl VariableKind {
    pub fn plottable(&self) -> bool {
        !matches!(self, VariableKind::String | VariableKind::Binary)
    }
}

#[derive(Clone, Debug)]
pub struct VariableInfo {
    pub name: String,
    pub description: Option<String>,
    pub causality: String,
    pub kind: VariableKind,
    pub is_output: bool,
}

#[derive(Clone, Debug)]
pub struct LoadedFmu {
    pub path: PathBuf,
    pub version: FmiVersion,
    pub model_name: String,
    pub variables: Vec<VariableInfo>,
    pub default_stop_time: Option<f64>,
    pub default_step_size: Option<f64>,
    pub supports_cs: bool,
    #[allow(dead_code)]
    pub supports_me: bool,
}

#[derive(Clone, Debug)]
pub struct StartValue {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterfaceKind {
    CoSimulation,
    ModelExchange,
}

impl InterfaceKind {
    pub fn label(self) -> &'static str {
        match self {
            InterfaceKind::CoSimulation => "Co-Simulation",
            InterfaceKind::ModelExchange => "Model Exchange (not yet supported)",
        }
    }
}

pub struct UiConfig {
    pub stop_time: f64,
    pub step_size: f64,
    pub logging_on: bool,
    pub log_fmi_calls: bool,
    pub interface: InterfaceKind,
    pub log_level_filter: LogLevel,
    pub auto_scroll_logs: bool,
    pub follow_plot: bool,
    pub start_values: Vec<StartValue>,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            stop_time: 1.0,
            step_size: 1e-3,
            logging_on: true,
            log_fmi_calls: false,
            interface: InterfaceKind::CoSimulation,
            log_level_filter: LogLevel::Info,
            auto_scroll_logs: true,
            follow_plot: true,
            start_values: Vec::new(),
        }
    }
}

pub enum SimState {
    Idle,
    Running { stop_time: f64, started_at: Instant },
    Finished { message: String, ok: bool },
}

#[derive(Default)]
pub struct LiveSeries {
    pub time: Vec<f64>,
    pub value: Vec<f64>,
}

impl LiveSeries {
    fn push(&mut self, t: f64, v: f64) {
        if self.time.len() >= MAX_SERIES_POINTS {
            let drop = MAX_SERIES_POINTS / 10;
            self.time.drain(..drop);
            self.value.drain(..drop);
        }
        self.time.push(t);
        self.value.push(v);
    }

    pub fn clear(&mut self) {
        self.time.clear();
        self.value.clear();
    }
}

pub struct FmusimApp {
    pub loaded: Option<LoadedFmu>,
    pub load_error: Option<String>,
    pub config: UiConfig,
    pub selected_outputs: Vec<String>,
    pub sim_state: SimState,
    pub logs: VecDeque<LogEntry>,
    pub series: HashMap<String, LiveSeries>,
    pub latest_sim_time: Option<f64>,

    job: Option<SimJob>,
    log_rx: Option<Receiver<LogEntry>>,
    sample_rx: Option<Receiver<SampleMsg>>,
    status_rx: Option<Receiver<StatusMsg>>,
}

impl Default for FmusimApp {
    fn default() -> Self {
        Self {
            loaded: None,
            load_error: None,
            config: UiConfig::default(),
            selected_outputs: Vec::new(),
            sim_state: SimState::Idle,
            logs: VecDeque::with_capacity(MAX_LOG_ENTRIES),
            series: HashMap::new(),
            latest_sim_time: None,
            job: None,
            log_rx: None,
            sample_rx: None,
            status_rx: None,
        }
    }
}

impl FmusimApp {
    pub fn load_fmu(&mut self, path: &Path) {
        self.load_error = None;
        match load_fmu(path) {
            Ok(fmu) => {
                if let Some(t) = fmu.default_stop_time {
                    self.config.stop_time = t;
                }
                if let Some(t) = fmu.default_step_size {
                    self.config.step_size = t;
                }
                self.selected_outputs = fmu
                    .variables
                    .iter()
                    .filter(|v| v.is_output && v.kind.plottable())
                    .map(|v| v.name.clone())
                    .collect();
                self.series.clear();
                for name in &self.selected_outputs {
                    self.series.insert(name.clone(), LiveSeries::default());
                }
                self.logs.clear();
                self.sim_state = SimState::Idle;
                self.loaded = Some(fmu);
            }
            Err(e) => {
                self.load_error = Some(e);
            }
        }
    }

    pub fn can_start(&self) -> bool {
        self.loaded.as_ref().is_some_and(|f| f.supports_cs)
            && matches!(self.sim_state, SimState::Idle | SimState::Finished { .. })
            && self.config.stop_time > 0.0
            && self.config.step_size > 0.0
            && !self.selected_outputs.is_empty()
    }

    pub fn start_simulation(&mut self) {
        let Some(fmu) = self.loaded.as_ref() else {
            return;
        };
        if !fmu.supports_cs {
            return;
        }

        // Reset live state.
        self.logs.clear();
        for series in self.series.values_mut() {
            series.clear();
        }
        for name in &self.selected_outputs {
            self.series.entry(name.clone()).or_default();
        }
        self.latest_sim_time = None;

        let (log_tx, log_rx) = unbounded();
        let (sample_tx, sample_rx) = unbounded();
        let (status_tx, status_rx) = unbounded();

        let cfg = SimJobConfig {
            fmu_path: fmu.path.clone(),
            selected_outputs: self.selected_outputs.clone(),
            stop_time: self.config.stop_time,
            output_interval: self.config.step_size,
            logging_on: self.config.logging_on,
            log_fmi_calls: self.config.log_fmi_calls,
            start_values: self.config.start_values.clone(),
        };

        self.log_rx = Some(log_rx);
        self.sample_rx = Some(sample_rx);
        self.status_rx = Some(status_rx);

        let job = worker::spawn(cfg, fmu.version, log_tx, sample_tx, status_tx);
        self.sim_state = SimState::Running {
            stop_time: self.config.stop_time,
            started_at: job.started_at,
        };
        self.job = Some(job);
    }

    pub fn cancel_simulation(&mut self) {
        if let Some(job) = &self.job {
            job.cancel
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    pub fn drain_channels(&mut self) {
        // Drain logs.
        if let Some(rx) = &self.log_rx {
            while let Ok(entry) = rx.try_recv() {
                if self.logs.len() == MAX_LOG_ENTRIES {
                    self.logs.pop_front();
                }
                self.logs.push_back(entry);
            }
        }
        // Drain samples.
        if let Some(rx) = &self.sample_rx {
            while let Ok(msg) = rx.try_recv() {
                self.latest_sim_time = Some(msg.time);
                for (name, opt_value) in self.selected_outputs.iter().zip(msg.values.iter()) {
                    if let Some(v) = opt_value {
                        let series = self.series.entry(name.clone()).or_default();
                        series.push(msg.time, *v);
                    }
                }
            }
        }
        // Drain status.
        if let Some(rx) = &self.status_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    StatusMsg::Started { stop_time, started_at } => {
                        self.sim_state = SimState::Running { stop_time, started_at };
                    }
                    StatusMsg::Finished(result) => {
                        let (ok, message) = match result {
                            Ok(()) => (true, "Simulation finished".to_owned()),
                            Err(e) => (false, format!("Simulation failed: {e}")),
                        };
                        self.sim_state = SimState::Finished { message, ok };
                        self.job = None;
                    }
                }
            }
        }
    }
}

impl eframe::App for FmusimApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_channels();

        crate::ui::toolbar::show(self, ctx);
        crate::ui::side_panel::show(self, ctx);
        crate::ui::central::show(self, ctx);

        if matches!(self.sim_state, SimState::Running { .. }) {
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        }
    }
}

fn load_fmu(path: &Path) -> Result<LoadedFmu, String> {
    let unzipdir = tempfile::TempDir::new()
        .map_err(|e| format!("Failed to create temp directory: {e}"))?;
    fmi_rs::zip::extract_zip_archive(path, &unzipdir)
        .map_err(|e| format!("Failed to extract FMU: {e}"))?;

    let xml_path = unzipdir.path().join("modelDescription.xml");
    let version = fmi_rs::model_description::peek_fmi_major_version(&xml_path)
        .map_err(|e| format!("Failed to read modelDescription.xml: {e}"))?;

    match version {
        FMIMajorVersion::V3 => load_fmu_v3(path.to_owned(), &xml_path),
        FMIMajorVersion::V2 => load_fmu_v2(path.to_owned(), &xml_path),
    }
}

fn load_fmu_v3(path: PathBuf, xml_path: &Path) -> Result<LoadedFmu, String> {
    use fmi_rs::model_description::fmi3::{Causality, ModelDescription};

    let md = ModelDescription::from_path(xml_path)
        .map_err(|e| format!("Failed to parse modelDescription.xml: {e}"))?;

    let variables = md
        .modelVariables
        .iter()
        .map(|v| VariableInfo {
            name: v.name.clone(),
            description: v.description.clone(),
            causality: format!("{:?}", v.causality),
            kind: kind_of_fmi3(&v.variableType),
            is_output: v.causality == Causality::Output,
        })
        .collect();

    let default_stop_time = md
        .defaultExperiment
        .as_ref()
        .and_then(|d| d.stopTime.as_ref())
        .and_then(|s| s.parse().ok());
    let default_step_size = md
        .defaultExperiment
        .as_ref()
        .and_then(|d| d.stepSize.as_ref())
        .and_then(|s| s.parse().ok());

    let supports_cs = md.coSimulation.is_some();
    let supports_me = md.modelExchange.is_some();
    let model_name = md.modelName.clone();

    Ok(LoadedFmu {
        path,
        version: FmiVersion::V3,
        model_name,
        variables,
        default_stop_time,
        default_step_size,
        supports_cs,
        supports_me,
    })
}

fn kind_of_fmi3(t: &fmi_rs::model_description::fmi3::VariableType) -> VariableKind {
    use fmi_rs::model_description::fmi3::VariableType as T;
    match t {
        T::Float32 { .. } | T::Float64 { .. } => VariableKind::Float,
        T::Int8 { .. }
        | T::UInt8 { .. }
        | T::Int16 { .. }
        | T::UInt16 { .. }
        | T::Int32 { .. }
        | T::UInt32 { .. }
        | T::Int64 { .. }
        | T::UInt64 { .. } => VariableKind::Integer,
        T::Boolean { .. } => VariableKind::Boolean,
        T::Enumeration { .. } => VariableKind::Enumeration,
        T::String { .. } => VariableKind::String,
        T::Binary { .. } => VariableKind::Binary,
        _ => VariableKind::Float,
    }
}

fn load_fmu_v2(path: PathBuf, xml_path: &Path) -> Result<LoadedFmu, String> {
    use fmi_rs::model_description::fmi2::{Causality, ModelDescription, VariableType};

    let md = ModelDescription::from_path(xml_path)
        .map_err(|e| format!("Failed to parse modelDescription.xml: {e}"))?;

    let variables = md
        .modelVariables
        .iter()
        .map(|v| VariableInfo {
            name: v.name.clone(),
            description: v.description.clone(),
            causality: format!("{:?}", v.causality),
            kind: match v.variableType {
                VariableType::Real { .. } => VariableKind::Float,
                VariableType::Integer { .. } => VariableKind::Integer,
                VariableType::Boolean { .. } => VariableKind::Boolean,
                VariableType::Enumeration { .. } => VariableKind::Enumeration,
                VariableType::String { .. } => VariableKind::String,
            },
            is_output: v.causality == Causality::Output,
        })
        .collect();

    let default_stop_time = md
        .defaultExperiment
        .as_ref()
        .and_then(|d| d.stopTime.as_ref())
        .and_then(|s| s.parse().ok());
    let default_step_size = md
        .defaultExperiment
        .as_ref()
        .and_then(|d| d.stepSize.as_ref())
        .and_then(|s| s.parse().ok());

    Ok(LoadedFmu {
        path,
        version: FmiVersion::V2,
        model_name: md.modelName.clone(),
        variables,
        default_stop_time,
        default_step_size,
        supports_cs: md.coSimulation.is_some(),
        supports_me: md.modelExchange.is_some(),
    })
}
