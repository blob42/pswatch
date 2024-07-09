//! # Scheduler Module
//!
//! This module contains the `Scheduler` struct and its associated methods for managing and scheduling processes based on certain conditions. The scheduler refreshes process information periodically, checks for matching processes in the system, and updates their state accordingly. It uses a global sampling rate to determine how often it should refresh process information.
//!
//! ## Usage
//! To use this module, create a new `Scheduler` instance by providing a list of `ProcessWatchConfig`. The scheduler will then continuously watch for processes that match the configured conditions and execute specified commands based on their state changes.
//!
//! ### Example
//! ```rust
//! let watches = vec![
//!     ProcessWatchConfig {
//!         condition: ProcCondition::Exists("my_process"),
//!         exec: vec!["command".to_string(), "arg1".to_string()],
//!         run_once: false,
//!     },
//! ];
//! let mut scheduler = Scheduler::new(&watches);
//! scheduler.watch(); // Start watching for processes and executing commands based on conditions
//! ```
//! # Structs
//! - `CmdSchedule`: Represents a scheduled command with its condition and execution details.
//! - `Scheduler`: Manages process watches, refreshes system information, and updates the state of each watch.
//!
//! # Constants
//! - `SAMPLING_RATE`: The global sampling rate in seconds used for scheduling tasks.
//!
//! # Types
//! - `SysProcess`: A type alias for a hashmap containing process information.
//!
//! # Modules
//! - `process`: Contains the definition and implementation of `ProcessWatch`, `ProcCondition`, and `ProcessWatchConfig`.
use std::{collections::HashMap, thread::sleep, time::Duration};

#[cfg(not(test))]
use std::time::Instant;

use log::trace;
#[cfg(test)]
use mock_instant::global::Instant;

use serde::{Deserialize, Deserializer};
use std::process::Command;
use sysinfo::{Pid, Process, ProcessRefreshKind, RefreshKind, System, UpdateKind};

use crate::{config::Config, utils::debug_process};

pub(crate) use process::{ProcState, ProcessWatch, ProcessWatchConfig};

mod process;

/// global sampling rate in seconds
pub const SAMPLING_RATE: Duration = Duration::from_secs(5);

/// CmdSchedule is the base configuration unit, it can be defined one or many times.
/// It consists of a single condition coupled with one or more actions (exec commands for now)
#[derive(Debug, Deserialize, Clone)]
pub struct CmdSchedule {
    /// The condition under which the command should be executed.
    condition: ProcState,

    /// The list of commands to execute. Currently marked as TODO; consider replacing with an Action enum for better type control.
    exec: Vec<String>,

    /// When `exec_end` is defined, the command schedule behaves like a toggle, indicating when the execution should stop.
    exec_end: Option<Vec<String>>,

    /// Default to false; indicates whether the commands should be executed only once.
    #[serde(default)]
    run_once: bool,

    /// Not serialized or deserialized by `serde`; indicates if the command schedule is disabled.
    #[serde(skip)]
    disabled: bool,
}

type SysProcess = HashMap<Pid, Process>;

pub struct Scheduler {
    refresh_type: RefreshKind,
    system_info: System,
    watches: Vec<ProcessWatch>,
}

impl Scheduler {
    const SAMPLING_RATE: Duration = Duration::from_secs(3);

    pub fn new(watches: &[ProcessWatchConfig]) -> Self {
        let process_refresh_kind = ProcessRefreshKind::new()
            .with_cmd(UpdateKind::Always)
            .with_cwd(UpdateKind::Always)
            .with_exe(UpdateKind::Always);

        let process_refresh = RefreshKind::new().with_processes(process_refresh_kind);

        Self {
            refresh_type: process_refresh,
            system_info: System::new(),
            watches: watches
                .iter()
                .map(|p| ProcessWatch::new(p.clone()))
                .collect(),
        }
    }

    fn refresh_proc_info(&mut self) {
        self.system_info.refresh_specifics(self.refresh_type);
    }

    pub fn watch(&mut self) {
        loop {
            self.refresh_proc_info();

            // iterate over all watched processes and find matching ones in system info
            //
            // Process detections cases:
            // - seen pattern + process exists
            // - not seen pattern + process exists
            // - seen pattern + no process
            // - not seen pattern + no process

            self.watches
                .iter_mut()
                .for_each(|w| w.update_state(&self.system_info, Instant::now()));

            trace!("refresh");
            sleep(Self::SAMPLING_RATE);
        }
    }
}
