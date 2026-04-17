use crate::core::models::{DbCommand, MediaType, ScanEvent, StagingItem};
use crate::utils::current_timestamp;
use crossbeam_channel::{bounded, Sender};
use ignore::WalkBuilder;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::UNIX_EPOCH;

const BATCH_SIZE: usize = 200;
const WALKER_QUEUE: usize = BATCH_SIZE * 4;
const MAX_WALKER_THREADS: usize = 2;

const ARCHIVE_EXTENSIONS: &[&str] = &[
    "zip", "rar", "7z", "tar", "gz", "bz2", "xz", "zst", "lz4", "cab", "iso",
];

fn staging_media_type(ext: &str) -> Option<MediaType> {
    match ext {
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" => Some(MediaType::Video),
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tiff" | "tif" => Some(MediaType::Image),
        _ => None,
    }
}

fn is_archive(ext: &str) -> bool {
    ARCHIVE_EXTENSIONS.contains(&ext)
}

fn build_staging_item(path: &Path) -> Option<Arc<StagingItem>> {
    if !path.is_file() {
        return None;
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())?;

    if is_archive(&ext) {
        return None;
    }

    let media_type = staging_media_type(&ext)?;

    let metadata = fs::metadata(path).ok()?;
    let modified = metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;

    let name = path.file_name()?.to_string_lossy().to_string();
    let path_str = path.to_string_lossy().to_string();

    Some(Arc::new(StagingItem {
        path: path_str,
        name,
        media_type,
        modified,
    }))
}

pub struct StagingScanner;

impl StagingScanner {
    fn run(root_path: String, ui_tx: Sender<ScanEvent>, db_tx: Sender<DbCommand>) {
        let scan_id = current_timestamp();

        let (item_tx, item_rx) = bounded::<Arc<StagingItem>>(WALKER_QUEUE);
        let db_tx_clone = db_tx.clone();
        let ui_tx_clone = ui_tx.clone();

        let aggregator = thread::Builder::new()
            .name("nexa-staging-aggregator".into())
            .spawn(move || {
                let mut batch = Vec::with_capacity(BATCH_SIZE);

                for item in item_rx {
                    batch.push(item);

                    if batch.len() >= BATCH_SIZE {
                        let to_send = std::mem::take(&mut batch);
                        let count = to_send.len() as u64;
                        db_tx_clone
                            .send(DbCommand::StagingUpsertBatch(to_send, scan_id))
                            .ok();
                        ui_tx_clone.send(ScanEvent::Progress(count)).ok();
                    }
                }

                if !batch.is_empty() {
                    let count = batch.len() as u64;
                    db_tx_clone
                        .send(DbCommand::StagingUpsertBatch(batch, scan_id))
                        .ok();
                    ui_tx_clone.send(ScanEvent::Progress(count)).ok();
                }

                db_tx_clone
                    .send(DbCommand::StagingDeleteNotSeen(scan_id))
                    .ok();
            })
            .expect("Failed to spawn staging aggregator thread");

        let walker_threads = num_cpus::get().clamp(1, MAX_WALKER_THREADS);

        WalkBuilder::new(&root_path)
            .hidden(false)
            .git_ignore(false)
            .threads(walker_threads)
            .build_parallel()
            .run(|| {
                let tx = item_tx.clone();
                Box::new(move |result| {
                    if let Ok(entry) = result
                        && let Some(item) = build_staging_item(entry.path())
                    {
                        tx.send(item).ok();
                    }
                    ignore::WalkState::Continue
                })
            });

        drop(item_tx);
        aggregator.join().ok();

        ui_tx.send(ScanEvent::Finished).ok();
    }

    pub fn start(root_path: String, ui_tx: Sender<ScanEvent>, db_tx: Sender<DbCommand>) {
        thread::Builder::new()
            .name("nexa-staging-scanner".into())
            .spawn(move || Self::run(root_path, ui_tx, db_tx))
            .expect("Failed to spawn staging scanner thread");
    }
}
