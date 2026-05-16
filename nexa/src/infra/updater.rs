use crate::core::models::{GhRelease, UpdateCmd, UpdateEvent, UpdateState};
use crossbeam_channel::{bounded, Receiver, Sender};
use semver::Version;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

pub const GITHUB_REPO: &str = "Envo1d/media-viewer";
pub const RELEASE_EXE_NAME: &str = "Nexa.exe";
pub const RELEASE_SIG_NAME: &str = "Nexa.exe.sig";
pub const UPDATER_EXE_NAME: &str = "nexa-updater.exe";

const USER_AGENT: &str = concat!("Nexa/", env!("CARGO_PKG_VERSION"));
const CHECK_TIMEOUT_SECS: u64 = 15;

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn is_newer(current: &str, candidate: &str) -> bool {
    let cur = Version::parse(current).unwrap_or_else(|_| Version::new(0, 0, 0));
    let can =
        Version::parse(candidate.trim_start_matches('v')).unwrap_or_else(|_| Version::new(0, 0, 0));
    can > cur
}

pub struct UpdateWorker {
    cmd_tx: Sender<UpdateCmd>,
    pub event_rx: Receiver<UpdateEvent>,
}

impl UpdateWorker {
    pub fn spawn() -> Self {
        let (cmd_tx, cmd_rx) = bounded::<UpdateCmd>(8);
        let (evt_tx, evt_rx) = bounded::<UpdateEvent>(64);
        thread::Builder::new()
            .name("nexa-update-checker".into())
            .spawn(move || worker_loop(cmd_rx, evt_tx))
            .expect("failed to spawn update checker");
        Self {
            cmd_tx,
            event_rx: evt_rx,
        }
    }

    pub fn check(&self) {
        self.cmd_tx.send(UpdateCmd::Check).ok();
    }
    pub fn download(&self, _version: String, _url: String, _dest_dir: PathBuf) {}
    pub fn cancel_download(&self) {}
    pub fn poll(&self) -> Vec<UpdateEvent> {
        self.event_rx.try_iter().collect()
    }
}

fn worker_loop(cmd_rx: Receiver<UpdateCmd>, event_tx: Sender<UpdateEvent>) {
    for cmd in cmd_rx {
        if let UpdateCmd::Check = cmd {
            do_check(&event_tx);
        }
    }
}

fn do_check(tx: &Sender<UpdateEvent>) {
    tx.send(UpdateEvent::StateChanged(UpdateState::Checking))
        .ok();

    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");

    let state = (|| -> Result<UpdateState, String> {
        let resp = build_agent()
            .get(&url)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "application/vnd.github+json")
            .call()
            .map_err(|e| format!("HTTP error: {e}"))?;

        let release: GhRelease = resp
            .into_body()
            .read_json()
            .map_err(|e| format!("JSON: {e}"))?;

        if !is_newer(current_version(), &release.tag_name) {
            return Ok(UpdateState::UpToDate);
        }

        let exe_asset = release
            .assets
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(RELEASE_EXE_NAME))
            .ok_or_else(|| format!("No {RELEASE_EXE_NAME} in release"))?;

        let sig_exists = release
            .assets
            .iter()
            .any(|a| a.name.eq_ignore_ascii_case(RELEASE_SIG_NAME));

        if !sig_exists {
            return Err(format!(
                "Release missing {RELEASE_SIG_NAME} — cannot update safely"
            ));
        }

        let sig_url = exe_asset
            .browser_download_url
            .replace(RELEASE_EXE_NAME, RELEASE_SIG_NAME);

        let combined = format!("{}||{}", exe_asset.browser_download_url, sig_url);

        Ok(UpdateState::Available {
            version: release.tag_name.trim_start_matches('v').to_owned(),
            download_url: combined,
            size_bytes: exe_asset.size,
        })
    })()
    .unwrap_or_else(UpdateState::Error);

    tx.send(UpdateEvent::StateChanged(state)).ok();
}

#[cfg(windows)]
pub fn launch_updater_and_exit(new_version: &str, combined_url: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    let (exe_url, sig_url) = combined_url
        .split_once("||")
        .ok_or("Malformed combined URL")?;

    let current_exe =
        std::env::current_exe().map_err(|e| format!("Cannot locate current exe: {e}"))?;

    let updater_exe = current_exe
        .parent()
        .ok_or("No parent dir")?
        .join(UPDATER_EXE_NAME);

    if !updater_exe.exists() {
        return Err(format!(
            "{UPDATER_EXE_NAME} not found next to Nexa.exe.\n\
             Ensure both files are distributed together."
        ));
    }

    #[derive(serde::Serialize)]
    struct UpdaterArgs<'a> {
        target_exe: PathBuf,
        download_url: &'a str,
        sig_url: &'a str,
        new_version: &'a str,
        current_version: &'a str,
        parent_pid: u32,
        autostart: bool,
    }

    let args = UpdaterArgs {
        target_exe: current_exe.clone(),
        download_url: exe_url,
        sig_url,
        new_version,
        current_version: current_version(),
        parent_pid: std::process::id(),
        autostart: true,
    };

    let json = serde_json::to_string_pretty(&args).map_err(|e| format!("Serialise: {e}"))?;

    let args_file =
        std::env::temp_dir().join(format!("nexa-updater-args-{}.json", std::process::id()));

    std::fs::write(&args_file, &json).map_err(|e| format!("Write args: {e}"))?;

    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;

    let spawned = std::process::Command::new(&updater_exe)
        .arg(&args_file)
        .creation_flags(DETACHED_PROCESS | CREATE_BREAKAWAY_FROM_JOB)
        .spawn()
        .or_else(|_| {
            std::process::Command::new(&updater_exe)
                .arg(&args_file)
                .creation_flags(DETACHED_PROCESS)
                .spawn()
        });

    match spawned {
        Ok(_) => {
            tracing::info!(version = new_version, "Updater spawned — exiting");
            thread::sleep(Duration::from_millis(300));
            std::process::exit(0);
        }
        Err(e) => {
            let _ = std::fs::remove_file(&args_file);
            Err(format!("Failed to spawn {UPDATER_EXE_NAME}: {e}"))
        }
    }
}

#[cfg(not(windows))]
pub fn launch_updater_and_exit(_new_version: &str, _combined_url: &str) -> Result<(), String> {
    Err("Auto-update is only supported on Windows.".into())
}

pub fn cleanup_leftover_files() {
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
    {
        for name in &[
            "Nexa_pending_update.exe",
            "Nexa_old.exe",
            "Nexa_update_staging.exe",
        ] {
            let _ = std::fs::remove_file(dir.join(name));
        }
    }
}

pub fn cleanup_staged_downloads() {
    let dir = std::env::temp_dir().join("nexa_updates");
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension()
                .and_then(|x| x.to_str())
                .map(|x| x.eq_ignore_ascii_case("exe") || x == "sig")
                .unwrap_or(false)
            {
                std::fs::remove_file(p).ok();
            }
        }
    }
}

fn build_agent() -> ureq::Agent {
    let cfg = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_recv_response(Some(Duration::from_secs(CHECK_TIMEOUT_SECS)))
        .build();
    cfg.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn version_ordering() {
        assert!(is_newer("0.9.0", "v0.9.1"));
        assert!(is_newer("0.9.0", "1.0.0"));
        assert!(!is_newer("1.0.0", "v1.0.0"));
        assert!(!is_newer("1.0.1", "1.0.0"));
    }
}
