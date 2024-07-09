#![allow(dead_code)]

use std::time::{Duration, Instant};

use super::matching::*;
use serde::Deserialize;
use sysinfo::System;

#[derive(Debug, Clone)]
enum ProcState {
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

#[derive(Debug)]
pub struct Process {
    // TODO: add process matchers
    // matchers: Vec<Box<dyn Matcher>>,
    pattern: String,
    first_seen: Option<Instant>,
    last_seen: Option<Instant>,
    state: ProcState,
    prev_state: Option<ProcState>,
    pids: Vec<usize>,
}

impl Process {
    fn from_pattern(pat: String) -> Self {
        Self {
            pattern: pat,
            first_seen: None,
            last_seen: None,
            state: ProcState::NeverSeen,
            prev_state: None,
            pids: vec![],
        }
    }

    fn refresh(&mut self, sysinfo: &System, last_refresh: Instant) {
        let processes: Vec<_> = sysinfo.processes_by_name(&self.pattern).collect();
        match processes.len() {
            0 => {
                // no change if process still never seen
                if !matches!(self.state, ProcState::NeverSeen) {
                    self.prev_state = Some(ProcState::NeverSeen);
                    self.state = ProcState::NotSeen;
                    self.pids = vec![];
                }
            }
            _ => {
                if matches! (self.prev_state, Some(ProcState::NeverSeen)) {
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

impl Matcher<Seen> for Process {
    type Condition = ProcLifetime<Seen>;

    fn matches(&self, c: Self::Condition) -> bool {
        if !matches!(self.state, ProcState::Seen) {
            return false;
        };
        if let Some(first_seen) = self.first_seen {
            first_seen.elapsed() > c.span
        } else {
            false
        }
    }
}

impl Matcher<NotSeen> for Process {
    type Condition = ProcLifetime<NotSeen>;

    fn matches(&self, c: Self::Condition) -> bool {
        if !matches!(self.state, ProcState::NotSeen | ProcState::NeverSeen) {
            return false;
        };
        if let Some(last_seen) = self.last_seen {
            last_seen.elapsed() > c.span
        } else {
            false
        }
    }
}

impl<T> ProcessMatcher<T> for Process
where
    Process: Matcher<T>,
{
    fn matches_exe(&self, process: &sysinfo::Process) -> bool {
        // if let Some(exe) = process.exe().and_then(|c| c.to_str()) {
        //     exe.contains(&self.conf.pattern)
        // } else {
        //     false
        // }
        todo!()
    }

    fn matches_cmdline(&self, process: &sysinfo::Process) -> bool {
        todo!()
    }
}

trait ProcessMatcher<T>: Matcher<T> {
    fn matches_exe(&self, process: &sysinfo::Process) -> bool;
    fn matches_cmdline(&self, process: &sysinfo::Process) -> bool;
}
