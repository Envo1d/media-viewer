use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdaterArgs {
    pub target_exe: PathBuf,

    pub download_url: String,

    pub sig_url: String,

    pub new_version: String,

    pub current_version: String,

    pub parent_pid: u32,

    pub autostart: bool,
}

pub fn parse_args() -> Result<UpdaterArgs, String> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        return Err(format!(
            "Usage: nexa-updater <args-json-path>\n\
             Got {} argument(s): {:?}",
            args.len() - 1,
            &args[1..]
        ));
    }

    let json_path = PathBuf::from(&args[1]);

    let json = fs::read_to_string(&json_path)
        .map_err(|e| format!("Cannot read args file `{}`: {e}", json_path.display()))?;

    let _ = fs::remove_file(&json_path);

    serde_json::from_str::<UpdaterArgs>(&json).map_err(|e| format!("Invalid args JSON: {e}"))
}

pub fn write_args_file(args: &UpdaterArgs) -> std::io::Result<PathBuf> {
    let pid = std::process::id();
    let name = format!("nexa-updater-args-{pid}.json");
    let path = std::env::temp_dir().join(name);
    let json = serde_json::to_string_pretty(args).expect("UpdaterArgs is always serialisable");
    fs::write(&path, json)?;
    Ok(path)
}
