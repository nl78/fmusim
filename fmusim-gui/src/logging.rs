use std::time::Instant;

use crossbeam_channel::Sender;
use fmi_rs::fmi2::types::fmi2Status;
use fmi_rs::fmi3::types::fmi3Status;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Fatal,
}

impl LogLevel {
    pub fn label(self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warning => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
        }
    }

    pub fn color(self) -> egui::Color32 {
        match self {
            LogLevel::Debug => egui::Color32::LIGHT_GRAY,
            LogLevel::Info => egui::Color32::from_rgb(120, 180, 255),
            LogLevel::Warning => egui::Color32::from_rgb(255, 200, 80),
            LogLevel::Error => egui::Color32::from_rgb(255, 100, 100),
            LogLevel::Fatal => egui::Color32::from_rgb(255, 60, 60),
        }
    }
}

pub struct LogEntry {
    /// Wall-clock instant at which the message was produced.
    pub at: Instant,
    pub level: LogLevel,
    pub category: String,
    pub message: String,
}

/// Adapter from `fmi_rs::fmi3::log::Logger` to a channel of `LogEntry`.
pub struct GuiLoggerFmi3 {
    tx: Sender<LogEntry>,
}

impl GuiLoggerFmi3 {
    pub fn new(tx: Sender<LogEntry>) -> Self {
        Self { tx }
    }
}

impl fmi_rs::fmi3::log::Logger for GuiLoggerFmi3 {
    fn log_call(&self, _status: fmi3Status, message: &str) {
        let _ = self.tx.send(LogEntry {
            at: Instant::now(),
            level: LogLevel::Debug,
            category: "FMI".to_owned(),
            message: message.to_owned(),
        });
    }

    fn log_message(&self, status: fmi3Status, category: &str, message: &str) {
        let level = match status {
            fmi3Status::fmi3OK | fmi3Status::fmi3Pending => LogLevel::Info,
            fmi3Status::fmi3Warning => LogLevel::Warning,
            fmi3Status::fmi3Discard | fmi3Status::fmi3Error => LogLevel::Error,
            fmi3Status::fmi3Fatal => LogLevel::Fatal,
        };
        let _ = self.tx.send(LogEntry {
            at: Instant::now(),
            level,
            category: category.to_owned(),
            message: message.trim_end().to_owned(),
        });
    }
}

/// Adapter from `fmi_rs::fmi2::log::Logger` to a channel of `LogEntry`.
pub struct GuiLoggerFmi2 {
    tx: Sender<LogEntry>,
}

impl GuiLoggerFmi2 {
    pub fn new(tx: Sender<LogEntry>) -> Self {
        Self { tx }
    }
}

impl fmi_rs::fmi2::log::Logger for GuiLoggerFmi2 {
    fn log_call(&self, _status: fmi2Status, message: &str) {
        let _ = self.tx.send(LogEntry {
            at: Instant::now(),
            level: LogLevel::Debug,
            category: "FMI".to_owned(),
            message: message.to_owned(),
        });
    }

    fn log_message(&self, status: fmi2Status, category: &str, message: &str) {
        let level = match status {
            fmi2Status::fmi2OK | fmi2Status::fmi2Pending => LogLevel::Info,
            fmi2Status::fmi2Warning => LogLevel::Warning,
            fmi2Status::fmi2Discard | fmi2Status::fmi2Error => LogLevel::Error,
            fmi2Status::fmi2Fatal => LogLevel::Fatal,
        };
        let _ = self.tx.send(LogEntry {
            at: Instant::now(),
            level,
            category: category.to_owned(),
            message: message.trim_end().to_owned(),
        });
    }
}
