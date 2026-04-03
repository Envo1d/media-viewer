use crate::core::models::{DbCommand, MediaItem, MediaType, ScanEvent};
use crate::utils::current_timestamp;
use crossbeam_channel::{unbounded, Sender};
use ignore::WalkBuilder;
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use std::{fs, thread};

const BATCH_SIZE: usize = 500;

pub struct MediaScanner;

impl MediaScanner {
    fn process_entry(root_path: &str, entry: &ignore::DirEntry) -> Option<Arc<MediaItem>> {
        let path = entry.path();

        if !path.is_file() {
            return None;
        }

        let ext = path.extension()?.to_str()?.to_lowercase();

        let media_type = match ext.as_str() {
            "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" => MediaType::Video,
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tiff" | "tif" => MediaType::Image,
            _ => return None,
        };

        let metadata = fs::metadata(path).ok()?;
        let modified = metadata
            .modified()
            .ok()?
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_secs() as i64;

        let rel = path.strip_prefix(root_path).ok()?;
        let parts: Vec<_> = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect();

        if parts.len() < 3 {
            return None;
        }

        Some(Arc::new(MediaItem {
            path: path.to_string_lossy().to_string(),
            name: path.file_name()?.to_string_lossy().to_string(),
            media_type,
            category: parts[0].to_string(),
            author: parts[1].to_string(),
            modified,
        }))
    }

    fn run(root_path: String, ui_tx: Sender<ScanEvent>, db_tx: Sender<DbCommand>) {
        let scan_id = current_timestamp();
        let (item_tx, item_rx) = unbounded::<Arc<MediaItem>>();

        let db_tx_clone = db_tx.clone();
        let ui_tx_clone = ui_tx.clone();

        let aggregator = thread::spawn(move || {
            let mut batch = Vec::with_capacity(BATCH_SIZE);

            for item in item_rx {
                batch.push(item);

                if batch.len() >= BATCH_SIZE {
                    let to_send = std::mem::take(&mut batch);
                    let count = to_send.len() as u64;

                    db_tx_clone
                        .send(DbCommand::UpsertBatch(to_send, scan_id))
                        .ok();

                    ui_tx_clone.send(ScanEvent::Progress(count)).ok();
                }
            }

            if !batch.is_empty() {
                let count = batch.len() as u64;
                db_tx_clone
                    .send(DbCommand::UpsertBatch(batch, scan_id))
                    .ok();
                ui_tx_clone.send(ScanEvent::Progress(count)).ok();
            }

            db_tx_clone.send(DbCommand::DeleteNotSeen(scan_id)).ok();
        });

        let root = Arc::new(root_path);

        let walker = WalkBuilder::new(&*root)
            .hidden(false)
            .git_ignore(false)
            .threads(num_cpus::get())
            .build_parallel();

        walker.run(|| {
            let tx = item_tx.clone();
            let root = root.clone();

            Box::new(move |result| {
                if let Ok(entry) = result {
                    if let Some(item) = Self::process_entry(&root, &entry) {
                        tx.send(item).ok();
                    }
                }
                ignore::WalkState::Continue
            })
        });

        drop(item_tx);

        aggregator.join().ok();

        ui_tx.send(ScanEvent::Finished).ok();
    }

    pub fn start(root_path: String, ui_tx: Sender<ScanEvent>, db_tx: Sender<DbCommand>) {
        thread::spawn(move || {
            Self::run(root_path, ui_tx, db_tx);
        });
    }
}
