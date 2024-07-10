
use std::time::{Duration, Instant};

use pswatch::{matching::Matcher, process::{self, NotSeen, ProcCondition, ProcLifetime, Seen}};
use sysinfo::System;


#[test]
fn process_match_name_substring() {
    let p = std::process::Command::new("sleep")
        .arg("300")
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();

    std::thread::sleep(std::time::Duration::from_secs(1));
    let mut s = System::new();
    s.refresh_processes();
    assert!(s.process((p.id() as usize).into()).is_some());


    let pat = "leep";
    let mut p = process::Process::from_pattern(pat.into());
    p.refresh(&s, Instant::now());

    // should be detected
    let user_cond = ProcCondition::Seen(Duration::from_secs(3));
    let cond = user_cond.to_proc_lifetime();
    std::thread::sleep(std::time::Duration::from_secs(2));

    assert!(Matcher::<Seen>::matches(&p, cond));

    // assert!(p.matches(c))
}
