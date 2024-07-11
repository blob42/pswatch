use serde::Deserialize;
use std::{process::Command, sync::OnceLock, thread::sleep, time::Duration};

#[cfg(not(test))]
use std::time::Instant;

use log::{debug, error, trace};
#[cfg(test)]
use mock_instant::global::Instant;

use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind};

use super::process::{ProcCondition, Process};

/// CmdSchedule is the base configuration unit, it can be defined one or many times.
/// It consists of a single condition coupled with one or more actions (exec commands for now)
#[derive(Debug, Deserialize, Clone)]
pub struct CmdSchedule {
    /// The condition under which the command should be executed.
    condition: ProcCondition,

    /// The list of commands to execute. Currently marked as TODO; consider replacing with an Action enum for better type control.
    exec: Vec<String>,

    /// When `exec_end` is defined, the command schedule behaves like a toggle, indicating when the execution should stop.
    exec_end: Option<Vec<String>>,

    /// Default to false; indicates whether the commands should be executed only once.
    #[serde(default)]
    run_once: bool,

    /// Not serialized or deserialized by `serde`; indicates if the command schedule is disabled.
    #[serde(skip)]
    disabled: bool,
}

pub struct Scheduler {
    system_info: System,
    //FIX:
    jobs: Vec<ProfileJob>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Profile {
    /// pattern of process name to match against
    pub pattern: String,

    /// List of commands to run when condition is met
    pub commands: Vec<CmdSchedule>,

    #[serde(default)]
    /// Interpret `pattern` as regex
    pub regex: bool,

    //TODO:
    // pub match_by:
    /// process watch sampling rate
    #[serde(default = "default_watch_interval", with = "humantime_serde")]
    pub interval: Duration,

    #[serde(default)]
    pub keep_watch: bool,
}

/// default process watch interval
fn default_watch_interval() -> Duration {
    Duration::from_secs(5)
}

pub(crate) struct ProfileJob {
    profile: Profile,
    process: Process,
}

impl ProfileJob {
    pub fn new(profile: Profile) -> Self {
        let pattern = profile.pattern.clone();
        Self {
            profile,
            process: Process::from_pattern(pattern),
        }
    }

    pub(crate) fn update_state(&mut self, sysinfo: &System, last_refresh: Instant) {
        // let detected = sysinfo.processes_by_name(&self.conf.pattern).count() > 0;
        // let detected = match sysinfo.processes_by_name(&self.conf.pattern).count() {
        //     0 => Event::NotDetected,
        //     _ => Event::Detected(last_refresh),
        // };

        self.process.refresh(sysinfo, last_refresh);

        let enabled_cmds: Vec<_> = self
            .profile
            .commands
            .iter_mut()
            .filter(|c| !c.disabled)
            .collect();
        // dbg!(&enabled_cmds);

        for cmd in enabled_cmds {
            // let action = self
            //     .state
            //     .update(cmd.condition.clone(), detected, last_refresh);

            if self.process.matches(cmd.condition.clone()) {
                let out = Command::new(&cmd.exec[0]).args(&cmd.exec[1..]).output();

                match out {
                    Ok(output) => {
                        if !output.status.success() {
                            eprint!(
                                "cmd error: {}",
                                String::from_utf8_lossy(output.stderr.as_slice())
                            );
                            debug!("disabling watch for <{}>", self.profile.pattern);
                            cmd.disabled = true
                        }
                    }
                    Err(e) => {
                        error!("failed to run cmd for {}", self.profile.pattern);
                        cmd.disabled = true
                    }
                }

                if cmd.run_once {
                    cmd.disabled = true
                }
            }
        }
    }
}

static PROCESS_REFRESH_SPECS: OnceLock<RefreshKind> = OnceLock::new();

impl Scheduler {
    const SAMPLING_RATE: Duration = Duration::from_secs(3);

    pub fn process_refresh_specs() -> RefreshKind {
        *PROCESS_REFRESH_SPECS.get_or_init(||{

        let process_refresh_kind = ProcessRefreshKind::new()
            .with_cmd(UpdateKind::Always)
            .with_cwd(UpdateKind::Always)
            .with_exe(UpdateKind::Always);

        RefreshKind::new().with_processes(process_refresh_kind)
        })
    }

    pub fn new(profiles: Vec<Profile>) -> Self {
        debug!("Using sampling rate of {:?}.", Self::SAMPLING_RATE);

        let jobs: Vec<ProfileJob> = profiles
            .iter()
            .map(|p| ProfileJob::new(p.clone()))
            .collect();

        Self {
            system_info: System::new(),
            jobs: profiles
            .into_iter()
            .map(ProfileJob::new)
            .collect(),
        }
    }

    fn refresh_proc_info(&mut self) {
        self.system_info.refresh_specifics(Self::process_refresh_specs());
    }

    pub fn run(&mut self) {
        loop {
            self.refresh_proc_info();

            // iterate over all watched processes and find matching ones in system info
            //
            // Process detections cases:
            // - seen pattern + process exists
            // - not seen pattern + process exists
            // - seen pattern + no process
            // - not seen pattern + no process

            self.jobs
                .iter_mut()
                .for_each(|j| j.update_state(&self.system_info, Instant::now()));

            trace!("refresh sysinfo");
            sleep(Self::SAMPLING_RATE);
        }
    }
}
