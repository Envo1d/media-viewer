use crate::core::models::{DbCommand, MediaItem, ScanEvent};
use crate::utils::{build_media_item, current_timestamp};
use crossbeam_channel::{unbounded, Sender};
use ignore::WalkBuilder;
use std::sync::Arc;
use std::thread;

const BATCH_SIZE: usize = 500;

pub struct MediaScanner;

impl MediaScanner {
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

        WalkBuilder::new(&*root)
            .hidden(false)
            .git_ignore(false)
            .threads(num_cpus::get())
            .build_parallel()
            .run(|| {
                let tx = item_tx.clone();
                let root = root.clone();

                Box::new(move |result| {
                    if let Ok(entry) = result {
                        if let Some(item) = build_media_item(&root, entry.path()) {
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
        thread::spawn(move || Self::run(root_path, ui_tx, db_tx));
    }
}
