use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use serde::Deserialize;

use crate::watch::ProcessWatchConfig;

/// Main config for project. It is loaded from TOML or YAML in that order
#[derive(Debug, Deserialize)]
pub struct Config {
    pub profiles: Vec<ProcessWatchConfig>,
}


pub(crate) fn read_config(p: Option<PathBuf>) -> anyhow::Result<Config> {
    // let cfg_dir = dirs::config_dir()
    //                     .unwrap_or(PathBuf::from("~/.config/"))
    //                     .push("pswatch");

    let cfg_file = p.unwrap_or(
        xdg::BaseDirectories::with_prefix(env!("CARGO_PKG_NAME"))
            .with_context(|| "could not find config dir")?
            .get_config_file("config.toml"),
    );

    // .get_config_file(PathBuf::from("pswatch.toml"))

    // let cfg_file = PathBuf::from("./test.toml");

    Ok(toml::from_str(
        &fs::read_to_string(&cfg_file)
            .with_context(|| format!("loading config: {}", cfg_file.display()))?,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    //TODO:
    #[test]
    fn config() {
        
    }
}
