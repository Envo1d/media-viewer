use crate::core::models::{DbCommand, MediaItem, ScanEvent};
use crate::infra::config::FolderMapping;
use crate::utils::{build_media_item, current_timestamp};
use crossbeam_channel::{bounded, Sender};
use ignore::WalkBuilder;
use std::sync::Arc;
use std::thread;

const BATCH_SIZE: usize = 500;
const WALKER_QUEUE: usize = BATCH_SIZE * 8;
const MAX_WALKER_THREADS: usize = 4;

pub struct MediaScanner;

impl MediaScanner {
    fn run(
        root_path: String,
        mapping: FolderMapping,
        char_sep: String,
        excluded_dirs: Vec<String>,
        ui_tx: Sender<ScanEvent>,
        db_tx: Sender<DbCommand>,
    ) {
        let scan_id = current_timestamp();

        let (item_tx, item_rx) = bounded::<Arc<MediaItem>>(WALKER_QUEUE);

        let db_tx_clone = db_tx.clone();
        let ui_tx_clone = ui_tx.clone();

        let aggregator = thread::Builder::new()
            .name("nexa-scan-aggregator".into())
            .spawn(move || {
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
            })
            .expect("Failed to spawn scan aggregator thread");

        let root = Arc::new(root_path);
        let mapping = Arc::new(mapping);
        let char_sep = Arc::new(char_sep);
        let excluded_dirs = Arc::new(excluded_dirs);

        let walker_threads = num_cpus::get().clamp(1, MAX_WALKER_THREADS);

        WalkBuilder::new(&*root)
            .hidden(false)
            .git_ignore(false)
            .threads(walker_threads)
            .filter_entry(move |entry| {
                let path = entry.path().to_string_lossy();
                !excluded_dirs.iter().any(|ex| path.starts_with(ex.as_str()))
            })
            .build_parallel()
            .run(|| {
                let tx = item_tx.clone();
                let root = root.clone();
                let mapping = mapping.clone();
                let char_sep = char_sep.clone();

                Box::new(move |result| {
                    if let Ok(entry) = result
                        && let Some(item) =
                            build_media_item(&root, entry.path(), &mapping, &char_sep)
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

    pub fn start(
        root_path: String,
        mapping: FolderMapping,
        char_sep: String,
        excluded_dirs: Vec<String>,
        ui_tx: Sender<ScanEvent>,
        db_tx: Sender<DbCommand>,
    ) {
        thread::Builder::new()
            .name("nexa-scanner".into())
            .spawn(move || Self::run(root_path, mapping, char_sep, excluded_dirs, ui_tx, db_tx))
            .expect("Failed to spawn scanner thread");
    }
}
