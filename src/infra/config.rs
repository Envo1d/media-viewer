use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub library_path: Option<PathBuf>,
    pub database_path: PathBuf,
    pub last_scan_date: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            library_path: None,
            database_path: PathBuf::from("../../media_vault.db"),
            last_scan_date: None,
        }
    }
}

impl AppConfig {
    fn get_config_path() -> PathBuf {
        ProjectDirs::from("com", "envoid", "media_viewer")
            .map(|proj_dirs| {
                let config_dir = proj_dirs.config_dir();
                fs::create_dir_all(config_dir).ok();
                config_dir.join("config.json")
            })
            .unwrap_or_else(|| PathBuf::from("config.json"))
    }

    pub fn load() -> Self {
        let path = Self::get_config_path();
        if let Ok(data) = fs::read_to_string(path) {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::get_config_path();
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}
