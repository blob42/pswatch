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

impl Matcher<ConditionSeen> for Process {
    type Condition = LifetimeCond<ConditionSeen>;

    fn matches(&self, c: Self::Condition) -> bool {
        if let Some(last_seen) = self.last_seen {
            last_seen.elapsed() > c.span
        } else { false }
    }
}


struct ConditionSeen {}
impl ConditionSeen {
    fn from_duration(d: Duration) -> LifetimeCond<Self> {
        LifetimeCond{span: d, ty: PhantomData{}}
    }
}

struct ConditionNotSeen {}
impl ConditionNotSeen {
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



trait Matcher<CondType> {
    type Condition: Condition<CondType>;

    fn matches(&self, c: Self::Condition) -> bool;
}


fn main() {
    unimplemented!()
}

#[cfg(test)]
mod test {
    use sysinfo::{ProcessRefreshKind, RefreshKind, UpdateKind};

    use super::*;

    #[test]
    fn process_watch_condition() {
        let cond = ConditionSeen::from_duration(Duration::from_secs(5));

        let process_refresh_kind = ProcessRefreshKind::new()
            .with_cmd(UpdateKind::Always)
            .with_cwd(UpdateKind::Always)
            .with_exe(UpdateKind::Always);
        let process_refresh = RefreshKind::new().with_processes(process_refresh_kind);
        let mut sys_info = System::new();
        sys_info.refresh_specifics(process_refresh);


        let mut p = Process::new();
        p.refresh(&sys_info, Instant::now());
        assert!(!p.matches(cond));
    }
}
