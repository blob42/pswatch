use core::fmt::Debug;
use std::os::unix::ffi::OsStrExt;
use std::{fmt::Display, time::Duration};

use crate::matching::{MatchBy, PatternIn};
use crate::state::{ConditionMatcher, StateTracker};
use log::debug;
use memchr;
use serde::Deserialize;
use sysinfo;

#[cfg(test)]
use mock_instant::thread_local::Instant;

#[cfg(not(test))]
use std::time::Instant;

#[derive(Debug, Clone, PartialEq)]
pub enum ProcState {
    NeverSeen,
    Seen,
    NotSeen,
}

#[derive(Debug, Clone)]
pub struct ProcLifetime {
    first_seen: Option<Instant>,
    last_seen: Option<Instant>,
    last_refresh: Option<Instant>,
    prev_refresh: Option<Instant>,
    prev_state: Option<ProcState>,
    state: ProcState,
}

impl ProcLifetime {
    pub fn new() -> ProcLifetime {
        Self {
            first_seen: None,
            last_seen: None,
            last_refresh: None,
            prev_refresh: None,
            prev_state: None,
            state: ProcState::NeverSeen,
        }
    }

    // /// the state is entering or exiting a Seen/NotSeen state
    // pub fn is_switching(&self) -> bool {
    //     (matches!(self.state, ProcState::Seen)
    //         && matches!(self.prev_state, Some(ProcState::NotSeen))) ||
    //     (matches!(self.state, ProcState::NotSeen)
    //         && matches!(self.prev_state, Some(ProcState::Seen)))
    // }
}

impl Display for ProcState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = match self {
            ProcState::NeverSeen => "never_seen",
            ProcState::Seen => "seen",
            ProcState::NotSeen => "not_seen",
        };
        write!(f, "{state}")
    }
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
    pattern: PatternIn<String>,
    lifetime: ProcLifetime,
    pids: Vec<usize>,
}

impl Process {
    pub fn build(pat: PatternIn<String>, state_matcher: ProcLifetime) -> Self {
        Self {
            pattern: pat,
            lifetime: state_matcher,
            pids: vec![],
        }
    }

    pub fn from_pattern(pat: PatternIn<String>) -> Self {
        Self {
            pattern: pat,
            lifetime: ProcLifetime::new(),
            pids: vec![],
        }
    }

    fn update_inner_state(&mut self) {
        if self.pids.is_empty() {
            // no change if process still never seen
            if !matches!(self.state(), ProcState::NeverSeen) {
                self.lifetime.prev_state = Some(self.lifetime.state.clone());
                self.lifetime.state = ProcState::NotSeen;
                debug!("<{}>: process disappread", self.pattern);
            } else {
                self.lifetime.prev_state = Some(ProcState::NeverSeen);
                debug!("<{}>: never seen so far", self.pattern);
            }
            // process found
        } else {
            match self.state() {
                ProcState::NeverSeen => {
                    self.lifetime.first_seen = self.lifetime.last_refresh;
                    debug!("<{}>: process seen first time", self.pattern);
                }
                ProcState::NotSeen => {
                    debug!("<{}>: process reappeared", self.pattern);
                }
                ProcState::Seen => {
                    debug!("<{}>: process still running", self.pattern);
                }
            }
            self.lifetime.prev_state = Some(self.lifetime.state.clone());
            self.lifetime.state = ProcState::Seen;
            self.lifetime.last_seen = self.lifetime.last_refresh;
        }
    }

    // /// matches processes on the full path to the executable
    // fn matches_exe(&self, info: &sysinfo::System) -> bool {
    //     info.processes()
    //         .values()
    //         .filter_map(|proc| {
    //             let finder = memchr::memmem::Finder::new(&self.pattern);
    //             proc.exe()
    //                 .and_then(|exe_name| finder.find(exe_name.as_os_str().as_bytes()))
    //         })
    //         .next()
    //         .is_some()
    // }

    // /// matches processes on the full command line
    // fn matches_cmdline(&self, info: &sysinfo::System) -> bool {
    //     info.processes()
    //         .values()
    //         .filter_map(|proc| {
    //             let finder = memchr::memmem::Finder::new(&self.pattern);
    //             finder.find(proc.cmd().join(" ").as_bytes())
    //         })
    //         .next()
    //         .is_some()
    // }

    // /// matches processes the command name only
    // fn matches_name(&self, info: &sysinfo::System) -> bool {
    //     info.processes_by_name(&self.pattern).next().is_some()
    // }
}

// pub trait ProcessMatcher<MatchBy> {
//     fn matches_process(&self, info: &sysinfo::System, match_by: MatchBy) -> bool;
// }

// FIX: remove
// impl ProcessMatcher<PatternIn> for Process {
//     fn matches_process(&self, info: &sysinfo::System, match_by: PatternIn) -> bool {
//         match match_by {
//             PatternIn::ExePath => self.matches_exe(info),
//             PatternIn::Cmdline => self.matches_cmdline(info),
//             PatternIn::Name => self.matches_name(info),
//         }
//     }
// }


impl StateTracker for Process
{
    type State = ProcState;

    /// updates the state and return a copy of the new state
    fn update_state(&mut self, info: &sysinfo::System, t_refresh: Instant) -> ProcState {
        self.pids = info
            .processes()
            .into_iter()
            .filter(|(_, proc)| MatchBy::match_by(*proc, self.pattern.clone()))
            .map(|(_, proc)| proc.pid().into())
            .collect();

        // detect running process
        // self.pids = info
        //     //FIX:
        //     .processes_by_name(self.pattern)
        //     .map(|p| p.pid().into())
        //     .collect::<Vec<usize>>();
        debug!("<{}> detected pids: {}", self.pattern, self.pids.len());

        self.lifetime.prev_refresh = self.lifetime.last_refresh;
        self.lifetime.last_refresh = Some(t_refresh);

        self.update_inner_state();
        self.lifetime.state.clone()
    }

    fn state(&self) -> Self::State {
        self.lifetime.state.clone()
    }

    fn prev_state(&self) -> Option<Self::State> {
        self.lifetime.prev_state.clone()
    }
}

impl ConditionMatcher for Process {
    type Condition = ProcCondition;

    fn matches(&self, c: Self::Condition) -> bool {
        self.lifetime.matches(c)
    }
}

impl ConditionMatcher for ProcLifetime {
    type Condition = ProcCondition;

    fn matches(&self, cond: Self::Condition) -> bool {
        match cond {
            ProcCondition::Seen(_) => {
                if !matches!(self.state, ProcState::Seen) {
                    return false;
                };
                if let Some(first_seen) = self.first_seen {
                    first_seen.elapsed() > cond.span()
                } else {
                    false
                }
            }
            ProcCondition::NotSeen(span) => {
                if !matches!(self.state, ProcState::NotSeen | ProcState::NeverSeen) {
                    false
                } else if let Some(last_seen) = self.last_seen {
                    last_seen.elapsed() > cond.span()
                } else {
                    matches!(self.state, ProcState::NeverSeen)
                        && self.prev_state.is_some()
                        && self.state == self.prev_state.clone().unwrap()
                        && self.prev_refresh.is_some()
                        && self.prev_refresh.unwrap().elapsed() > span
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(unused_imports)]
mod test {
    use super::*;
    use crate::{sched::Scheduler, state::*};
    use mock_instant::thread_local::MockClock;
    use sysinfo::System;

    #[test]
    fn default_process() {
        let p = Process::from_pattern(PatternIn::Name("foo".into()));
        assert!(matches!(p.state(), ProcState::NeverSeen))
    }

    // default process matching on process name
    #[test]
    fn match_process_name() -> anyhow::Result<(), std::io::Error> {
        let pattern = "53829";
        let mut target = std::process::Command::new("tests/5382952proc.sh")
            .arg("300")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_secs(1));

        let mut p_match = Process::from_pattern(PatternIn::Name(pattern.into()));
        let mut p_does_not_match = Process::from_pattern(PatternIn::Name("foobar_234324".into()));
        let mut sys = System::new();
        sys.refresh_specifics(Scheduler::process_refresh_specs());

        p_match.update_state(&sys, Instant::now());
        // dbg!(sys.processes().get(&p_match.pids[0].into()));
        assert!(!p_match.pids.is_empty());

        p_does_not_match.update_state(&sys, Instant::now());
        assert!(p_does_not_match.pids.is_empty());

        target.kill()
    }

    #[test]
    fn match_pattern_exe() -> anyhow::Result<(), std::io::Error> {
        let pattern = "/bin";
        let mut target = std::process::Command::new("tests/5382952proc.sh")
            .arg("300")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_secs(1));

        let mut p_match = Process::from_pattern(PatternIn::ExePath(pattern.into()));
        let mut sys = System::new();
        sys.refresh_specifics(Scheduler::process_refresh_specs());
        p_match.update_state(&sys, Instant::now());
        assert!(!p_match.pids.is_empty());
        target.kill()
    }

    #[test]
    fn match_pattern_cmdline() -> anyhow::Result<(), std::io::Error> {
        let pattern = "300";
        let mut target = std::process::Command::new("tests/5382952proc.sh")
            .arg("300")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_secs(1));

        let mut p_match = Process::from_pattern(PatternIn::Cmdline(pattern.into()));
        let mut sys = System::new();
        sys.refresh_specifics(Scheduler::process_refresh_specs());
        p_match.update_state(&sys, Instant::now());
        assert!(!p_match.pids.is_empty());
        target.kill()
    }

    #[test]
    fn cond_seen_since() {
        MockClock::set_time(Duration::ZERO);
        let cond_seen = ProcCondition::Seen(Duration::from_secs(5));
        let mut p = Process::from_pattern(PatternIn::Name("foo".into()));
        p.lifetime.last_refresh = Some(Instant::now());

        // no process detected initially
        // let mut action = proc_state.update(cond_seen.clone(), !detected, last_refresh);
        assert!(matches!(p.state(), ProcState::NeverSeen));
        assert!(!p.lifetime.matches(cond_seen.clone()));

        MockClock::advance(Duration::from_secs(2));

        // process detected
        p.pids = vec![1];
        p.lifetime.last_refresh = Some(Instant::now());
        let first_seen = p.lifetime.last_refresh;
        p.update_inner_state();
        assert!(
            matches!(p.lifetime.state, ProcState::Seen),
            "should be detected"
        );
        assert!(
            !p.lifetime.matches(cond_seen.clone()),
            "should match user condition"
        );

        // process exceeded condition
        MockClock::advance(Duration::from_secs(6));
        p.lifetime.last_refresh = Some(Instant::now());
        let last_seen = p.lifetime.last_refresh.unwrap();
        p.update_inner_state();
        assert!(
            p.lifetime.matches(cond_seen.clone()),
            "should match user condition"
        );

        // process disappread
        MockClock::advance(Duration::from_secs(2));
        p.pids = vec![];
        p.lifetime.last_refresh = Some(Instant::now());
        p.update_inner_state();
        assert!(
            matches!(p.lifetime.state, ProcState::NotSeen),
            "should be not seen"
        );
        assert!(p.lifetime.last_seen.unwrap().elapsed() == last_seen.elapsed());
        assert!(
            !p.lifetime.matches(cond_seen.clone()),
            "should not match user condition"
        );

        // process still not seen
        MockClock::advance(Duration::from_secs(5));
        p.lifetime.last_refresh = Some(Instant::now());
        p.update_inner_state();
        // 5+2 = 7
        assert!(p.lifetime.last_seen.unwrap().elapsed() == Duration::from_secs(7));
        assert!(!p.lifetime.matches(cond_seen.clone()));
        assert!(p.lifetime.first_seen.unwrap() == first_seen.unwrap());
    }

    #[test]
    fn test_not_seen_since() {
        MockClock::set_time(Duration::ZERO);
        let cond_not_seen = ProcCondition::NotSeen(Duration::from_secs(5));
        let mut p = Process::from_pattern(PatternIn::Name("foo".into()));
        p.lifetime.last_refresh = Some(Instant::now());
        let t1 = p.lifetime.last_refresh;
        p.update_inner_state();
        assert!(matches!(p.lifetime.state, ProcState::NeverSeen));
        assert!(p.lifetime.last_refresh.is_some());

        // // Case 1: The process is never seen and the condition timeout is exceeded, triggering Run.
        MockClock::advance(Duration::from_secs(7));
        p.lifetime.last_refresh = Some(Instant::now());
        p.lifetime.prev_refresh = t1;
        p.update_inner_state();
        assert!(p.pids.is_empty(), "no pid should be detected");
        assert!(p.lifetime.matches(cond_not_seen.clone()));
        assert!(matches!(p.lifetime.state, ProcState::NeverSeen));

        MockClock::advance(Duration::from_secs(5));

        // Case 2: A process is already running then disappears, the Run is triggered after the timeout.
        p.pids = vec![1];
        p.lifetime.last_refresh = Some(Instant::now());
        p.update_inner_state();

        assert!(
            matches!(p.lifetime.state, ProcState::Seen),
            "process found but state is {} ",
            p.state()
        );
        assert_eq!(p.lifetime.last_seen, p.lifetime.last_refresh);
        assert!(!p.lifetime.matches(cond_not_seen.clone()));

        // process disappears
        MockClock::advance(Duration::from_secs(1));
        p.pids = vec![];
        p.lifetime.last_refresh = Some(Instant::now());
        p.update_inner_state();
        assert!(matches!(p.lifetime.state, ProcState::NotSeen));

        // Process now exceeded the absent limit and matches user cond
        MockClock::advance(Duration::from_secs(6));
        p.lifetime.last_refresh = Some(Instant::now());
        p.update_inner_state();
        assert!(matches!(p.lifetime.state, ProcState::NotSeen));
        assert!(p.lifetime.matches(cond_not_seen.clone()));
    }
}
