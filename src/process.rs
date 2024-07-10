#![allow(dead_code)]

use std::{borrow::Borrow, os::unix::ffi::OsStrExt, time::Duration};

use super::matching::*;
use log::debug;
use memchr;
use serde::Deserialize;
use sysinfo::System;

#[cfg(not(test))]
use std::time::Instant;

#[derive(Debug, Clone, PartialEq)]
pub enum ProcState {
    NeverSeen,
    Seen,
    NotSeen,
}

/// User defined condition on a Process
#[derive(Debug, Deserialize, Clone)]
pub enum ProcCondition {
    #[serde(rename = "seen", with = "humantime_serde")]
    Seen(Duration),

    #[serde(rename = "not_seen", with = "humantime_serde")]
    NotSeen(Duration),
    //TODO: resource management: ram, cpu, IO ...
}

impl ProcCondition {
    fn span(&self) -> Duration {
        match self {
            ProcCondition::Seen(s) => *s,
            ProcCondition::NotSeen(s) => *s,
        }
    }
}


#[derive(Debug)]
pub struct Process {
    pattern: String,
    first_seen: Option<Instant>,
    last_seen: Option<Instant>,
    last_refresh: Option<Instant>,
    state: ProcState,
    prev_state: Option<ProcState>,
    pids: Vec<usize>,
}

impl Process {
    pub fn from_pattern(pat: String) -> Self {
        Self {
            pattern: pat,
            first_seen: None,
            last_seen: None,
            last_refresh: None,
            state: ProcState::NeverSeen,
            prev_state: None,
            pids: vec![],
        }
    }

    pub fn refresh(&mut self, sysinfo: &System, last_refresh: Instant) {
        let processes: Vec<_> = sysinfo.processes_by_name(&self.pattern).collect();
        self.pids = processes
            .iter()
            .map(|p| p.pid().into())
            .collect::<Vec<usize>>();
        self.last_refresh = Some(last_refresh);
        match processes.len() {
            0 => {
                // no change if process still never seen
                if !matches!(self.state, ProcState::NeverSeen) {
                    self.prev_state = Some(self.state.clone());
                    self.state = ProcState::NotSeen;
                    debug!("<{}>: process disappread", self.pattern);
                } else {
                    self.prev_state = Some(ProcState::NeverSeen);
                    debug!("<{}>: never seen so far", self.pattern);
                }
            }

            // process found
            _ => {
                if self.prev_state.is_none() {
                    self.first_seen = Some(last_refresh);
                    debug!("<{}>: process seen first time", self.pattern);
                } else {
                    debug!("<{}>: process reappeared", self.pattern);
                }
                self.prev_state = Some(self.state.clone());
                self.state = ProcState::Seen;
                self.last_seen = Some(last_refresh);
            }
        }
    }

    /// matches processes on the full path to the executable
    fn matches_exe(&self, info: &sysinfo::System) -> bool {
        info.processes().values().filter_map(|proc| {
            let finder = memchr::memmem::Finder::new(&self.pattern);
            proc.exe().and_then(|exe_name| finder.find(exe_name.as_os_str().as_bytes()))
        }).next().is_some()
    }

    /// matches processes on the full command line
    fn matches_cmdline(&self, info: &sysinfo::System) -> bool {
        info.processes().values().filter_map(|proc| {
            let finder = memchr::memmem::Finder::new(&self.pattern);
            finder.find(proc.cmd().join(" ").as_bytes())
        }).next().is_some()
    }

    /// matches processes the command name only
    fn matches_name(&self, info: &sysinfo::System) -> bool {
        info.processes_by_name(&self.pattern).next().is_some()

    }

    fn matches_pattern(&self, info: &sysinfo::System, match_by: ProcessMatchBy) -> bool {
        match match_by {
            ProcessMatchBy::ExePath => {self.matches_exe(info) }
            ProcessMatchBy::Cmdline => {self.matches_cmdline(info)}
            ProcessMatchBy::Name => {self.matches_name(info)},
        }

    }
}

impl Matcher for Process {
    type Condition = ProcCondition;

    fn matches(&self, c: Self::Condition) -> bool {
        match c {
            ProcCondition::Seen(span) => {
                if !matches!(self.state, ProcState::Seen) {
                    return false;
                };
                if let Some(first_seen) = self.first_seen {
                    first_seen.elapsed() > c.span()
                } else {
                    false
                }
            },
            ProcCondition::NotSeen(span) => {
                if !matches!(self.state, ProcState::NotSeen | ProcState::NeverSeen) {
                    false
                } else if let Some(last_seen) = self.last_seen {
                    last_seen.elapsed() > c.span()


                } else { matches!(self.state, ProcState::NeverSeen) &&
                        self.state == self.prev_state.clone().unwrap() &&
                            self.last_refresh.is_some() &&
                            self.last_refresh.unwrap().elapsed() > span

                }
            } 
        }
    }
}


enum ProcessMatchBy {
    ExePath,
    Cmdline,
    Name
}



#[cfg(test)]
use mock_instant::global::Instant;
mod test {
    use mock_instant::global::MockClock;
    use sysinfo::{ProcessRefreshKind, RefreshKind, UpdateKind};
    use super::*;

    #[test]
    fn default_process() {
        let pat = "foo";
        let p = Process::from_pattern(pat.into());
        assert!(matches!(p.state, ProcState::NeverSeen))
    } 

    #[test]
    fn match_pattern() -> anyhow::Result<(), std::io::Error> {
        let mut p = std::process::Command::new("tests/5382952proc.sh")
            .arg("300")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_secs(1));



        p.kill()
    }

    // #[test]
    // fn cond_seen_since() {
    //     let cond_seen = ProcState::Seen(Duration::from_secs(5));
    //     let mut proc_state = ProcessState::new();
    //     let mut last_refresh = Instant::now();
    //
    //     let detected = true;
    //
    //     // no process detected
    //     let mut action = proc_state.update(cond_seen.clone(), !detected, last_refresh);
    //     assert!(matches!(action, Action::None));
    //
    //     // process detected
    //     _ = proc_state.update(cond_seen.clone(), detected, last_refresh);
    //     assert!(matches!(proc_state, ProcessState::Seen(n) if n == last_refresh));
    //
    //     // process exceeded condition
    //     MockClock::advance(Duration::from_secs(6));
    //     last_refresh = Instant::now();
    //     action = proc_state.update(cond_seen.clone(), detected, last_refresh);
    //     assert!(matches!(action, Action::Run));
    //
    //     // process disappread
    //     MockClock::advance(Duration::from_secs(2));
    //     last_refresh = Instant::now();
    //     action = proc_state.update(cond_seen.clone(), !detected, last_refresh);
    //     assert!(matches!(proc_state, ProcessState::NotSeen(_)));
    //     assert!(proc_state.not_seen_since().unwrap() == last_refresh.elapsed());
    //     assert!(matches!(action, Action::None));
    //
    //     // process not seen
    //     MockClock::advance(Duration::from_secs(5));
    //     last_refresh = Instant::now();
    //     action = proc_state.update(cond_seen.clone(), !detected, last_refresh);
    //     assert!(proc_state.not_seen_since().unwrap() == Duration::from_secs(5));
    //     assert!(matches!(action, Action::None));
    // }

    // #[test]
    // fn test_not_seen_since() {
    //     let cond_not_seen = ProcCondition::NotSeen(Duration::from_secs(5));
    //     let p = Process::from_pattern("foo".into());
    //     let mut last_refresh = Instant::now();
    //
    //     let process_refresh_kind = ProcessRefreshKind::new()
    //         .with_cmd(UpdateKind::Always)
    //         .with_cwd(UpdateKind::Always)
    //         .with_exe(UpdateKind::Always);
    //
    //     let process_refresh = RefreshKind::new().with_processes(process_refresh_kind);
    //     let mut s = System::new();
    //
    //     // used to simulate detection
    //     let pids: Vec<usize> = vec![];
    //
    //     // Case 1: The process is never seen and the condition timeout is exceeded, triggering Run.
    //     MockClock::advance(Duration::from_secs(7));
    //     s.refresh_specifics(process_refresh);
    //     p.refresh(&s, last_refresh);
    //     assert!(p.matches(cond_not_seen));
    //     // assert!(matches!(p.state, ProcState::NeverSeen { .. }));
    //
    //     // Reset for the next case
    //     let p = Process::from_pattern("foo".into());
    //     // Ensure we are at least 5 seconds into the future to test the timeout correctly.
    //     MockClock::advance(Duration::from_secs(5));
    //     p.refresh(&s, last_refresh);
    //
    //     // Case 2: A process is already running then disappears, the Run is triggered after the timeout.
    //     last_refresh = Instant::now();
    //
    //     assert!(
    //         matches!(proc_state, ProcessState::Seen(_)),
    //         "process found but state is {} ",
    //         proc_state
    //     );
    //     assert!(proc_state.seen_since().unwrap() == last_refresh.elapsed());
    //
    //     // Advance time to ensure the process is not seen anymore.
    //     MockClock::advance(Duration::from_secs(1));
    //     last_refresh = Instant::now();
    //     proc_state.update(cond_not_seen.clone(), !found, last_refresh);
    //     assert!(matches!(proc_state, ProcessState::NotSeen(_)));
    //
    //     // Process now exceeded the absent limit
    //     MockClock::advance(Duration::from_secs(6));
    //     last_refresh = Instant::now();
    //     let action = proc_state.update(cond_not_seen.clone(), !found, last_refresh);
    //     assert!(matches!(action, Action::Run));
    //     }
}
