#![allow(dead_code)]

use std::{borrow::Borrow, time::Duration};

use super::matching::*;
use log::debug;
use serde::Deserialize;
use sysinfo::System;

#[cfg(not(test))]
use std::time::Instant;

#[derive(Debug, Clone, PartialEq)]
enum ProcState {
    NeverSeen,
    Seen,
    NotSeen,
}


impl Condition for ProcCondition {}

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
        self.last_refresh = Some(last_refresh);
        match processes.len() {
            0 => {
                // no change if process still never seen
                if !matches!(self.state, ProcState::NeverSeen) {
                    self.prev_state = Some(ProcState::NeverSeen);
                    self.state = ProcState::NotSeen;
                    self.pids = vec![];
                } else {
                    self.prev_state = Some(ProcState::NeverSeen);
                }
            }
            // process detected
            _ => {
                if self.prev_state.is_none() {
                    self.first_seen = Some(last_refresh);
                }
                self.prev_state = Some(self.state.clone());
                self.state = ProcState::Seen;
                self.last_seen = Some(last_refresh);
                self.pids = processes
                    .into_iter()
                    .map(|p| p.pid().into())
                    .collect::<Vec<usize>>();
            }
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

// impl Matcher<NotSeen> for Process {
//     type Condition = ProcLifetime<NotSeen>;
//
//     fn matches(&self, c: Self::Condition) -> bool {
//         if !matches!(self.state, ProcState::NotSeen | ProcState::NeverSeen) {
//             return false;
//         };
//         if let Some(last_seen) = self.last_seen {
//             last_seen.elapsed() > c.span
//         } else {
//             false
//         }
//     }
// }

// trait ProcessMatcher<T>: Matcher<T> {
//     fn matches_exe(&self, process: &sysinfo::Process) -> bool;
//     fn matches_cmdline(&self, process: &sysinfo::Process) -> bool;
// }
//
// impl<T> ProcessMatcher<T> for Process
// where
//     Process: Matcher<T>,
// {
//     fn matches_exe(&self, process: &sysinfo::Process) -> bool {
//         if let Some(exe) = process.exe().and_then(|c| c.to_str()) {
//             exe.contains(&self.pattern)
//         } else {
//             false
//         }
//     }
//
//     fn matches_cmdline(&self, process: &sysinfo::Process) -> bool {
//         return process.name().contains(&self.pattern);
//     }
// }



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
    fn name() {
        todo!();
    }

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
    //         "process detected but state is {} ",
    //         proc_state
    //     );
    //     assert!(proc_state.seen_since().unwrap() == last_refresh.elapsed());
    //
    //     // Advance time to ensure the process is not seen anymore.
    //     MockClock::advance(Duration::from_secs(1));
    //     last_refresh = Instant::now();
    //     proc_state.update(cond_not_seen.clone(), !detected, last_refresh);
    //     assert!(matches!(proc_state, ProcessState::NotSeen(_)));
    //
    //     // Process now exceeded the absent limit
    //     MockClock::advance(Duration::from_secs(6));
    //     last_refresh = Instant::now();
    //     let action = proc_state.update(cond_not_seen.clone(), !detected, last_refresh);
    //     assert!(matches!(action, Action::Run));
    //     }
}
