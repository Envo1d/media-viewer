use crate::protocol::UpdaterArgs;
use crate::verify;
use crossbeam_channel::{Receiver, Sender};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub enum WorkerCmd {
    Cancel,
    Retry,
}

#[derive(Debug, Clone)]
pub enum WorkerEvent {
    WaitingForParent,
    DownloadStarted { total_bytes: u64 },
    DownloadProgress { done_bytes: u64, total_bytes: u64 },
    Verifying,
    Applying,
    Done,
    Error { message: String },
}

const CHUNK_SIZE: usize = 65_536;
const PARENT_WAIT_MS: u32 = 15_000;
const PARENT_POLL_INTERVAL_MS: u64 = 100;
const DOWNLOAD_CONNECT_SECS: u64 = 15;
const DOWNLOAD_RECV_SECS: u64 = 120;
const USER_AGENT: &str = concat!("nexa-updater/", env!("CARGO_PKG_VERSION"));

pub fn spawn(args: UpdaterArgs) -> (Sender<WorkerCmd>, Receiver<WorkerEvent>, Arc<AtomicBool>) {
    let (cmd_tx, cmd_rx) = crossbeam_channel::bounded::<WorkerCmd>(8);
    let (event_tx, event_rx) = crossbeam_channel::bounded::<WorkerEvent>(64);
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_clone = Arc::clone(&cancel);

    thread::Builder::new()
        .name("nexa-updater-worker".into())
        .spawn(move || run(args, event_tx, cmd_rx, cancel_clone))
        .expect("failed to spawn updater worker");

    (cmd_tx, event_rx, cancel)
}

fn run(
    args: UpdaterArgs,
    events: Sender<WorkerEvent>,
    cmds: Receiver<WorkerCmd>,
    cancel: Arc<AtomicBool>,
) {
    macro_rules! bail {
        ($msg:expr) => {{
            tracing::error!("{}", $msg);
            events
                .send(WorkerEvent::Error {
                    message: $msg.to_string(),
                })
                .ok();
            return;
        }};
    }

    events.send(WorkerEvent::WaitingForParent).ok();

    if !wait_for_parent(args.parent_pid) {
        tracing::warn!(
            pid = args.parent_pid,
            "parent did not exit within timeout; proceeding anyway"
        );
    }

    if cancel.load(Ordering::Relaxed) {
        return;
    }

    let tmp_dir = std::env::temp_dir().join("nexa_updates");
    if let Err(e) = fs::create_dir_all(&tmp_dir) {
        bail!(format!("Cannot create temp directory: {e}"));
    }

    let exe_dest = tmp_dir.join(format!("Nexa_{}_update.exe", args.new_version));
    let sig_dest = tmp_dir.join(format!("Nexa_{}_update.exe.sig", args.new_version));

    let _ = fs::remove_file(&exe_dest);
    let _ = fs::remove_file(&sig_dest);

    tracing::info!(url = %args.download_url, dest = %exe_dest.display(), "Starting exe download");

    match download_file(
        &args.download_url,
        &exe_dest,
        &cancel,
        |done, total| {
            events
                .send(WorkerEvent::DownloadProgress {
                    done_bytes: done,
                    total_bytes: total,
                })
                .ok();
        },
        true,
        &events,
    ) {
        Ok(()) => {}
        Err(DownloadError::Cancelled) => {
            cleanup(&exe_dest, &sig_dest);
            return;
        }
        Err(e) => {
            cleanup(&exe_dest, &sig_dest);
            bail!(format!("Download failed: {e}"));
        }
    }

    if cancel.load(Ordering::Relaxed) {
        cleanup(&exe_dest, &sig_dest);
        return;
    }

    tracing::info!(url = %args.sig_url, dest = %sig_dest.display(), "Downloading signature");

    match download_file(&args.sig_url, &sig_dest, &cancel, |_, _| {}, false, &events) {
        Ok(()) => {}
        Err(DownloadError::Cancelled) => {
            cleanup(&exe_dest, &sig_dest);
            return;
        }
        Err(e) => {
            cleanup(&exe_dest, &sig_dest);
            bail!(format!("Signature download failed: {e}"));
        }
    }

    events.send(WorkerEvent::Verifying).ok();
    tracing::info!("Verifying Ed25519 signature");

    if let Err(e) = verify::verify_file(&exe_dest, &sig_dest) {
        cleanup(&exe_dest, &sig_dest);
        bail!(format!("Signature verification failed: {e}"));
    }

    let _ = fs::remove_file(&sig_dest);

    events.send(WorkerEvent::Applying).ok();
    tracing::info!(target = %args.target_exe.display(), "Applying update");

    if let Err(e) = apply_update(&exe_dest, &args.target_exe) {
        let _ = fs::remove_file(&exe_dest);
        bail!(format!("Failed to install update: {e}"));
    }

    events.send(WorkerEvent::Done).ok();
    tracing::info!("Update applied successfully");

    if args.autostart {
        thread::sleep(Duration::from_millis(1_800));
        tracing::info!(exe = %args.target_exe.display(), "Relaunching Nexa");
        let _ = std::process::Command::new(&args.target_exe).spawn();
    }
}

#[cfg(windows)]
fn wait_for_parent(pid: u32) -> bool {
    use windows::Win32::{
        Foundation::{CloseHandle, WAIT_OBJECT_0},
        System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE},
    };
    unsafe {
        match OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            Ok(handle) if !handle.is_invalid() => {
                let result = WaitForSingleObject(handle, PARENT_WAIT_MS);
                let _ = CloseHandle(handle);
                result == WAIT_OBJECT_0
            }
            _ => {
                tracing::debug!(pid, "OpenProcess failed; process likely already exited");
                true
            }
        }
    }
}

#[cfg(not(windows))]
fn wait_for_parent(pid: u32) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_millis(PARENT_WAIT_MS as u64);
    while std::time::Instant::now() < deadline {
        let proc_path = format!("/proc/{pid}");
        if !std::path::Path::new(&proc_path).exists() {
            return true;
        }
        thread::sleep(Duration::from_millis(PARENT_POLL_INTERVAL_MS));
    }
    false
}

fn apply_update(source: &Path, target: &Path) -> Result<(), String> {
    let parent = target.parent().ok_or("target has no parent directory")?;

    fs::create_dir_all(parent).map_err(|e| format!("Cannot access target directory: {e}"))?;

    if fs::rename(source, target).is_ok() {
        tracing::info!("Applied via direct rename");
        return Ok(());
    }

    let staging = parent.join("Nexa_update_staging.exe");
    let backup = parent.join("Nexa_old.exe");

    fs::copy(source, &staging).map_err(|e| format!("Copy to staging failed: {e}"))?;

    if target.exists() {
        if let Err(e) = fs::rename(target, &backup) {
            let _ = fs::remove_file(&staging);
            return Err(format!("Cannot move current exe to backup: {e}"));
        }
    }

    if let Err(e) = fs::rename(&staging, target) {
        let _ = fs::rename(&backup, target);
        let _ = fs::remove_file(&staging);
        return Err(format!("Final rename failed: {e}"));
    }

    let _ = fs::remove_file(&backup);
    let _ = fs::remove_file(source);

    tracing::info!("Applied via copy+rename (cross-volume path)");
    Ok(())
}

fn cleanup(exe: &Path, sig: &Path) {
    let _ = fs::remove_file(exe);
    let _ = fs::remove_file(sig);
}

#[derive(Debug)]
enum DownloadError {
    Cancelled,
    Http(String),
    Io(std::io::Error),
}

impl std::fmt::Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => write!(f, "cancelled by user"),
            Self::Http(e) => write!(f, "HTTP error: {e}"),
            Self::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl From<std::io::Error> for DownloadError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

fn build_agent() -> ureq::Agent {
    let cfg = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(DOWNLOAD_CONNECT_SECS)))
        .timeout_recv_response(Some(Duration::from_secs(DOWNLOAD_RECV_SECS)))
        .build();
    cfg.into()
}

fn download_file(
    url: &str,
    dest: &Path,
    cancel: &AtomicBool,
    on_progress: impl Fn(u64, u64),
    emit_started: bool,
    events: &Sender<WorkerEvent>,
) -> Result<(), DownloadError> {
    let resp = build_agent()
        .get(url)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| DownloadError::Http(e.to_string()))?;

    let total_bytes: u64 = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    if emit_started {
        events
            .send(WorkerEvent::DownloadStarted { total_bytes })
            .ok();
    }

    let mut file = fs::File::create(dest)?;
    let mut reader = resp.into_body().into_reader();
    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut done_bytes: u64 = 0;

    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(DownloadError::Cancelled);
        }

        let n = reader.read(&mut buf).map_err(DownloadError::Io)?;
        if n == 0 {
            break;
        }

        file.write_all(&buf[..n]).map_err(DownloadError::Io)?;
        done_bytes += n as u64;
        on_progress(done_bytes, total_bytes);
    }

    file.flush().map_err(DownloadError::Io)?;
    drop(file);

    if total_bytes > 0 && done_bytes < total_bytes {
        let _ = fs::remove_file(dest);
        return Err(DownloadError::Http(format!(
            "incomplete: received {done_bytes} of {total_bytes} bytes"
        )));
    }

    tracing::info!(url, bytes = done_bytes, dest = %dest.display(), "Download complete");
    Ok(())
}
