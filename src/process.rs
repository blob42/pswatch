use std::{fmt::Display, time::Duration};

use crate::matching::{MatchBy, ProcessMatcher};
use crate::state::{ConditionMatcher, StateTracker};
use log::{debug, log_enabled, trace};
use serde::Deserialize;
use sysinfo::{self, Pid, ProcessStatus};

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
    state_exit: bool,
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
            state_exit: false,
        }
    }
}

impl Default for ProcLifetime {
    fn default() -> Self {
        Self::new()
    }
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
    matcher: ProcessMatcher,
    lifetime: ProcLifetime,
    pids: Vec<usize>,
}

impl Process {
    pub fn build(matcher: ProcessMatcher, state_matcher: ProcLifetime) -> Self {
        Self {
            matcher,
            lifetime: state_matcher,
            pids: vec![],
        }
    }

    pub fn from_pattern(pat: impl Into<ProcessMatcher>) -> Self {
        Self {
            matcher: pat.into(),
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
                self.lifetime.state_exit = self.lifetime.prev_state != Some(ProcState::NotSeen);
                debug!("<{}>: process disappread", self.matcher);
            } else {
                self.lifetime.state_exit = false;
                self.lifetime.prev_state = Some(ProcState::NeverSeen);
                debug!("<{}>: never seen so far", self.matcher);
            }
            // process found
        } else {
            match self.state() {
                ProcState::NeverSeen => {
                    self.lifetime.state_exit = false;
                    self.lifetime.first_seen = self.lifetime.last_refresh;
                    debug!("<{}>: process seen first time", self.matcher);
                }
                ProcState::NotSeen => {
                    debug!("<{}>: process reappeared", self.matcher);
                    self.lifetime.state_exit = true;

                    // reset first_seen
                    self.lifetime.first_seen = self.lifetime.last_refresh;
                }
                ProcState::Seen => {
                    self.lifetime.state_exit = false;
                    debug!("<{}>: process still running", self.matcher);
                }
            }
            self.lifetime.prev_state = Some(self.lifetime.state.clone());
            self.lifetime.state = ProcState::Seen;
            self.lifetime.last_seen = self.lifetime.last_refresh;
        }
    }
}

impl StateTracker for Process {
    type State = ProcState;

    /// updates the state and return a copy of the new state
    fn update_state(&mut self, info: &sysinfo::System, t_refresh: Instant) -> ProcState {
        self.pids = info
            .processes()
            .iter()
            // .filter(|(_, proc)| MatchBy::match_by(*proc, self.matching.pattern.clone()))
            .filter(|(_, proc)| proc.match_by(self.matcher.clone()))
            // .inspect(|(pid, proc)| debug!("[{}] status: {}", pid, (proc.status()))) 

            // filter out non active and dead processes
            .filter(|(_, proc)| {
                !matches!(proc.status(), ProcessStatus::Stop | ProcessStatus::Dead | ProcessStatus::Zombie)
            })
            .map(|(_, proc)| proc.pid().into())
            .collect();

        debug!("<{}> detected pids: {}", self.matcher, self.pids.len());
        // trace matched pids

        if log_enabled!(log::Level::Trace) {
            self.pids
                .iter()
                .filter_map(|pid| info.processes().get(&Pid::from(*pid)))
                .for_each(|p| trace!("- {}: \n {:#?}", p.pid(), p));
        }

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

    fn exiting(&self) -> bool {
        self.lifetime.state_exit
    }
}

impl ConditionMatcher for Process {
    type Condition = ProcCondition;

    fn matches(&self, c: Self::Condition) -> bool {
        self.lifetime.matches(c)
    }

    fn partial_match(&self, c: Self::Condition) -> Option<bool> {
        self.lifetime.partial_match(c)
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

    fn partial_match(&self, cond: Self::Condition) -> Option<bool> {
        match cond {
            ProcCondition::Seen(_) => Some(matches!(self.state, ProcState::Seen)),
            ProcCondition::NotSeen(_) => Some(matches!(
                self.state,
                ProcState::NotSeen | ProcState::NeverSeen
            )),
        }
    }
}

//DEBUG:
#[cfg(test)]
#[allow(unused_imports)]
mod test {
    use super::*;
    use crate::{matching::PatternIn, sched::Scheduler, state::*};
    use mock_instant::thread_local::MockClock;
    use regex::Regex;
    use sysinfo::System;

    #[test]
    fn default_process() {
        let p = Process::from_pattern(PatternIn::Name("foo".to_string()));
        assert!(matches!(p.state(), ProcState::NeverSeen))
    }

    // default process matching on process name
    #[test]
    fn match_process_name() -> anyhow::Result<(), std::io::Error> {
        let pattern = "aPYe1K";
        let mut target = std::process::Command::new("tests/fake_bins/proc-50aPYe1K.sh")
            .arg("300")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_secs(1));

        let mut p_match = Process::from_pattern(PatternIn::Name(pattern.to_string()));
        let mut p_does_not_match =
            Process::from_pattern(PatternIn::Name("foobar_234324".to_string()));
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
        let pattern = "fake_bins/sleep-w61Z";
        let mut target = std::process::Command::new("tests/fake_bins/sleep-w61Z")
            .arg("300")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_secs(1));

        let mut p_match = Process::from_pattern(PatternIn::ExePath(pattern.to_string()));
        let mut sys = System::new();
        sys.refresh_specifics(Scheduler::process_refresh_specs());
        p_match.update_state(&sys, Instant::now());
        assert!(!p_match.pids.is_empty());
        target.kill()
    }

    // exe path dereferences the os symlink so exe path here is the target of the symlink
    #[test]
    fn regex_pattern_exe() -> anyhow::Result<(), std::io::Error> {
        let pattern = r"sl\w+-w\d{2}Z$"; // Example regex for files ending with .sh
        let mut target = std::process::Command::new("tests/fake_bins/sleep-583a")
            .arg("5")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_secs(1));

        let mut p_match = Process::from_pattern(PatternIn::ExePath(Regex::new(pattern).unwrap()));
        let mut sys = System::new();
        sys.refresh_specifics(Scheduler::process_refresh_specs());
        p_match.update_state(&sys, Instant::now());
        assert!(!p_match.pids.is_empty());
        target.kill() // Ensure you handle the Result from kill properly
    }

    // regex for process name
    #[test]
    fn regex_pattern_name() -> anyhow::Result<(), std::io::Error> {
        let pattern = r"sleep-\d{3}a$"; // Example regex for files ending with .sh
        let mut target = std::process::Command::new("tests/fake_bins/sleep-583a")
            .arg("5")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_secs(1));

        let mut p_match = Process::from_pattern(PatternIn::Name(Regex::new(pattern).unwrap()));
        let mut sys = System::new();
        sys.refresh_specifics(Scheduler::process_refresh_specs());
        p_match.update_state(&sys, Instant::now());
        assert!(!p_match.pids.is_empty());
        target.kill() // Ensure you handle the Result from kill properly
    }

    // regex for process cmdline
    #[test]
    fn regex_pattern_cmdline() -> anyhow::Result<(), std::io::Error> {
        let pattern = r"sleep-\d{3}a\s5"; // Example regex for files ending with .sh
        let mut target = std::process::Command::new("tests/fake_bins/sleep-583a")
            .arg("5")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_secs(1));

        let mut p_match = Process::from_pattern(PatternIn::Cmdline(Regex::new(pattern).unwrap()));
        let mut sys = System::new();
        sys.refresh_specifics(Scheduler::process_refresh_specs());
        p_match.update_state(&sys, Instant::now());
        assert!(!p_match.pids.is_empty());
        target.kill() // Ensure you handle the Result from kill properly
    }

    #[test]
    fn match_pattern_cmdline() -> anyhow::Result<(), std::io::Error> {
        let pattern = "300";
        let mut target = std::process::Command::new("tests/fake_bins/proc-Ml51n.sh")
            .arg("300")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_secs(1));

        let mut p_match = Process::from_pattern(PatternIn::Cmdline(pattern.to_string()));
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
        let mut p = Process::from_pattern(PatternIn::Name("foo".to_string()));
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
        let mut p = Process::from_pattern(PatternIn::Name("foo".to_string()));
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
