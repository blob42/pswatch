use std::time::{Duration, Instant};

use pswatch::{process::{self, ProcCondition}, sched::Scheduler};
use sysinfo::System;


#[test]
// cond: seen for 200ms
// start state: seen
// test state: seen for 400ms
//FIX:
fn match_cond_seen() {
    let cond_span = Duration::from_millis(200);
    let test_span = Duration::from_millis(400);
    let mut s = System::new();
    let mut target = std::process::Command::new("tests/5382952proc.sh")
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();

    std::thread::sleep(std::time::Duration::from_secs(1));
    s.refresh_specifics(Scheduler::process_refresh_specs());

    // process exists
    assert!(s.process((target.id() as usize).into()).is_some());


    let pat = "538295";
    let mut p = process::Process::from_pattern(pat.into());
    p.refresh(&s, Instant::now());
    dbg!(&p);

    let cond = ProcCondition::Seen(cond_span);

    std::thread::sleep(test_span);

    s.refresh_specifics(Scheduler::process_refresh_specs());
    p.refresh(&s, Instant::now());

    // process exceeded cond
    assert!(p.matches(cond), "process should be seen");
    let _ = target.kill();
}

#[test]
// cond: not seen for 400ms
// start state: never seen
// test state: never seen for 1s
fn match_cond_not_seen() {
    let cond_span = Duration::from_millis(400);
    let test_span = Duration::from_millis(100);
    let mut s = System::new();
    s.refresh_specifics(Scheduler::process_refresh_specs());
    let cond = ProcCondition::NotSeen(cond_span);



    let pat = "4hxHtngjjkXbA9XJtl9nrs/0kxqjvXnFK79Q8iUzWXo=";
    let mut p = process::Process::from_pattern(pat.into());
    s.refresh_specifics(Scheduler::process_refresh_specs());
    let t1 = Instant::now();
    p.refresh(&s, t1);


    std::thread::sleep(test_span);

    s.refresh_specifics(Scheduler::process_refresh_specs());
    p.refresh(&s, Instant::now());

    // process exceeded cond
    let d = t1.elapsed().as_millis();
    assert!(!p.matches(cond),
    "process is not absent long enough. \ncondition: not_seen({}ms) > observation: not_seen: {}ms",
    cond_span.as_millis() ,d);
}

#[test]
// cond: not seen for 400ms
// start state: seen
// test state: not seen for 200ms
fn match_cond_not_seen_2() {
    let cond_span = Duration::from_millis(400);
    let test_span = Duration::from_millis(200);
    let mut s = System::new();
    s.refresh_specifics(Scheduler::process_refresh_specs());
    let cond = ProcCondition::NotSeen(cond_span);



    let pat = "4hxHtngjjkXbA9XJtl9nrs/0kxqjvXnFK79Q8iUzWXo=";
    let mut p = process::Process::from_pattern(pat.into());
    s.refresh_specifics(Scheduler::process_refresh_specs());
    let t1 = Instant::now();
    p.refresh(&s, t1);


    std::thread::sleep(test_span);

    s.refresh_specifics(Scheduler::process_refresh_specs());
    p.refresh(&s, Instant::now());

    // process exceeded cond
    let d = t1.elapsed().as_millis();
    assert!(!p.matches(cond),
    "process is not absent long enough. \ncondition: not_seen({}ms) > observation: not_seen: {}ms",
    cond_span.as_millis() ,d);

}

