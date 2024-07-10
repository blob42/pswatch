
use std::time::{Duration, Instant};

use pswatch::{matching::Matcher, process::{self, ProcCondition}};
use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind};

// filtered sysinfo for processes 
struct ProcSysinfo {
    sys: sysinfo::System,
    ref_kind: sysinfo::RefreshKind,

}

impl ProcSysinfo {
    fn new() -> Self {
    let process_refresh_kind = ProcessRefreshKind::new()
        .with_cmd(UpdateKind::Always)
        .with_cwd(UpdateKind::Always)
        .with_exe(UpdateKind::Always);
    let process_refresh = RefreshKind::new().with_processes(process_refresh_kind);

        Self { sys: sysinfo::System::new(), ref_kind: process_refresh }
    }

    fn refresh(&mut self) {
        self.sys.refresh_specifics(self.ref_kind)
    }
}

#[test]
// cond: seen for 1s
// start state: seen
// test state: seen for 2s
fn match_substring_cond_seen() {
    let cond_span = Duration::from_millis(200);
    let test_span = Duration::from_millis(400);
    let mut s = ProcSysinfo::new();
    let p = std::process::Command::new("sleep")
        .arg("300")
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();

    std::thread::sleep(std::time::Duration::from_millis(500));
    s.refresh();
    // process exists
    assert!(s.sys.process((p.id() as usize).into()).is_some());


    let pat = "leep";
    let mut p = process::Process::from_pattern(pat.into());
    s.refresh();
    p.refresh(&s.sys, Instant::now());

    // cond: seen for 1+ sec 
    let cond = ProcCondition::Seen(cond_span);

    std::thread::sleep(test_span);

    s.refresh();
    p.refresh(&s.sys, Instant::now());

    // process exceeded cond
    assert!(p.matches(cond), "process should be seen");

    // assert!(p.matches(c))
}

#[test]
// cond: not seen for 4s
// start state: never seen
// test state: never seen for 1s
fn match_substring_cond_not_seen() {
    let cond_span = Duration::from_millis(400);
    let test_span = Duration::from_millis(100);
    let mut s = ProcSysinfo::new();
    s.refresh();
    let cond = ProcCondition::NotSeen(cond_span);



    let pat = "4hxHtngjjkXbA9XJtl9nrs/0kxqjvXnFK79Q8iUzWXo=";
    let mut p = process::Process::from_pattern(pat.into());
    s.refresh();
    let t1 = Instant::now();
    p.refresh(&s.sys, t1);
    dbg!(&p);


    std::thread::sleep(test_span);

    s.refresh();
    p.refresh(&s.sys, Instant::now());
    dbg!(&p);

    // process exceeded cond
    let d = t1.elapsed().as_millis();
    assert!(!p.matches(cond),
    "process is not absent long enough. \ncondition: not_seen({}ms) > observation: not_seen: {}ms",
    cond_span.as_millis() ,d);
    // assert!(p.matches(c))
}
