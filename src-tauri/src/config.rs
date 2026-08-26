use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const CONFIG_FILE: &str = "airbox.json";

#[derive(Serialize, Deserialize)]
pub struct AppConfig {
    pub dir: PathBuf,
}

pub fn default_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join("Downloads").join("AirBox"))
        .unwrap_or_else(|| PathBuf::from("AirBox"))
}

pub fn load(app_config_dir: &Path) -> AppConfig {
    let path = app_config_dir.join(CONFIG_FILE);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<AppConfig>(&s).ok())
        .filter(|c| !c.dir.as_os_str().is_empty())
        .unwrap_or(AppConfig {
            dir: default_dir(),
        })
}

pub fn save(app_config_dir: &Path, dir: &Path) -> Result<(), String> {
    let path = app_config_dir.join(CONFIG_FILE);
    let json = serde_json::to_string(&AppConfig {
        dir: dir.to_path_buf(),
    })
    .map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}
