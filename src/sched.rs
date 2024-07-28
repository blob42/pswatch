use std::{process::Command, sync::OnceLock, thread::sleep, time::Duration};

use log::{debug, error, trace};

#[cfg(test)]
use mock_instant::thread_local::Instant;

#[cfg(not(test))]
use std::time::Instant;

use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind};

use crate::config::{CmdSchedule, Profile, ProfileMatching};
use crate::process::ProcLifetime;
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

        Self {
            profile: profile.clone(),
            object: Process::build(profile.matching.pattern, ProcLifetime::new()),
        }
    }
}

fn run_cmd(cmd: &mut CmdSchedule, matching: ProfileMatching, exec_end: bool) {

    let out = if exec_end && cmd.exec_end.is_some() {
        dbg!("run exec_end !");
        Command::new(&cmd.exec_end.as_ref().unwrap()[0]).args(&cmd.exec_end.as_ref().unwrap()[1..]).output()
    } else if exec_end && cmd.exec_end.is_none() {
        return;
    } else {
        dbg!("running command !");
        Command::new(&cmd.exec[0]).args(&cmd.exec[1..]).output()
    };


    match out {
        Ok(output) => {

            if !output.status.success() {
                eprint!(
                    "cmd error: {}",
                    String::from_utf8_lossy(output.stderr.as_slice())
                );
                debug!("disabling watch for <{:?}>", matching);
                cmd.disabled = true
            }
        },
        Err(e) => {
            error!("{:?}: failed to run cmd for: {}", matching, e);
            cmd.disabled = true
        }
    }

    if cmd.run_once {
        cmd.disabled = true
    }
}

impl Job for ProfileJob<Process> {


    fn update(&mut self, sysinfo: &System, last_refresh: Instant) {
        let _ = self.object.update_state(sysinfo, last_refresh);

        // self.profile.commands.iter_mut()
        //     // only process enabled commands
        //     .filter(|cmd| cmd.disabled && cmd.run_once)
        //     .for_each(|cmd| {
        //         cmd.disabled = false;
        //     });

        
        dbg!(&self.object);
        // run commands when entering match state `exec`
        self.profile.commands.iter_mut()
            // only process enabled commands
            .filter(|cmd| !cmd.disabled)
            .filter(|cmd| dbg!(self.object.matches(cmd.condition.clone())))
            .for_each(|cmd| {
                debug!("running exec cmd");

                // if we should run the exec_end command
                // dbg!(&self.object);
                // let run_exec_end = (matches!(self.object.state(), ProcState::Seen)
                //  && matches!(self.object.prev_state(), Some(ProcState::NotSeen)))
                //     || (matches!(self.object.state(), ProcState::NotSeen)
                //         && matches!(self.object.prev_state(), Some(ProcState::Seen)));

                // dbg!(run_exec_end);
                run_cmd(cmd, self.profile.matching.clone(), false);
            });

        // run commands on exit of matching state `exec_end`
        if self.object.exiting() {
            self.profile.commands.iter_mut()
                // .filter(|cmd| !cmd.disabled)
                .for_each(|cmd| {
                    //FIX: current state should be opposite of condition
                    //ie cond=Seen, exec_end runs when state is NotSeen after Seen
                    run_cmd(cmd, self.profile.matching.clone(), true);
                });
        }

        // if object does not match since 2 cycles, reset the run_once state
        self.profile.commands.iter_mut()
            .filter(|cmd| cmd.disabled && cmd.run_once)
            .for_each(|cmd| {
                if !self.object.matches(cmd.condition.clone()) &&
                self.object.prev_state().is_some_and(|s| s == self.object.state()) {
                    debug!("disabling cmd");
                    cmd.disabled = false;
                }
            });

        // for cmd in self.profile.commands.iter_mut().filter(|c| !c.disabled) {
        //     if self.object.matches(cmd.condition.clone()) {
        //         let _ = run_cmd(cmd, self.profile.matching.clone()).inspect_err(|e|{
        //                error!("{}", e);
        //         });
        //         //REFACT: function
        //         // let out = Command::new(&cmd.exec[0]).args(&cmd.exec[1..]).output();
        //         //
        //         // match out {
        //         //     Ok(output) => {
        //         //         if !output.status.success() {
        //         //             eprint!(
        //         //                 "cmd error: {}",
        //         //                 String::from_utf8_lossy(output.stderr.as_slice())
        //         //             );
        //         //             debug!("disabling watch for <{:?}>", self.profile.matching);
        //         //             cmd.disabled = true
        //         //         }
        //         //     }
        //         //     Err(e) => {
        //         //         error!("{:?}: failed to run cmd for: {}", self.profile.matching, e);
        //         //         cmd.disabled = true
        //         //     }
        //         // }
        //         //
        //         // if cmd.run_once {
        //         //     cmd.disabled = true
        //         // }
        //     }
        // }
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

            #[cfg(debug_assertions)]
            let _  = Command::new("clear").spawn();
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}
