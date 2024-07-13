//! A program that allows you to monitor system processes and execute custom commands when specific
//! patterns are matched. This application is designed for managing and automating tasks based on
//! the presence or absence of certain processes in your system.

#![allow(dead_code, unused_variables, unused_imports)]
mod utils;

use std::{
    os,
    path::PathBuf,
    process::exit,
    thread::sleep,
    time::{Duration, Instant},
};

use anyhow::{bail, Context};
use clap::Parser;
use pswatch::{config, sched::Scheduler};
use sd_notify::{notify, NotifyState};
use sysinfo::{ProcessRefreshKind, RefreshKind, System};


/// Watch and run commands on matching processes
///
/// This program watches system processes for user setup patterns and runs
/// custom commands when a process match is found.
#[derive(Parser, Debug)]
#[command(author, version)]
struct Cli {
    /// path to config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Debug parameters (-d, ..., -ddd)
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}

fn main() -> anyhow::Result<()> {
    let _ = sd_notify::notify(true, &[NotifyState::Ready]);
    let cli = Cli::parse();

    let mut logger = env_logger::builder();
    logger.filter_level(log::LevelFilter::Info);
    match cli.debug {
        0 => {
            logger.filter_level(log::LevelFilter::Warn);
        }
        1 => {
            logger.filter_level(log::LevelFilter::Info);
        }
        2 => {
            logger.filter_level(log::LevelFilter::Debug);
        }
        3 => {
            logger.filter_level(log::LevelFilter::Trace);
        }
        _ => {}
    }
    logger.init();

    let program_cfg = config::read_config(cli.config).context("missing config file")?;
    dbg!(program_cfg);

    // let mut scheduler = Scheduler::from_profiles(program_cfg.profiles);
    // //TODO: own thread
    // scheduler.run();
    Ok(())
}
