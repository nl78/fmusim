use crossbeam_channel::Sender;

use crate::messages::SampleMsg;

/// Adapter from `fmi_rs::sim::fmi3::recorder::SampleObserver` to a channel
/// of [`SampleMsg`]. Non-numeric values (string / binary) are forwarded as
/// `None`.
pub struct GuiObserverFmi3 {
    tx: Sender<SampleMsg>,
}

impl GuiObserverFmi3 {
    pub fn new(tx: Sender<SampleMsg>) -> Self {
        Self { tx }
    }
}

impl fmi_rs::sim::fmi3::recorder::SampleObserver for GuiObserverFmi3 {
    fn on_sample(&mut self, time: f64, values: &[fmi_rs::sim::fmi3::VariableValue]) {
        use fmi_rs::sim::fmi3::VariableValue;
        let projected = values
            .iter()
            .map(|v| match v {
                VariableValue::String(_) | VariableValue::Binary(_) => None,
                other => other.as_f64().first().copied(),
            })
            .collect();
        let _ = self.tx.send(SampleMsg {
            time,
            values: projected,
        });
    }
}

/// Adapter from `fmi_rs::sim::fmi2::recorder::SampleObserver` to a channel
/// of [`SampleMsg`].
pub struct GuiObserverFmi2 {
    tx: Sender<SampleMsg>,
}

impl GuiObserverFmi2 {
    pub fn new(tx: Sender<SampleMsg>) -> Self {
        Self { tx }
    }
}

impl fmi_rs::sim::fmi2::recorder::SampleObserver for GuiObserverFmi2 {
    fn on_sample(&mut self, time: f64, values: &[fmi_rs::sim::fmi2::VariableValue]) {
        use fmi_rs::sim::fmi2::VariableValue;
        let projected = values
            .iter()
            .map(|v| match v {
                VariableValue::String(_) => None,
                other => Some(other.to_f64()),
            })
            .collect();
        let _ = self.tx.send(SampleMsg {
            time,
            values: projected,
        });
    }
}
