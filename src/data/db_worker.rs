use crate::core::models::DbCommand;
use crate::data::db::Database;
use crossbeam_channel::Sender;
use std::sync::{Arc, OnceLock};

static DB: OnceLock<Sender<DbCommand>> = OnceLock::new();

fn start_db_worker() -> Sender<DbCommand> {
    let (tx, rx) = crossbeam_channel::bounded::<DbCommand>(100);

    std::thread::spawn(move || {
        let mut db = Database::new();

        for cmd in rx {
            if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match cmd {
                DbCommand::UpsertBatch(items, scan_id) => {
                    db.upsert_batch(&items, scan_id);
                }
                DbCommand::DeleteNotSeen(scan_id) => {
                    db.delete_not_seen(scan_id);
                }
                DbCommand::Query {
                    id,
                    limit,
                    offset,
                    filter,
                    sort,
                    resp,
                } => {
                    let items = db.query(limit, offset, &filter, &sort);
                    let arced: Vec<Arc<_>> = items.into_iter().map(Arc::new).collect();
                    let _ = resp.send((id, arced));
                }
                DbCommand::Search {
                    id,
                    query,
                    limit,
                    offset,
                    filter,
                    sort,
                    resp,
                } => {
                    let items = db.search(&query, limit, offset, &filter, &sort);
                    let arced: Vec<Arc<_>> = items.into_iter().map(Arc::new).collect();
                    let _ = resp.send((id, arced));
                }
            })) {
                eprintln!("DB worker panic: {:?}", e);
                // The resp sender was dropped by the panic, so poll_db on the UI thread
                // will see TryRecvError::Disconnected and correctly reset is_loading_more.
            }
        }
    });

    tx
}

pub fn init_db() -> Sender<DbCommand> {
    let tx = start_db_worker();
    DB.set(tx.clone()).expect("DB already initialized");
    tx
}

pub fn get_db() -> &'static Sender<DbCommand> {
    DB.get().expect("DB not initialized")
}
