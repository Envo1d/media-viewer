use crate::core::models::{MediaItem, MediaType, ScanEvent};
use crate::data::db::Database;
use crate::utils::current_timestamp::current_timestamp;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use std::{fs, thread};
use ignore::WalkBuilder;

const BATCH_SIZE: usize = 500;

pub struct MediaScanner;

impl MediaScanner {
    // === DB WORKER ===
    fn db_worker(db: &mut Database, rx: Receiver<MediaItem>, scan_id: i64) {
        let mut batch = Vec::with_capacity(BATCH_SIZE);

        for item in rx {
            batch.push(item);

            if batch.len() >= BATCH_SIZE {
                db.upsert_batch(&batch, scan_id);
                batch.clear();
            }
        }

        if !batch.is_empty() {
            db.upsert_batch(&batch, scan_id);
        }

        db.delete_not_seen(scan_id);
    }

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

    fn run(root_path: String, ui_tx: Sender<ScanEvent>) {
        let (tx, rx) = unbounded();
        let mut db = Database::new();
        let scan_id = current_timestamp();
        
        let db_thread = thread::spawn(move || {
            Self::db_worker(&mut db, rx, scan_id);
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
                        tx.send(item.clone()).ok();
                        ui_tx.send(ScanEvent::Item(item)).ok();
                    }
                }
                ignore::WalkState::Continue
            })
        });
        
        drop(tx);
        db_thread.join().ok();
        ui_tx.send(ScanEvent::Finished).ok();
    }

    pub fn start(root_path: String, ui_tx: Sender<ScanEvent>) {
        thread::spawn(move || {
            Self::run(root_path, ui_tx);
        });
    }
}
