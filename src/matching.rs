use std::{fmt::Display, os::unix::ffi::OsStrExt};

use memchr::memmem;
use regex::Regex;
use serde::{de, Deserialize};

// TODO!:
/// Match a process by a given `Criteria'
pub trait MatchBy<Criteria>
where
    Criteria: Display,
{
    fn match_by(&self, matcher: Criteria) -> bool;
}

//TODO: handle different type of patterns (String, Regex ...)
// pub trait PatternMatcher<Pat> where Pat: String, Regex ...

/// A PatternMatcher for processes. Matches a running process given a generic pattern P
trait MatchProcByPattern<P> {
    fn matches_exe(&self, pattern: P) -> bool;
    fn matches_cmdline(&self, pattern: P) -> bool;
    fn matches_name(&self, pattern: P) -> bool;
}

// Raw structures for deseiralizing matchers
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
enum PatternInRaw {
    ExePath(String),
    Cmdline(String),
    Name(String)
}

#[derive(Deserialize, Clone, Debug)]
struct ProcessMatcherRaw {
    #[serde(flatten)]
    pattern: PatternInRaw,
    regex: Option<bool>
}

//NOTE: help from https://users.rust-lang.org/t/serde-deserializing-a-generic-enum/117560
impl TryFrom<ProcessMatcherRaw> for ProcessMatcher {
    type Error =  de::value::Error;

    fn try_from(raw: ProcessMatcherRaw) -> Result<Self, Self::Error> {
        if raw.regex.is_some_and(|x| x)  {
            let pattern = convert_pattern(raw.pattern, parse_regex)?;
            Ok(ProcessMatcher::RegexPattern(pattern))
        } else {
            let pattern = convert_pattern(raw.pattern, Ok)?;
            Ok(ProcessMatcher::StringPattern(pattern))
        }
    }
}

fn convert_pattern<F, P, E>(raw: PatternInRaw, convert: F) -> Result<PatternIn<P>, E>
where
    F: FnOnce(String) -> Result<P, E>
{
    Ok(match raw {
        PatternInRaw::ExePath(s) => PatternIn::ExePath(convert(s)?),
        PatternInRaw::Cmdline(s) => PatternIn::Cmdline(convert(s)?),
        PatternInRaw::Name(s) => PatternIn::Name(convert(s)?),
    })
}

fn parse_regex(raw: String) -> Result<Regex, de::value::Error> {
    raw.parse::<Regex>().map_err(de::Error::custom)
}

#[derive(Deserialize, Debug, Clone)]
#[serde(try_from = "ProcessMatcherRaw")]
pub enum ProcessMatcher {
    StringPattern(PatternIn<String>),
    RegexPattern(PatternIn<Regex>)
}

impl From<PatternIn<String>> for ProcessMatcher {
    fn from(value: PatternIn<String>) -> Self {
        Self::StringPattern(value)
    }
}

impl From<PatternIn<Regex>> for ProcessMatcher {
    fn from(value: PatternIn<Regex>) -> Self {
        Self::RegexPattern(value)
    }
}

impl Display for ProcessMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StringPattern(p) => {
                p.fmt(f)
            },
            Self::RegexPattern(p) => {p.fmt(f)},
        }
    }
}


#[derive(Deserialize, Clone, Debug)]
pub enum PatternIn<P> {
    ExePath(P),
    Cmdline(P),
    Name(P),
}


impl MatchProcByPattern<String> for sysinfo::Process {
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

impl MatchProcByPattern<Regex> for sysinfo::Process {
    fn matches_exe(&self, pattern: Regex) -> bool {
        self.exe()
        .and_then(|exe_name| exe_name.as_os_str().to_str())
        .is_some_and(|hay| pattern.is_match(hay))
    }

    fn matches_cmdline(&self, pattern: Regex) -> bool {
        pattern.is_match(&self.cmd().join(" "))
    }

    fn matches_name(&self, pattern: Regex) -> bool {
        pattern.is_match(self.name())
    }
}

impl<P> Display for PatternIn<P> where P: Display {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatternIn::ExePath(p) => {
                write!(f, "exe_path[{}]", p)
            },
            PatternIn::Cmdline(p) => {
                write!(f, "cmdline[{}]", p)
            },
            PatternIn::Name(p) => {
                write!(f, "name[{}]", p)
            },
        }
    }
}

impl<P> MatchBy<PatternIn<P>> for sysinfo::Process
where
    sysinfo::Process: MatchProcByPattern<P>,
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

impl MatchBy<ProcessMatcher> for sysinfo::Process {
    fn match_by(&self, matcher: ProcessMatcher) -> bool {
        match matcher {
            ProcessMatcher::StringPattern(pat) => self.match_by(pat),
            ProcessMatcher::RegexPattern(pat) => self.match_by(pat),
        }
    }
}

