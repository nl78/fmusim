use std::time::Instant;

/// Progress updates streamed from the simulation worker to the UI.
pub enum StatusMsg {
    Started {
        stop_time: f64,
        started_at: Instant,
    },
    Finished(Result<(), String>),
}

/// One recorded sample, projected from the FMU's native types to `f64`
/// so it can be plotted directly. Non-numeric variables are forwarded as `None`.
pub struct SampleMsg {
    pub time: f64,
    /// One entry per selected output variable, same order as
    /// `AppConfig::selected_outputs`. `None` for non-plottable types.
    pub values: Vec<Option<f64>>,
}
