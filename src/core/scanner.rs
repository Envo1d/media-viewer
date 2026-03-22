use crate::core::models::{MediaItem, MediaType, ScanEvent};
use crate::data::db::Database;
use crate::utils::current_timestamp::current_timestamp;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use std::{fs, thread};
use walkdir::WalkDir;

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
    fn process_entry(root_path: &str, entry: &walkdir::DirEntry) -> Option<MediaItem> {
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
        let scan_id = current_timestamp();

        let (tx, rx) = unbounded::<MediaItem>();

        // === DB WORKER ===
        let db_thread = thread::spawn(move || {
            let mut db = Database::new();
            Self::db_worker(&mut db, rx, scan_id);
        });

        // === WORKERS ===
        let root = Arc::new(root_path);

        let walker = WalkDir::new(&*root)
            .min_depth(3)
            .into_iter()
            .filter_map(Result::ok);

        let num_threads = num_cpus::get();

        let walker = Arc::new(parking_lot::Mutex::new(walker));

        let mut handles = Vec::new();

        for _ in 0..num_threads {
            let tx = tx.clone();
            let root = root.clone();
            let walker = walker.clone();
            let ui_tx = ui_tx.clone();

            let handle = thread::spawn(move || {
                loop {
                    let entry = {
                        let mut w = walker.lock();
                        w.next()
                    };

                    let entry = match entry {
                        Some(e) => e,
                        None => break,
                    };

                    if let Some(item) = Self::process_entry(&root, &entry) {
                        tx.send(item.clone()).ok();

                        ui_tx.send(ScanEvent::Item(item)).ok();
                    }
                }
            });

            handles.push(handle);
        }

        drop(tx);

        for h in handles {
            h.join().ok();
        }

        db_thread.join().ok();

        ui_tx.send(ScanEvent::Finished).ok();
    }

    pub fn start(root_path: String, ui_tx: Sender<ScanEvent>) {
        thread::spawn(move || {
            Self::run(root_path, ui_tx);
        });
    }
}
