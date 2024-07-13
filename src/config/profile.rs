use std::time::Duration;
use serde::Deserialize;
use crate::process::{PatternIn, ProcCondition};

#[derive(Debug, Deserialize, Clone)]
pub struct Profile {
    /// pattern of process name to match against
    pub pattern: String,

    #[serde(default)]
    pub pattern_in: PatternIn,

    /// List of commands to run when condition is met
    pub commands: Vec<CmdSchedule>,

    #[serde(default)]
    /// Interpret `pattern` as regex
    pub regex: bool,

    //TODO:
    // pub match_by:
    /// process watch sampling rate
    #[serde(default = "default_watch_interval", with = "humantime_serde")]
    pub interval: Duration,

    #[serde(default)]
    pub keep_watch: bool,
}

/// default process watch interval
fn default_watch_interval() -> Duration {
    Duration::from_secs(5)
}

/// CmdSchedule is the base configuration unit, it can be defined one or many times.
/// It consists of a single condition coupled with one or more actions (exec commands for now)
#[derive(Debug, Deserialize, Clone)]
pub struct CmdSchedule {
    /// The condition under which the command should be executed.
    pub condition: ProcCondition,

    /// The list of commands to execute. Currently marked as TODO; consider replacing with an Action enum for better type control.
    pub exec: Vec<String>,

    /// When `exec_end` is defined, the command schedule behaves like a toggle, indicating when the execution should stop.
    pub exec_end: Option<Vec<String>>,

    /// Default to false; indicates whether the commands should be executed only once.
    #[serde(default)]
    pub run_once: bool,

    /// Not serialized or deserialized by `serde`; indicates if the command schedule is disabled.
    #[serde(skip)]
    pub disabled: bool,
}
