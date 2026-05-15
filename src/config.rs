use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Config {
    pub token: Option<String>,
}

fn config_path() -> PathBuf {
    let mut dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.push("updatenight");
    dir.push("config.json");
    dir
}

pub fn load() -> Config {
    let path = config_path();
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(cfg: &Config) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(cfg)?)?;
    Ok(())
}

pub fn logout() -> Result<()> {
    let mut cfg = load();
    cfg.token = None;
    save(&cfg)?;
    println!("Logged out.");
    Ok(())
}
