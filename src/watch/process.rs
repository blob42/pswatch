//! # Process State Module
//!
//! This module defines the state of processes and provides methods to manipulate and query this state.
//! The primary types are `ProcessState`, which represents the current state of a process, and `Action`, which indicates the action that should be taken based on the process's state.

use std::{
    fmt::Display,
    io::Read,
    process::{ExitCode, ExitStatus},
    time::Duration,
};

use log::{debug, error, info};
use std::fmt::Debug;

#[cfg(not(test))]
use std::time::Instant;

use anyhow::Context;
use serde::{Deserialize, Deserializer};
use std::process::Command;
use sysinfo::{Pid, Process, ProcessRefreshKind, RefreshKind, System, UpdateKind};

use crate::{config::Config, utils::debug_process};

use super::CmdSchedule;

pub(crate) struct ProcessWatch {
    conf: ProcessWatchConfig,
    state: ProcessState,
}

impl ProcessWatch {
    pub fn new(conf: ProcessWatchConfig) -> Self {
        Self {
            conf,
            state: ProcessState::new(),
        }
    }

    /// matches exe path
    fn matches_exe(&self, process: &Process) -> bool {
        if let Some(exe) = process.exe().and_then(|c| c.to_str()) {
            exe.contains(&self.conf.pattern)
        } else {
            false
        }
    }

    /// matches cmdline
    fn matches_cmdline(&self, process: &Process) -> bool {
        return process.name().contains(&self.conf.pattern);
    }

    pub(crate) fn update_state(&mut self, sysinfo: &System, last_refresh: Instant) {
        let detected = sysinfo.processes_by_name(&self.conf.pattern).count() > 0;
        // let detected = match sysinfo.processes_by_name(&self.conf.pattern).count() {
        //     0 => Event::NotDetected,
        //     _ => Event::Detected(last_refresh),
        // };

        let enabled_cmds: Vec<_> = self
            .conf
            .commands
            .iter_mut()
            .filter(|c| !c.disabled)
            .collect();
        // dbg!(&enabled_cmds);

        for cmd in enabled_cmds {
            let action = self
                .state
                .update(cmd.condition.clone(), detected, last_refresh);
            // match cmd.condition {
            //     ProcCondition::NotSeen(duration) if !detected => {}
            //     ProcCondition::NotSeen(_) => {
            //         unreachable!()
            //     }
            // 		//TODO!: refactor
            //     // ProcessCondition::NotSeen(duration) => {
            //     //     if !detected && self.state.not_seen_since().is_some() {
            //     //         if self.state.not_seen_since().unwrap() > duration {
            //     //             // println!("process disappeared since {:?}", );
            //     //         }
            //     //     }
            //     //     else if !detected && self.state.seen() {
            //     //         self.state = ProcessState::NotSeen(last_refresh)
            //     //     } else if detected  && self.state.not_seen() {
            //     //
            //     //     } else if detected && self.state.seen(){}
            //     //
            //     // }
            //     ProcCondition::Seen(duration) => {
            //         // if we detected the process for first time
            //         if detected && self.state.not_seen() {
            //             self.state = ProcessState::Seen(last_refresh);
            //
            //             // process still running since detection
            //         } else if detected && self.state.seen() {
            //             println!(
            //                 "process running since {:?}",
            //                 self.state.seen_since().unwrap()
            //             );
            //             if let Some(seen_since) = self.state.seen_since() {
            //                 if seen_since > duration {
            //                     println!("process exceeded limit ");
            //                     cmd.dirty = true;
            //                 }
            //             }
            //
            //             // process stopped
            //         } else if !detected && self.state.seen() {
            //             dbg!("process disappared");
            //             self.state.unsee(last_refresh);
            //         }
            //     }
            // }

            if let Action::Run = action {
                let out = Command::new(&cmd.exec[0]).args(&cmd.exec[1..]).output();

                match out {
                    Ok(output) => {
                        if !output.status.success() {
                            eprint!(
                                "cmd error: {}",
                                String::from_utf8_lossy(output.stderr.as_slice())
                            );
                            debug!("disabling watch for <{}>", self.conf.pattern);
                            cmd.disabled = true
                        }
                    }
                    Err(e) => {
                        error!("failed to run cmd for {}", self.conf.pattern);
                        cmd.disabled = true
                    }
                }

                if cmd.run_once {
                    cmd.disabled = true
                }
            }
        }
    }
}

/// User defined condition on a Process
#[derive(Debug, Deserialize, Clone)]
pub(crate) enum ProcCondition {
    #[serde(rename = "seen", with = "humantime_serde")]
    Seen(Duration),

    #[serde(rename = "not_seen", with = "humantime_serde")]
    NotSeen(Duration),
    //TODO: resource management: ram, cpu, IO ...
}

impl ProcCondition {
    fn duration(self) -> Duration {
        match self {
            Self::Seen(d) => d,
            Self::NotSeen(d) => d,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct ProcessWatchConfig {
    /// pattern of process name to match against
    pub pattern: String,

    /// List of commands to run when condition is met
    pub commands: Vec<CmdSchedule>,

    #[serde(default)]
    /// Interpret `pattern` as regex
    pub regex: bool,

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

#[derive(Clone, Debug)]
enum Event {
    Detected(Instant),
    NotDetected,
}

#[derive(Debug)]
enum Action {
    Run,
    None,
}

enum ProcessState {
    NeverSeen {
        t: Instant,
    },

    /// instant since first/last time process seen
    Seen(Instant),

    /// instant since last/last time process seen
    NotSeen(Instant),
}

impl Display for ProcessState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (name, t) = match self {
            Self::NeverSeen { t } => ("never_seen", t),
            ProcessState::Seen(t) => ("seen", t),
            ProcessState::NotSeen(t) => ("not_seen", t),
        };
        write!(f, "{name}{{{}s}}", t.elapsed().as_secs())
    }
}

impl ProcessState {
    fn new() -> Self {
        Self::NeverSeen { t: Instant::now() }
    }

    fn seen(&self) -> bool {
        matches!(self, Self::Seen(_))
    }

    fn not_seen(&self) -> bool {
        matches!(self, Self::NotSeen(_) | Self::NeverSeen { .. })
    }

    fn seen_since(&self) -> Option<Duration> {
        match self {
            Self::Seen(since) => Some(since.elapsed()),
            _ => None,
        }
    }

    fn not_seen_since(&self) -> Option<Duration> {
        match self {
            Self::NotSeen(inst) => Some(inst.elapsed()),
            _ => None,
        }
    }

    fn see(&mut self, inst: Instant) {
        *self = Self::Seen(inst);
    }

    fn unsee(&mut self, inst: Instant) {
        *self = Self::NotSeen(inst);
    }

    fn update(&mut self, cond: ProcCondition, detected: bool, last_refresh: Instant) -> Action {
        match self {
            ProcessState::Seen(since) => {
                match cond {
                    // COND SEEN
                    // process already detected AND still running
                    ProcCondition::Seen(duration) if detected => {
                        let elapsed = since.elapsed().as_secs();
                        debug!("process running since {elapsed}s");
                        if self.seen_since().unwrap() > duration {
                            debug!("process exceeded runtime limit {duration:?}: RUN");
                            return Action::Run;
                        }
                    }
                    // process disappread
                    _ if !detected => {
                        debug!("process disappread");
                        self.unsee(last_refresh);
                    }
                    ProcCondition::NotSeen(duration) if detected => {}
                    ProcCondition::NotSeen(duration) if !detected => {}
                    ProcCondition::Seen(_) => {
                        unreachable!()
                    }
                    ProcCondition::NotSeen(_) => {
                        unreachable!()
                    }
                }
            }
            ProcessState::NotSeen(since) | ProcessState::NeverSeen { t: since } => match cond {
                ProcCondition::NotSeen(duration) if !detected => {
                    let elapsed_since = since.elapsed();
                    debug!("process absent for {elapsed_since:?}");
                    if elapsed_since > duration {
                        debug!("process exceeded not_seen limit {duration:?}: RUN");
                        return Action::Run;
                    }
                }
                _ if detected => self.see(last_refresh),
                ProcCondition::Seen(_) => {}
                ProcCondition::NotSeen(_) => {}
            }, //
               //     if matches!(cond, ProcCondition::Seen(_))
               //         && matches!(ev, Event::Detected(_)) =>
               // {
               //     println!("process running since {since:?}");
               //
               //     if self.seen_since().unwrap() > cond.duration() {
               //         println!("process exceeded limit ");
               //     }
               //
               // },
               // ProcessState::NotSeen(_) | ProcessState::NeverSeen
        }
        Action::None
        // *self = Self::new()
    }
}

#[cfg(test)]
use mock_instant::global::Instant;
mod tests {
    use mock_instant::global::MockClock;

    use super::*;

    #[test]
    fn cond_seen_since() {
        let cond_seen = ProcCondition::Seen(Duration::from_secs(5));
        let mut proc_state = ProcessState::new();
        let mut last_refresh = Instant::now();

        let detected = true;

        // no process detected
        let mut action = proc_state.update(cond_seen.clone(), !detected, last_refresh);
        assert!(matches!(action, Action::None));

        // process detected
        _ = proc_state.update(cond_seen.clone(), detected, last_refresh);
        assert!(matches!(proc_state, ProcessState::Seen(n) if n == last_refresh));

        // process exceeded condition
        MockClock::advance(Duration::from_secs(6));
        last_refresh = Instant::now();
        action = proc_state.update(cond_seen.clone(), detected, last_refresh);
        assert!(matches!(action, Action::Run));

        // process disappread
        MockClock::advance(Duration::from_secs(2));
        last_refresh = Instant::now();
        action = proc_state.update(cond_seen.clone(), !detected, last_refresh);
        assert!(matches!(proc_state, ProcessState::NotSeen(_)));
        assert!(proc_state.not_seen_since().unwrap() == last_refresh.elapsed());
        assert!(matches!(action, Action::None));

        // process not seen
        MockClock::advance(Duration::from_secs(5));
        last_refresh = Instant::now();
        action = proc_state.update(cond_seen.clone(), !detected, last_refresh);
        assert!(proc_state.not_seen_since().unwrap() == Duration::from_secs(5));
        assert!(matches!(action, Action::None));
    }

    #[test]
    fn cond_not_seen_since() {
        let cond_not_seen = ProcCondition::NotSeen(Duration::from_secs(5));
        let mut proc_state = ProcessState::new();
        let mut last_refresh = Instant::now();
        let detected = true;

        // Case 1: The process is never seen and the condition timeout is exceeded, triggering Run.
        MockClock::advance(Duration::from_secs(7));
        let action = proc_state.update(cond_not_seen.clone(), !detected, last_refresh);
        assert!(matches!(proc_state, ProcessState::NeverSeen { .. }));
        assert!(matches!(action, Action::Run));

        // Reset for the next case
        proc_state = ProcessState::new();
        // Ensure we are at least 5 seconds into the future to test the timeout correctly.
        MockClock::advance(Duration::from_secs(5));

        // Case 2: A process is already running then disappears, the Run is triggered after the timeout.
        last_refresh = Instant::now();
        proc_state.update(cond_not_seen.clone(), detected, last_refresh);
        assert!(
            matches!(proc_state, ProcessState::Seen(_)),
            "process detected but state is {} ",
            proc_state
        );
        assert!(proc_state.seen_since().unwrap() == last_refresh.elapsed());

        // Advance time to ensure the process is not seen anymore.
        MockClock::advance(Duration::from_secs(1));
        last_refresh = Instant::now();
        proc_state.update(cond_not_seen.clone(), !detected, last_refresh);
        assert!(matches!(proc_state, ProcessState::NotSeen(_)));

        // Process now exceeded the absent limit
        MockClock::advance(Duration::from_secs(6));
        last_refresh = Instant::now();
        let action = proc_state.update(cond_not_seen.clone(), !detected, last_refresh);
        assert!(matches!(action, Action::Run));
    }
}
