use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FolderMapping {
    pub copyright_depth: usize,
    pub artist_depth: usize,
}

impl Default for FolderMapping {
    fn default() -> Self {
        Self {
            copyright_depth: 0,
            artist_depth: 1,
        }
    }
}

impl FolderMapping {
    pub fn min_folder_depth(&self) -> usize {
        self.copyright_depth.max(self.artist_depth) + 1
    }
}

fn default_char_sep() -> String {
    " x ".to_owned()
}

fn default_video_subfolder() -> String {
    String::new()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub library_path: Option<PathBuf>,
    pub database_path: PathBuf,
    pub last_scan_date: Option<String>,
    pub cache_path: PathBuf,
    #[serde(default)]
    pub auto_scan: bool,
    #[serde(default)]
    pub folder_mapping: FolderMapping,
    #[serde(default = "default_char_sep")]
    pub character_separator: String,

    pub staging_path: Option<PathBuf>,

    #[serde(default = "default_video_subfolder")]
    pub video_subfolder: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            library_path: None,
            database_path: Self::get_db_path(),
            cache_path: Self::get_cache_dir(),
            last_scan_date: None,
            auto_scan: false,
            folder_mapping: FolderMapping::default(),
            character_separator: default_char_sep(),
            staging_path: None,
            video_subfolder: default_video_subfolder(),
        }
    }
}

impl AppConfig {
    fn get_proj_dirs() -> &'static ProjectDirs {
        static DIRS: OnceLock<ProjectDirs> = OnceLock::new();
        DIRS.get_or_init(|| {
            ProjectDirs::from("com", "envoid", "Nexa")
                .expect("Unable to locate application system folders")
        })
    }

    pub fn get_config_path() -> PathBuf {
        let dir = Self::get_proj_dirs().config_dir().to_path_buf();
        let _ = fs::create_dir_all(&dir);
        dir.join("settings.json")
    }

    pub fn get_db_path() -> PathBuf {
        let dir = Self::get_proj_dirs().data_local_dir().join("db");
        let _ = fs::create_dir_all(&dir);
        dir.join("vault.db")
    }

    pub fn get_cache_dir() -> PathBuf {
        let dir = Self::get_proj_dirs().cache_dir().join("thumbnails");
        let _ = fs::create_dir_all(&dir);
        dir
    }

    pub fn load() -> Self {
        let path = Self::get_config_path();
        if let Ok(data) = fs::read_to_string(path) {
            serde_json::from_str(&data).unwrap_or_else(|_| Self::default())
        } else {
            let config = Self::default();
            let _ = config.save();
            config
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::get_config_path();
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}
