use std::{thread, time::Duration};

use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind};

fn monitor() {
    let process_refresh = RefreshKind::new().with_processes(
        ProcessRefreshKind::everything()
            .without_environ()
            .without_disk_usage(),
    );
    let mut sys = System::new();

    loop {
        sys.refresh_specifics(process_refresh);
        sys.processes().iter().take(1).for_each(|(pid, proc)| {
            println!("{} -> {:?}", pid, proc);
        });
        thread::sleep(Duration::from_secs(5))
    }
}

fn main() {

        let process_refresh_kind = ProcessRefreshKind::new()
            .with_cmd(UpdateKind::Always)
            .with_cwd(UpdateKind::Always)
            .with_exe(UpdateKind::Always);

        let process_refresh = RefreshKind::new().with_processes(process_refresh_kind);
        let mut s = System::new();
        s.refresh_specifics(process_refresh);
        println!("{:#?}", s.processes().iter().take(10).collect::<Vec<_>>());

}
