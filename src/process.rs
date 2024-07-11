use std::os::unix::ffi::OsStrExt;
use std::{fmt::Display, time::Duration};

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

impl Display for ProcState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = match self {
            Self::NeverSeen => "never_seen",
            Self::Seen => "seen",
            Self::NotSeen => "not_seen",
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
    pattern: String,
    first_seen: Option<Instant>,
    last_seen: Option<Instant>,
    last_refresh: Option<Instant>,
    prev_refresh: Option<Instant>,
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
            prev_refresh: None,
            state: ProcState::NeverSeen,
            prev_state: None,
            pids: vec![],
        }
    }

    pub fn refresh(&mut self, sysinfo: &System, last_refresh: Instant) {
        self.pids = sysinfo.processes_by_name(&self.pattern)
            .map(|p| p.pid().into())
            .collect::<Vec<usize>>();
        self.prev_refresh = self.last_refresh;
        self.last_refresh = Some(last_refresh);

        self.update_state();
    }

    fn update_state(&mut self) {

        if self.pids.is_empty() {
            // no change if process still never seen
            if !matches!(self.state, ProcState::NeverSeen) {
                self.prev_state = Some(self.state.clone());
                self.state = ProcState::NotSeen;
                debug!("<{}>: process disappread", self.pattern);
            } else {
                self.prev_state = Some(ProcState::NeverSeen);
                debug!("<{}>: never seen so far", self.pattern);
            }
            // process found
        } else {
            if self.prev_state.is_none() {
                self.first_seen = self.last_refresh;
                debug!("<{}>: process seen first time", self.pattern);
            } else {
                debug!("<{}>: process reappeared", self.pattern);
            }
            self.prev_state = Some(self.state.clone());
            self.state = ProcState::Seen;
            self.last_seen = self.last_refresh;

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

    pub fn matches(&self, c: ProcCondition) -> bool {
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
                            self.prev_refresh.is_some() &&
                            self.prev_refresh.unwrap().elapsed() > span

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
    use crate::sched::Scheduler;
    use super::*;

    #[test]
    fn default_process() {
        let pat = "foo";
        let p = Process::from_pattern(pat.into());
        assert!(matches!(p.state, ProcState::NeverSeen))
    } 

    // default process pattern matching (name)
    #[test]
    fn match_pattern_default() -> anyhow::Result<(), std::io::Error> {
        let pattern = "53829";
        let mut target = std::process::Command::new("tests/5382952proc.sh")
            .arg("300")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_secs(1));

        let mut p_match = Process::from_pattern(pattern.into());
        let mut not_p_match = Process::from_pattern("foobar_234324".into());
        let mut sys = System::new();
        sys.refresh_specifics(Scheduler::process_refresh_specs());
        p_match.refresh(&sys, Instant::now());
        assert!(!p_match.pids.is_empty());

        not_p_match.refresh(&sys, Instant::now());
        assert!(not_p_match.pids.is_empty());

        target.kill()
    }

    #[test]
    fn match_pattern_exe() {
        todo!();
    }

    #[test]
    fn match_pattern_cmdline() {
        todo!();
    }
    

    #[test]
    fn cond_seen_since() {
        let cond_seen = ProcCondition::Seen(Duration::from_secs(5));
        let mut p = Process::from_pattern("foo".into());
        p.last_refresh = Some(Instant::now());


        // no process detected initially
        // let mut action = proc_state.update(cond_seen.clone(), !detected, last_refresh);
        assert!(matches!(p.state, ProcState::NeverSeen));
        assert!(!p.matches(cond_seen.clone()));

        MockClock::advance(Duration::from_secs(2));

        // process detected
        p.pids = vec![1];
        p.last_refresh = Some(Instant::now());
        let first_seen = p.last_refresh;
        p.update_state();
        assert!(matches!(p.state, ProcState::Seen), "should be detected");
        assert!(!p.matches(cond_seen.clone()), "should match user condition");

        // process exceeded condition
        MockClock::advance(Duration::from_secs(6));
        p.last_refresh = Some(Instant::now());
        let last_seen = p.last_refresh.unwrap();
        p.update_state();
        assert!(p.matches(cond_seen.clone()), "should match user condition");

        // process disappread
        MockClock::advance(Duration::from_secs(2));
        p.pids = vec![];
        p.last_refresh = Some(Instant::now());
        p.update_state();
        assert!(matches!(p.state, ProcState::NotSeen), "should be not seen");
        assert!(p.last_seen.unwrap().elapsed() == last_seen.elapsed());
        assert!(!p.matches(cond_seen.clone()), "should not match user condition");

        // process still not seen
        MockClock::advance(Duration::from_secs(5));
        p.last_refresh = Some(Instant::now());
        p.update_state();
        // 5+2 = 7
        assert!(p.last_seen.unwrap().elapsed() == Duration::from_secs(7));
        assert!(!p.matches(cond_seen.clone()));
        assert!(p.first_seen.unwrap() == first_seen.unwrap());
    }

    #[test]
    fn test_not_seen_since() {
        let cond_not_seen = ProcCondition::NotSeen(Duration::from_secs(5));
        let mut p = Process::from_pattern("foo".into());
        p.last_refresh = Some(Instant::now());
        let t1 = p.last_refresh;
        p.update_state();
        assert!(matches!(p.state, ProcState::NeverSeen));
        assert!(p.last_refresh.is_some());


        // // Case 1: The process is never seen and the condition timeout is exceeded, triggering Run.
        MockClock::advance(Duration::from_secs(7));
        p.last_refresh = Some(Instant::now());
        p.prev_refresh = t1;
        p.update_state();
        assert!(p.pids.is_empty(), "no pid should be detected");
        assert!(p.matches(cond_not_seen.clone()));
        assert!(matches!(p.state, ProcState::NeverSeen));

        MockClock::advance(Duration::from_secs(5));

        // Case 2: A process is already running then disappears, the Run is triggered after the timeout.
        p.pids = vec![1];
        p.last_refresh = Some(Instant::now());
        p.update_state();

        assert!(
            matches!(p.state, ProcState::Seen),
            "process found but state is {} ",
            p.state
        );
        assert_eq!(p.last_seen, p.last_refresh);
        assert!(!p.matches(cond_not_seen.clone()));

        // process disappears
        MockClock::advance(Duration::from_secs(1));
        p.pids = vec![];
        p.last_refresh = Some(Instant::now());
        p.update_state();
        assert!(matches!(p.state, ProcState::NotSeen));

        // Process now exceeded the absent limit and matches user cond
        MockClock::advance(Duration::from_secs(6));
        p.last_refresh = Some(Instant::now());
        p.update_state();
        assert!(matches!(p.state, ProcState::NotSeen));
        assert!(p.matches(cond_not_seen.clone()));
        }
}
