use std::{fmt::Display, os::unix::ffi::OsStrExt};

use memchr::memmem;
use regex::Regex;
use serde::{de, Deserialize, Deserializer};

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

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ProcessMatcher {
    StringPattern(PatternIn<String>),
    #[serde(deserialize_with = "deserialize_regex_pattern")]
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
#[serde(rename_all = "snake_case")]
pub enum PatternIn<P> {
    ExePath(P),
    Cmdline(P),
    Name(P),
}

// DEBUG:
// impl PatternIn<String> {
//     pub fn as_regex(self) -> PatternIn<Regex> {
//         match self {
//             PatternIn::ExePath(pat) => PatternIn::ExePath(Regex::new(&pat).unwrap()),
//             PatternIn::Cmdline(pat) => PatternIn::Cmdline(Regex::new(&pat).unwrap()),
//             PatternIn::Name(pat) => PatternIn::Name(Regex::new(&pat).unwrap()),
//         }
//     }
// }

//NOTE: help from https://users.rust-lang.org/t/serde-deserializing-a-generic-enum/117560
fn deserialize_regex_pattern<'de, D>(deserializer: D) -> Result<PatternIn<Regex>, D::Error>
where D: Deserializer<'de>
{
    let mut table = toml::Table::deserialize(deserializer)?;

    let regex = table.remove("regex")
        .and_then(|regex| regex.as_bool())
        .unwrap_or_default();

    if !regex {
        Err(de::Error::custom("not a regex pattern"))
    } else {

        // Since `Regex` does not implement the `Deserialize` trait, we first need to convert the `table` into a `PatternIn<String>`
        table
        .try_into::<PatternIn<String>>()
        .map_err(|err| {
                regex::Error::Syntax(format!(
                    "could not convert pattern into a `PatternIn<String>`: {}", err
                ))
        })
        // Then, we can map the `PatternIn<String>` into a `PatternIn<Regex>` and return it.
        .and_then(|pattern| {
            match pattern {
                PatternIn::Name(pat) => pat.parse::<Regex>().map(PatternIn::Name) ,
                PatternIn::ExePath(pat) => pat.parse::<Regex>().map(PatternIn::ExePath),
                PatternIn::Cmdline(pat) => pat.parse::<Regex>().map(PatternIn::Cmdline),
            }
        })
        .map_err(|err| {
            de::Error::custom(format!("could not convert pattern into a regex: {}", err))

        })
    }
}

// fn deserialize_regex_pattern<'de, D>(deserializer: D) -> Result<PatternIn<Regex>, D::Error>
// where
//     D: Deserializer<'de>,
// {
//     #[derive(Deserialize, Debug)]
//     struct Pattern {
//         #[serde(flatten)]
//         pattern: PatternIn<String>,
//         regex: bool
//     }
//
//     let pd: Pattern = Pattern::deserialize(deserializer).inspect_err(|e| eprintln!("PatternIn<Regex> error: {}", e))?;
//     dbg!(&pd);
//     if !pd.regex {
//         Err(de::Error::custom("not a regex pattern"))
//     } else {
//         match pd.pattern {
//             PatternIn::Cmdline(pat) => {
//                 match pat.parse::<Regex>() {
//                     Ok(regex) => Ok(PatternIn::Cmdline(regex)),
//                     Err(err) => Err(de::Error::custom(err))
//                 }
//             },
//             PatternIn::Name(pat) => {
//                 match pat.parse::<Regex>() {
//                     Ok(regex) => Ok(PatternIn::Name(regex)),
//                     Err(err) => Err(de::Error::custom(err))
//                 }
//             },
//             PatternIn::ExePath(pat) => {
//                 match pat.parse::<Regex>() {
//                     Ok(regex) => Ok(PatternIn::Name(regex)),
//                     Err(err) => Err(de::Error::custom(err))
//                 }
//
//             },
//         }
//     }
// }

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
        todo!()
    }

    fn matches_cmdline(&self, pattern: Regex) -> bool {
        todo!()
    }

    fn matches_name(&self, pattern: Regex) -> bool {
        todo!()
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

