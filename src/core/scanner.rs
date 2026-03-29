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
    // === FILE PROCESSING ===
    fn process_entry(root_path: &str, entry: &ignore::DirEntry) -> Option<MediaItem> {
        let path = entry.path();

        if !path.is_file() {
            return None;
        }

        let ext = path.extension()?.to_str()?.to_lowercase();

        let media_type = match ext.as_str() {
            "mp4" | "mkv" | "avi" => MediaType::Video,
            "jpg" | "png" | "jpeg" | "gif" => MediaType::Image,
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

        Some(MediaItem {
            path: path.to_string_lossy().to_string(),
            name: path.file_name()?.to_string_lossy().to_string(),
            media_type,
            category: parts[0].to_string(),
            author: parts[1].to_string(),
            modified,
        })
    }

    fn run(root_path: String, ui_tx: Sender<ScanEvent>, db_tx: Sender<DbCommand>) {
        let scan_id = current_timestamp();
        let (tx, rx) = unbounded::<MediaItem>();

        // === AGGREGATOR THREAD ===
        let db_tx_clone = db_tx.clone();
        let aggregator = thread::spawn(move || {
            let mut batch = Vec::with_capacity(BATCH_SIZE);

            for item in rx {
                batch.push(item);

                if batch.len() >= BATCH_SIZE {
                    let to_send = std::mem::take(&mut batch);
                    db_tx_clone
                        .send(DbCommand::UpsertBatch(to_send, scan_id))
                        .ok();
                }
            }

            if !batch.is_empty() {
                db_tx_clone
                    .send(DbCommand::UpsertBatch(batch, scan_id))
                    .ok();
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
            let tx = tx.clone();
            let ui_tx = ui_tx.clone();
            let root = root.clone();

            Box::new(move |result| {
                if let Ok(entry) = result {
                    if let Some(item) = Self::process_entry(&root, &entry) {
                        ui_tx.send(ScanEvent::Item(item.clone())).ok();

                        tx.send(item).ok();
                    }
                }

                ignore::WalkState::Continue
            })
        });

        drop(tx); // важно!

        aggregator.join().ok();

        ui_tx.send(ScanEvent::Finished).ok();
    }

    pub fn start(root_path: String, ui_tx: Sender<ScanEvent>, db_tx: Sender<DbCommand>) {
        thread::spawn(move || {
            Self::run(root_path, ui_tx, db_tx);
        });
    }
}
