use std::{process::Command, sync::OnceLock, thread::sleep, time::Duration};

use log::{debug, error, trace};

#[cfg(test)]
use mock_instant::thread_local::Instant;

#[cfg(not(test))]
use std::time::Instant;

use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind};

use crate::config::Profile;
use crate::process::{ProcLifetime, ProcState};
use crate::state::{ConditionMatcher, StateTracker};

use super::process::Process;

/// A job that can run in the scheduler
trait Job {
    fn update(&mut self, sysinfo: &System, last_refresh: Instant);
}

pub(crate) struct ProfileJob<T>
where
    T: StateTracker + ConditionMatcher,
{
    profile: Profile,

    /// target object being profiled
    object: T,
}

impl ProfileJob<Process> {
    pub fn from_profile(profile: Profile) -> Self {
        // let pattern = profile.pattern.clone();

        Self {
            profile: profile.clone(),
            // object: Process::build(pattern, ProcLifetime::new()),
            object: Process::build(profile.matching.pattern, ProcLifetime::new()),
        }
    }
}

impl Job for ProfileJob<Process> {
    fn update(&mut self, sysinfo: &System, last_refresh: Instant) {
        // if we are entering or exiting the seen/not_seen state
        {
            let _ = self.object.update_state(sysinfo, last_refresh);
            if (matches!(self.object.state(), ProcState::Seen)
                && matches!(self.object.prev_state(), Some(ProcState::NotSeen)))
                || (matches!(self.object.state(), ProcState::NotSeen)
                    && matches!(self.object.prev_state(), Some(ProcState::Seen)))
            {
                dbg!("run exec_end !");
                //TEST: run exec_end
            }
        }

        // only process enabled commands
        for cmd in self.profile.commands.iter_mut().filter(|c| !c.disabled) {
            if self.object.matches(cmd.condition.clone()) {
                //REFACT: function
                let out = Command::new(&cmd.exec[0]).args(&cmd.exec[1..]).output();

                match out {
                    Ok(output) => {
                        if !output.status.success() {
                            eprint!(
                                "cmd error: {}",
                                String::from_utf8_lossy(output.stderr.as_slice())
                            );
                            debug!("disabling watch for <{:?}>", self.profile.matching);
                            cmd.disabled = true
                        }
                    }
                    Err(e) => {
                        error!("{:?}: failed to run cmd for: {}", self.profile.matching, e);
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

pub struct Scheduler {
    system_info: System,
    jobs: Vec<Box<dyn Job>>,
}

static PROCESS_REFRESH_SPECS: OnceLock<RefreshKind> = OnceLock::new();

impl Scheduler {
    const SAMPLING_RATE: Duration = Duration::from_secs(3);

    pub fn process_refresh_specs() -> RefreshKind {
        *PROCESS_REFRESH_SPECS.get_or_init(|| {
            let process_refresh_kind = ProcessRefreshKind::new()
                .with_cmd(UpdateKind::Always)
                .with_cwd(UpdateKind::Always)
                .with_exe(UpdateKind::Always);

            RefreshKind::new().with_processes(process_refresh_kind)
        })
    }

    pub fn new() -> Self {
        debug!("Using sampling rate of {:?}.", Self::SAMPLING_RATE);

        Self {
            system_info: System::new(),
            jobs: Vec::new(),
        }
    }

    // NOTE: when other types of (matcher, tracker) will be available for other resources:
    // Define type of profile in an enum and call the concrete version of the generic implmentation
    pub fn from_profiles(profiles: Vec<Profile>) -> Self {
        let mut jobs: Vec<Box<dyn Job>> = Vec::with_capacity(profiles.len());
        profiles
            .into_iter()
            .map(ProfileJob::from_profile)
            .for_each(|pj| jobs.push(Box::new(pj)));

        Self {
            system_info: System::new(),
            jobs,
        }
    }

    fn refresh_proc_info(&mut self) {
        self.system_info
            .refresh_specifics(Self::process_refresh_specs());
    }

    pub fn run(&mut self) {
        loop {
            self.refresh_proc_info();

            self.jobs
                .iter_mut()
                .for_each(|job| job.update(&self.system_info, Instant::now()));

            trace!("refresh sysinfo");
            sleep(Self::SAMPLING_RATE);
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}
