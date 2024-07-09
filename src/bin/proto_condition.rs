#![allow(dead_code)]
#![allow(unused_variables)]
use std::time::{Duration, Instant};
use sysinfo::System;

#[derive(Debug)]
enum ProcState {
    Seen,
    NotSeen,
    NeverSeen,
}

#[derive(Debug)]
struct ProcessWatch {
    first_seen: Option<Instant>,
    last_seen: Option<Instant>,
    state: ProcState,
    prev_state: Option<ProcState>,
    pid: Option<usize>,
}

impl ProcessWatch {
    fn new() -> Self {
        Self {
            first_seen: None,
            last_seen: None,
            state: ProcState::NeverSeen,
            prev_state: None,
            pid: None,
        }
    }

    fn refresh(&mut self, sysinfo: &System, last_refresh: Instant) {}
}

enum Condition {
    Seen(Duration),
    NotSeen(Duration),
}

trait CheckCondition {
    fn check_condition(&self, c: Condition) -> bool;
}

// fn main() {
//     let p = ProcessWatch::new();
//     println!("{:?}", p);
//     for cmd in watched_cmds {
//
//     }
//
// }
