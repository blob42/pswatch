use sysinfo::Process;

pub fn debug_process(p: &Process) {
    eprintln!("{:#?}", p);
    eprintln!("{:?}", p.name());
    eprintln!("{:?}", p.exe());
    eprintln!("{:?}", p.cwd());
}
