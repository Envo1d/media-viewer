use crate::core::models::DbCommand;
use crate::data::db::Database;
use crossbeam_channel::Sender;
use std::sync::{Arc, OnceLock};

static DB: OnceLock<Sender<DbCommand>> = OnceLock::new();

fn start_db_worker() -> Sender<DbCommand> {
    let (tx, rx) = crossbeam_channel::bounded::<DbCommand>(2000);

    std::thread::Builder::new()
        .name("nexa-db-worker".into())
        .spawn(move || {
            let mut db = Database::new();

            for cmd in rx {
                if let Err(e) =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match cmd {
                        DbCommand::UpsertBatch(items, scan_id) => {
                            db.upsert_batch(&items, scan_id);
                        }
                        DbCommand::DeleteNotSeen(scan_id) => {
                            db.delete_not_seen(scan_id);
                        }
                        DbCommand::DeleteByPath(path) => {
                            db.delete_by_path(&path);
                        }
                        DbCommand::UpdateTags { path, tags } => {
                            db.update_tags(&path, &tags);
                        }
                        DbCommand::UpdateCharacters { path, characters } => {
                            db.update_characters(&path, &characters);
                        }
                        DbCommand::QueryStats { resp } => {
                            let stats = db.query_stats();
                            let _ = resp.send(stats);
                        }
                        DbCommand::Query {
                            id,
                            limit,
                            offset,
                            filter,
                            sort,
                            field_filter,
                            resp,
                        } => {
                            let items = db.query(limit, offset, &filter, &sort, &field_filter);
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
                            field_filter,
                            resp,
                        } => {
                            let items =
                                db.search(&query, limit, offset, &filter, &sort, &field_filter);
                            let arced: Vec<Arc<_>> = items.into_iter().map(Arc::new).collect();
                            let _ = resp.send((id, arced));
                        }
                    }))
                {
                    eprintln!("[db-worker] panic: {:?}", e);
                }
            }
        })
        .expect("Failed to spawn DB worker thread");

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
