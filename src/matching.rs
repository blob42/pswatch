use std::{fmt::Display, os::unix::ffi::OsStrExt};

use memchr::memmem;
use serde::Deserialize;

// TODO!:
/// Match a process by a given criteria
pub trait MatchBy<Criteria>
where
    Criteria: Display,
{
    fn match_by(&self, matcher: Criteria) -> bool;
}

//TODO: handle different type of patterns (String, Regex ...)
// pub trait PatternMatcher<Pat> where Pat: String, Regex ...
pub trait PatternMatcher<P> {
    fn matches_exe(&self, pattern: P) -> bool;
    fn matches_cmdline(&self, pattern: P) -> bool;
    fn matches_name(&self, pattern: P) -> bool;
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum PatternIn<P> {
    ExePath(P),
    Cmdline(P),
    Name(P),
}

impl PatternMatcher<String> for sysinfo::Process {
    fn matches_exe(&self, pattern: String) -> bool {
        let finder = memmem::Finder::new(&pattern);
        self.exe()
            .and_then(|exe_name| finder.find(exe_name.as_os_str().as_bytes()))
            .is_some()
    }

    fn matches_cmdline(&self, pattern: String) -> bool {
        let finder = memmem::Finder::new(&pattern);
        finder.find(self.cmd().join(" ").as_bytes()).is_some()
    }

    fn matches_name(&self, pattern: String) -> bool {
        self.name().contains(&pattern)
    }
}

impl<P> Display for PatternIn<P> where P: Display {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatternIn::ExePath(p) => {
                write!(f, "exe_path[{}]", p)
            },
            PatternIn::Cmdline(p) => {
                write!(f, "cmd_line[{}]", p)
            },
            PatternIn::Name(p) => {
                write!(f, "name[{}]", p)
            },
        }
    }
}

impl<P> MatchBy<PatternIn<P>> for sysinfo::Process
where
    sysinfo::Process: PatternMatcher<P>,
    P: Display
{
    fn match_by(&self, matcher: PatternIn<P>) -> bool {
        match matcher {
            PatternIn::ExePath(pat) => self.matches_exe(pat),
            PatternIn::Cmdline(pat) => self.matches_cmdline(pat),
            PatternIn::Name(pat) => self.matches_name(pat),
        }
    }
}
