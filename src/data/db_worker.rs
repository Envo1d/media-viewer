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
                        DbCommand::UpsertBatch(items, scan_id) => db.upsert_batch(&items, scan_id),
                        DbCommand::DeleteNotSeen(scan_id) => db.delete_not_seen(scan_id),
                        DbCommand::DeleteByPath(path) => db.delete_by_path(&path),

                        DbCommand::UpdateMetadata {
                            path,
                            copyright,
                            artist,
                            characters,
                            tags,
                        } => {
                            db.update_metadata(&path, &copyright, &artist, &characters, &tags);
                        }

                        DbCommand::RenameMediaPath {
                            old_path,
                            new_path,
                            new_name,
                        } => {
                            db.rename_media_path(&old_path, &new_path, &new_name);
                        }

                        DbCommand::QueryStatsForValues {
                            copyrights,
                            artists,
                            tags,
                            resp,
                        } => {
                            let _ =
                                resp.send(db.query_stats_for_values(&copyrights, &artists, &tags));
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

                        DbCommand::InsertDistributed { item } => db.insert_distributed(&item),

                        DbCommand::StagingUpsertBatch(items, scan_id) => {
                            db.staging_upsert_batch(&items, scan_id)
                        }
                        DbCommand::StagingDeleteNotSeen(scan_id) => {
                            db.staging_delete_not_seen(scan_id)
                        }
                        DbCommand::StagingDeleteByPath(path) => db.staging_delete_by_path(&path),
                        DbCommand::StagingQuery { resp } => {
                            let items = db.staging_query();
                            let arced: Vec<Arc<_>> = items.into_iter().map(Arc::new).collect();
                            let _ = resp.send(arced);
                        }

                        DbCommand::QueryAutocomplete { resp } => {
                            let _ = resp.send(db.query_autocomplete());
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
