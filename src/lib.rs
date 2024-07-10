#![allow(dead_code)]
#![allow(unused_variables)]

pub mod process;

pub mod matching {

    pub trait Condition {}

    pub trait Matcher {
        type Condition;

        fn matches(&self, c: Self::Condition) -> bool;
    }

}

pub mod watch {
    use serde::Deserialize;
    use super::process::ProcCondition;


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
}




