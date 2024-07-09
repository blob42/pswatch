use sysinfo::{Pid, System};
use pswatch::process::Process;

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


    // 
    let pattern = "leep";
}
