use serde::{Deserialize, Serialize};
use std::{fs, path::{Path, PathBuf}};
use toml;
use std::env;
use anyhow::Result;


#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub url: String,
    pub token: String,
    pub window_size: Option<(f32, f32)>,
    pub worklog_file: Option<String>,
}

fn get_default_config_path() -> Result<PathBuf> {
    Ok(env::current_exe()?.with_extension("toml"))
}

pub fn load_config(path: Option<&str>) -> Option<Config> {
    let path = match path {
        Some(p) => PathBuf::from(p),
        None => get_default_config_path().ok()?,
    };
    let config_str = fs::read_to_string(&path).ok()?;
    let config: Config = toml::from_str(&config_str).ok()?;
    Some(config)
}
