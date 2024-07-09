#![allow(dead_code)]
#![allow(unused_variables)]
use std::{marker::PhantomData, time::{Duration, Instant}};
use sysinfo::System;


#[derive(Debug)]
enum ProcState {
    NeverSeen,
    Seen,
    NotSeen,
}

#[derive(Debug)]
struct Process {
    first_seen: Option<Instant>,
    last_seen: Option<Instant>,
    state: ProcState,
    prev_state: Option<ProcState>,
    pid: Option<usize>,
}


impl Process {
    fn new() -> Self {
        Self {
            first_seen: None,
            last_seen: None,
            state: ProcState::NeverSeen,
            prev_state: None,
            pid: None,
        }
    }

    fn refresh(&mut self, sysinfo: &System, last_refresh: Instant) {

    }
}

impl Matcher<Seen> for Process {
    type Condition = LifetimeCond<Seen>;

    fn matches(&self, c: Self::Condition) -> bool {
        if !matches!(self.state, ProcState::Seen) { return false };
        if let Some(first_seen) = self.first_seen {
            first_seen.elapsed() > c.span
        } else { false }
    }
}

impl Matcher<NotSeen> for Process {
    type Condition = LifetimeCond<NotSeen>;

    fn matches(&self, c: Self::Condition) -> bool {
        if !matches!(self.state, ProcState::NotSeen | ProcState::NeverSeen) { return false };
        if let Some(last_seen) = self.last_seen {
            last_seen.elapsed() > c.span
        } else { false }
    }
}

impl<T> ProcessMatcher<T> for Process where Process: Matcher<T>  {
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

struct Seen {}
impl Seen {
    fn from_duration(d: Duration) -> LifetimeCond<Self> {
        LifetimeCond{span: d, ty: PhantomData{}}
    }
}

struct NotSeen {}
impl NotSeen {
    fn from_duration(d: Duration) -> LifetimeCond<Self> {
        LifetimeCond{span: d, ty: PhantomData{}}
    }
}

trait Condition<Type> {}
impl<T> Condition<T> for LifetimeCond<T>{}

struct LifetimeCond<CondType> {
    span: Duration,
    ty: PhantomData<CondType>
}

impl<T> LifetimeCond<T> {
    fn new(span: Duration) -> Self {
        Self{ span, ty: PhantomData{} }
    }
}



trait Matcher<T> {
    type Condition: Condition<T>;

    fn matches(&self, c: Self::Condition) -> bool;
}

trait ProcessMatcher<T>: Matcher<T> {
    fn matches_exe(&self, process: &sysinfo::Process) -> bool;
    fn matches_cmdline(&self, process: &sysinfo::Process) -> bool;
}


fn main() {
    unimplemented!()
}

#[cfg(test)]
mod test {
    use sysinfo::{ProcessRefreshKind, RefreshKind, UpdateKind};

    use super::*;

    #[test]
    fn process_seen() {
        let cond = Seen::from_duration(Duration::from_secs(5));

        let process_refresh_kind = ProcessRefreshKind::new()
            .with_cmd(UpdateKind::Always)
            .with_cwd(UpdateKind::Always)
            .with_exe(UpdateKind::Always);
        let process_refresh = RefreshKind::new().with_processes(process_refresh_kind);
        let mut sys_info = System::new();
        sys_info.refresh_specifics(process_refresh);


        let mut p = Process::new();
        p.refresh(&sys_info, Instant::now());
        assert!(!Matcher::<Seen>::matches(&p, cond));
    }
}
