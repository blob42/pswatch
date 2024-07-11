use std::{fs, path::PathBuf};

use anyhow::Context;
use log::debug;
use serde::Deserialize;

use crate::sched::Profile;


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

    //TODO:
    #[test]
    fn config_template() -> anyhow::Result<()> {
        let config = indoc! {r###"
            [[profiles]]
            pattern = "foo_seen"
            regex = false
            [[profiles.commands]]
            condition = {seen = "5s"}
            exec = ["echo", "seen"]

            [[profiles.commands]]
            condition = {seen = "10s"}
            exec = ["echo", "still there"]

            [[profiles]]
            pattern = "foo_not_seen"
            regex = false
            [[profiles.commands]]
            condition = {not_seen = "5s"}
            exec = ["echo", "not seen"]

        "###};

        let c = parse_config(config)?;
        assert_eq!(c.profiles.len(), 2);
        assert_eq!(c.profiles[0].commands.len(), 2);
        Ok(())
    }
}
