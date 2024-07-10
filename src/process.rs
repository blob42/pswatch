#![allow(dead_code)]

use std::marker::PhantomData;
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


pub struct Seen {}
impl Seen {
    fn from_duration(d: Duration) -> ProcLifetime<Seen> {
        ProcLifetime {
            span: d,
            ty: PhantomData {},
        }
    }
}

pub struct NotSeen {}
impl NotSeen {
    fn from_duration(d: Duration) -> ProcLifetime<NotSeen> {
        ProcLifetime {
            span: d,
            ty: PhantomData {},
        }
    }
}

/// The lifetime of a process. 
/// Currently only handles `Seen` and `NotSeen` states.
pub struct ProcLifetime<CondType> {
    pub span: Duration,
    ty: PhantomData<CondType>,
}

impl<T> Condition for ProcLifetime<T> {}

/// User defined condition on a Process
#[derive(Debug, Deserialize, Clone)]
pub enum ProcCondition {
    #[serde(rename = "seen", with = "humantime_serde")]
    Seen(Duration),

    #[serde(rename = "not_seen", with = "humantime_serde")]
    NotSeen(Duration),
    //TODO: resource management: ram, cpu, IO ...
}

//WIP:
impl ProcCondition {
    pub fn to_proc_lifetime(&self) -> Box<dyn Condition> {
        match self {
            Self::Seen(span) => {
                Box::new(ProcLifetime::<Seen> {span: *span, ty: PhantomData{}})
            }
            Self::NotSeen(span) => {
                Box::new(ProcLifetime::<NotSeen> {span: *span, ty: PhantomData{}})
            },
        }
    }
}

#[derive(Debug)]
pub struct Process {
    pattern: String,
    first_seen: Option<Instant>,
    last_seen: Option<Instant>,
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
            state: ProcState::NeverSeen,
            prev_state: None,
            pids: vec![],
        }
    }

    pub fn refresh(&mut self, sysinfo: &System, last_refresh: Instant) {
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

trait ProcessMatcher<T>: Matcher<T> {
    fn matches_exe(&self, process: &sysinfo::Process) -> bool;
    fn matches_cmdline(&self, process: &sysinfo::Process) -> bool;
}

impl<T> ProcessMatcher<T> for Process
where
    Process: Matcher<T>,
{
    fn matches_exe(&self, process: &sysinfo::Process) -> bool {
        if let Some(exe) = process.exe().and_then(|c| c.to_str()) {
            exe.contains(&self.pattern)
        } else {
            false
        }
    }

    fn matches_cmdline(&self, process: &sysinfo::Process) -> bool {
        return process.name().contains(&self.pattern);
    }
}



#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn default_process() {
        let pat = "foo";
        let p = Process::from_pattern(pat.into());
        assert!(matches!(p.state, ProcState::NeverSeen))
    } 
}
