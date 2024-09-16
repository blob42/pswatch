//TODO: generate unique id per test and use it as symlink name for fake bin
use std::time::{Duration, Instant};

use pswatch::{process::{self, ProcCondition}, matching::PatternIn, sched::Scheduler, state::*};
use rstest::rstest;
use sysinfo::System;
use serial_test::serial;


#[rstest]
#[case((200, 400), true)]
#[case((200, 100), false)]
#[serial]
#[test]
// cond: seen for 200ms
// start state: seen
// test state: seen for test_span
// (cond_span, test_span, should_match)
fn match_cond_seen(
    #[case] spans: (u64, u64),
    #[case] should_match: bool
) {
    let cond_span = Duration::from_millis(spans.0);
    let test_span = Duration::from_millis(spans.1);
    let mut s = System::new();
    let mut target = std::process::Command::new("tests/fake_bins/proc-OPL6J.sh")
        .arg("300")
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();

    std::thread::sleep(std::time::Duration::from_millis(500));
    s.refresh_specifics(Scheduler::process_refresh_specs());

    // process exists
    assert!(s.process((target.id() as usize).into()).is_some());


    let pat = "OPL6J";
    let mut p = process::Process::from_pattern(PatternIn::Cmdline(String::from(pat)));
    p.update_state(&s, Instant::now());

    let cond = ProcCondition::Seen(cond_span);

    std::thread::sleep(test_span);

    s.refresh_specifics(Scheduler::process_refresh_specs());
    p.update_state(&s, Instant::now());

    // process exceeded cond
    assert_eq!(p.matches(cond), should_match,
    "process should be seen");
    let _ = target.kill();
}

// cond: not seen
// start state: never seen
// test state: never seen for `test_span`
// (cond_span, test_span, should_match)
#[rstest]
#[case((400, 500), true)]
#[case((400, 300), false)]
#[serial]
#[test]
fn match_cond_not_seen(
    #[case] spans: (u64, u64),
    #[case] should_match: bool
) {
    let cond_span = Duration::from_millis(spans.0);
    let test_span = Duration::from_millis(spans.1);
    let mut s = System::new();

    std::thread::sleep(std::time::Duration::from_millis(500));
    s.refresh_specifics(Scheduler::process_refresh_specs());
    let cond = ProcCondition::NotSeen(cond_span);


    let pat = "4hxHtngjjkXbA9XJtl9nrs/0kxqjvXnFK79Q8iUzWXo=";
    let mut p = process::Process::from_pattern(PatternIn::Name(pat.to_string()));
    s.refresh_specifics(Scheduler::process_refresh_specs());
    let t1 = Instant::now();
    p.update_state(&s, t1);


    std::thread::sleep(test_span);

    s.refresh_specifics(Scheduler::process_refresh_specs());
    p.update_state(&s, Instant::now());

    // process exceeded cond
    let d = t1.elapsed().as_millis();
    assert_eq!(p.matches(cond), should_match,
    "process is not absent long enough. \ncondition: not_seen({}ms) > observation: not_seen: {}ms",
    cond_span.as_millis() ,d);
}

// cond: not seen 
// start state: seen
// test state: not seen for `test_span`
// (cond_span, test_span, should_match)
#[rstest]
// REVIEW:
#[case((400, 200), false)]
#[case((200, 400), true)]
#[serial]
#[test]
fn match_cond_not_seen_2(
    #[case] spans: (u64, u64),
    #[case] should_match: bool
) {
    use pswatch::process::ProcState;

    let cond_span = Duration::from_millis(spans.0);
    let test_span = Duration::from_millis(spans.1);
    let mut s = System::new();
    let cond = ProcCondition::NotSeen(cond_span);

    let mut target = std::process::Command::new("tests/fake_bins/proc-dZWY4.sh")
        .arg("300")
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();

    let pat = "dZWY4";

    // ensure process is seen once
    std::thread::sleep(Duration::from_millis(200));
    s.refresh_specifics(Scheduler::process_refresh_specs());
    let mut p = process::Process::from_pattern(PatternIn::Cmdline(pat.to_string()));
    let last_seen = Instant::now();
    p.update_state(&s, last_seen);
    assert!(matches!(p.state(), ProcState::Seen));

    let _ = target.kill();

    std::thread::sleep(Duration::from_millis(10));
    s.refresh_specifics(Scheduler::process_refresh_specs());
    p.update_state(&s, Instant::now());
    assert!(matches!(p.state(), ProcState::NotSeen));


    std::thread::sleep(test_span);

    s.refresh_specifics(Scheduler::process_refresh_specs());
    p.update_state(&s, Instant::now());
    // dbg!(&p);

    // process exceeded cond
    let d = last_seen.elapsed().as_millis();
    assert_eq!(p.matches(cond), should_match,
    "\nnot_seen condition should match. \ncondition: not_seen ({}ms) > observation: {}: {}ms",
    cond_span.as_millis() , p.state(), d);
}

