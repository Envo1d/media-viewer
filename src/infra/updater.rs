use crate::core::models::{GhRelease, UpdateCmd, UpdateEvent, UpdateState};
use crossbeam_channel::{bounded, Receiver, Sender};
use semver::Version;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub const GITHUB_REPO: &str = "Envo1d/media-viewer";
pub const RELEASE_ASSET_NAME: &str = "Nexa.exe";
const USER_AGENT: &str = concat!("Nexa/", env!("CARGO_PKG_VERSION"));
const CHECK_TIMEOUT_SECS: u64 = 15;
const DOWNLOAD_TIMEOUT_SECS: u64 = 120;
const DOWNLOAD_CHUNK: usize = 65_536;

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn is_newer(current: &str, candidate: &str) -> bool {
    let current = Version::parse(current).unwrap_or_else(|_| Version::new(0, 0, 0));
    let candidate =
        Version::parse(candidate.trim_start_matches('v')).unwrap_or_else(|_| Version::new(0, 0, 0));

    candidate > current
}

pub struct UpdateWorker {
    cmd_tx: Sender<UpdateCmd>,
    pub event_rx: Receiver<UpdateEvent>,
    cancel: Arc<AtomicBool>,
}

impl UpdateWorker {
    pub fn spawn() -> Self {
        let (cmd_tx, cmd_rx) = bounded::<UpdateCmd>(8);
        let (event_tx, event_rx) = bounded::<UpdateEvent>(64);
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_clone = cancel.clone();

        thread::Builder::new()
            .name("nexa-updater".into())
            .spawn(move || worker_loop(cmd_rx, event_tx, cancel_clone))
            .expect("failed to spawn updater thread");

        Self {
            cmd_tx,
            event_rx,
            cancel,
        }
    }

    pub fn check(&self) {
        self.cmd_tx.send(UpdateCmd::Check).ok();
    }

    pub fn download(&self, version: String, url: String, dest_dir: PathBuf) {
        self.cancel.store(false, Ordering::Relaxed);
        self.cmd_tx
            .send(UpdateCmd::Download {
                version,
                url,
                dest_dir,
            })
            .ok();
    }

    pub fn cancel_download(&self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.cmd_tx.send(UpdateCmd::CancelDownload).ok();
    }

    pub fn poll(&self) -> Vec<UpdateEvent> {
        self.event_rx.try_iter().collect()
    }
}

fn worker_loop(
    cmd_rx: Receiver<UpdateCmd>,
    event_tx: Sender<UpdateEvent>,
    cancel: Arc<AtomicBool>,
) {
    for cmd in cmd_rx {
        match cmd {
            UpdateCmd::Check => do_check(&event_tx),
            UpdateCmd::Download {
                version,
                url,
                dest_dir,
            } => {
                do_download(&version, &url, &dest_dir, &event_tx, &cancel);
            }
            UpdateCmd::CancelDownload => {}
        }
    }
}

fn do_check(tx: &Sender<UpdateEvent>) {
    tx.send(UpdateEvent::StateChanged(UpdateState::Checking))
        .ok();

    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    let state = (|| -> Result<UpdateState, String> {
        let resp = build_agent(CHECK_TIMEOUT_SECS)
            .get(&url)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "application/vnd.github+json")
            .call()
            .map_err(|e| format!("HTTP error: {e}"))?;

        let release: GhRelease = resp
            .into_body()
            .read_json()
            .map_err(|e| format!("JSON parse error: {e}"))?;

        if !is_newer(current_version(), &release.tag_name) {
            return Ok(UpdateState::UpToDate);
        }

        let asset = release
            .assets
            .into_iter()
            .find(|a| a.name.eq_ignore_ascii_case(RELEASE_ASSET_NAME))
            .ok_or_else(|| {
                format!(
                    "Release {} has no asset named '{}'",
                    release.tag_name, RELEASE_ASSET_NAME
                )
            })?;

        Ok(UpdateState::Available {
            version: release.tag_name.trim_start_matches('v').to_owned(),
            download_url: asset.browser_download_url,
            size_bytes: asset.size,
        })
    })()
    .unwrap_or_else(UpdateState::Error);

    tx.send(UpdateEvent::StateChanged(state)).ok();
}

fn do_download(
    version: &str,
    url: &str,
    dest_dir: &Path,
    tx: &Sender<UpdateEvent>,
    cancel: &Arc<AtomicBool>,
) {
    match download_inner(version, url, dest_dir, tx, cancel) {
        Ok(path) => {
            tx.send(UpdateEvent::StateChanged(UpdateState::ReadyToInstall {
                version: version.to_owned(),
                staged_path: path,
            }))
            .ok();
        }
        Err(e) => {
            if cancel.load(Ordering::Relaxed) {
                tx.send(UpdateEvent::StateChanged(UpdateState::Idle)).ok();
            } else {
                tx.send(UpdateEvent::StateChanged(UpdateState::Error(e)))
                    .ok();
            }
        }
    }
}

fn download_inner(
    version: &str,
    url: &str,
    dest_dir: &Path,
    tx: &Sender<UpdateEvent>,
    cancel: &Arc<AtomicBool>,
) -> Result<PathBuf, String> {
    fs::create_dir_all(dest_dir).map_err(|e| format!("Cannot create temp dir: {e}"))?;

    let dest_path = dest_dir.join(format!("Nexa_{version}_update.exe"));
    if dest_path.exists() {
        fs::remove_file(&dest_path).ok();
    }

    let resp = build_agent(DOWNLOAD_TIMEOUT_SECS)
        .get(url)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;

    let total_bytes: u64 = resp
        .headers()
        .get("Content-Length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let mut file = fs::File::create(&dest_path).map_err(|e| format!("Cannot create file: {e}"))?;

    let mut reader = resp.into_body().into_reader();
    let mut buf = vec![0u8; DOWNLOAD_CHUNK];
    let mut bytes_done: u64 = 0;

    loop {
        if cancel.load(Ordering::Relaxed) {
            drop(file);
            fs::remove_file(&dest_path).ok();
            return Err("Cancelled".into());
        }

        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("Read error: {e}"))?;
        if n == 0 {
            break;
        }

        file.write_all(&buf[..n])
            .map_err(|e| format!("Write error: {e}"))?;
        bytes_done += n as u64;

        let progress = if total_bytes > 0 {
            (bytes_done as f32 / total_bytes as f32).min(1.0)
        } else {
            0.0
        };

        tx.send(UpdateEvent::DownloadProgress {
            bytes_done,
            total_bytes,
            progress,
        })
        .ok();
    }

    file.flush().map_err(|e| format!("Flush error: {e}"))?;
    drop(file);

    if total_bytes > 0 && bytes_done < total_bytes {
        fs::remove_file(&dest_path).ok();
        return Err(format!(
            "Incomplete: got {bytes_done} of {total_bytes} bytes"
        ));
    }

    Ok(dest_path)
}

#[cfg(windows)]
pub fn apply_update_and_restart(staged_path: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    let current_exe =
        std::env::current_exe().map_err(|e| format!("Cannot find current exe: {e}"))?;
    let exe_dir = current_exe
        .parent()
        .ok_or("Current exe has no parent directory")?;

    let pending_path = exe_dir.join("Nexa_pending_update.exe");
    fs::copy(staged_path, &pending_path).map_err(|e| format!("Cannot stage update: {e}"))?;

    let bat_path = exe_dir.join("nexa_update_helper.bat");
    let script = format!(
        "@echo off\r\n\
         ping 127.0.0.1 -n 3 >nul\r\n\
         move /y \"{pending}\" \"{exe}\"\r\n\
         if errorlevel 1 (\r\n\
             echo Update failed: could not replace executable. >&2\r\n\
             del \"%~f0\"\r\n\
             exit /b 1\r\n\
         )\r\n\
         start \"\" \"{exe}\"\r\n\
         del \"%~f0\"\r\n",
        pending = pending_path.to_string_lossy(),
        exe = current_exe.to_string_lossy(),
    );

    fs::write(&bat_path, script.as_bytes())
        .map_err(|e| format!("Cannot write helper script: {e}"))?;

    std::process::Command::new("cmd")
        .args(["/c", bat_path.to_str().unwrap_or("")])
        .creation_flags(0x00000008) // DETACHED_PROCESS
        .spawn()
        .map_err(|e| format!("Cannot spawn helper: {e}"))?;

    thread::sleep(Duration::from_millis(200));
    std::process::exit(0);
}

#[cfg(not(windows))]
pub fn apply_update_and_restart(_staged_path: &Path) -> Result<(), String> {
    Err("Auto-update is only supported on Windows.".into())
}

pub fn cleanup_leftover_files() {
    let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
    else {
        return;
    };
    for name in &["Nexa_pending_update.exe", "nexa_update_helper.bat"] {
        let p = exe_dir.join(name);
        if p.exists() {
            fs::remove_file(p).ok();
        }
    }
}

pub fn update_staging_dir() -> PathBuf {
    std::env::temp_dir().join("nexa_updates")
}

pub fn cleanup_staged_downloads() {
    let dir = update_staging_dir();
    if let Ok(entries) = fs::read_dir(&dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension()
                .and_then(|x| x.to_str())
                .map(|x| x.eq_ignore_ascii_case("exe"))
                .unwrap_or(false)
            {
                fs::remove_file(p).ok();
            }
        }
    }
}

fn build_agent(timeout_secs: u64) -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_send_request(Some(Duration::from_secs(timeout_secs)))
        .timeout_recv_response(Some(Duration::from_secs(timeout_secs)))
        .build();

    config.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_ordering() {
        assert!(is_newer("0.9.0", "v0.9.1"));
        assert!(is_newer("0.9.0", "1.0.0"));
        assert!(is_newer("0.9.0", "0.10.0"));
        assert!(!is_newer("1.0.0", "v1.0.0"));
        assert!(!is_newer("1.0.1", "1.0.0"));
    }
}
