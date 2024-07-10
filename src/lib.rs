#![allow(dead_code)]
#![allow(unused_variables)]

pub mod process;
pub mod sched;
pub mod config;

pub mod matching {

    pub trait Matcher {
        type Condition;

        fn matches(&self, c: Self::Condition) -> bool;
    }

}

