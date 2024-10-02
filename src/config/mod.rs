use std::{fs, path::PathBuf};

use anyhow::Context;
use log::debug;
use serde::Deserialize;
mod profile;

pub use profile::{Profile, CmdSchedule};

/// Main config for project. It is loaded from TOML or YAML in that order
#[derive(Debug, Deserialize)]
pub struct Config {
    pub profiles: Vec<Profile>,
}

fn parse_config(content: &str) -> anyhow::Result<Config> {
    Ok(toml::from_str(content)?)
}

pub fn read_config(p: Option<PathBuf>) -> anyhow::Result<Config> {
    let cfg_file = p.unwrap_or(
        xdg::BaseDirectories::with_prefix(env!("CARGO_PKG_NAME"))
            .with_context(|| "could not find config dir")?
            .get_config_file("config.toml"),
    );

    debug!("config file: {:?}", cfg_file);

    parse_config(
        &fs::read_to_string(&cfg_file)
            .with_context(|| format!("config: {}", cfg_file.display()))?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn config_template() -> anyhow::Result<()> {
        let config = indoc! {r###"
            [[profiles]]
            matching = { cmdline = "foo_seen" }

            [[profiles.commands]]
            condition = {seen = "5s"}
            exec = ["echo", "seen"]
            exec_end = ["echo", "end"]

            [[profiles.commands]]
            condition = {seen = "10s"}
            exec = ["echo", "still there"]

            ###

            [[profiles]]
            matching = { name = "foo_not_seen" }

            [[profiles.commands]]
            condition = {not_seen = "5s"}
            exec = ["echo", "not seen"]

            ###

            [[profiles]]
            matching = { exe_path = "b.n.*sh", regex = true }

            [[profiles.commands]]
            condition = {not_seen = "5s"}
            exec = ["echo", "not seen"]

        "###};

        let c = parse_config(config)?;
        assert_eq!(c.profiles.len(), 3, "non matching number of declared profiles");
        assert_eq!(c.profiles[0].commands.len(), 2, "non matching number of commands on profile1");
        Ok(())
    }
}
