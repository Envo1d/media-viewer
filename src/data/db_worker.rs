use crate::core::models::DbCommand;
use crate::data::db::Database;
use crossbeam_channel::Sender;
use std::sync::{Arc, OnceLock};

static WRITE_DB: OnceLock<Sender<DbCommand>> = OnceLock::new();
static READ_DB: OnceLock<Sender<DbCommand>> = OnceLock::new();

fn start_write_worker() -> (Sender<DbCommand>, crossbeam_channel::Receiver<()>) {
    let (tx, rx) = crossbeam_channel::bounded::<DbCommand>(2000);
    let (ready_tx, ready_rx) = crossbeam_channel::bounded::<()>(1);

    std::thread::Builder::new()
        .name("nexa-db-write".into())
        .spawn(move || {
            let mut db = Database::new();

            let _ = ready_tx.send(());

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

                        DbCommand::RenameGroupBatch(renames) => {
                            db.rename_group_batch(&renames);
                        }

                        DbCommand::InsertDistributed { item } => db.insert_distributed(&item),

                        DbCommand::StagingUpsertBatch(items, scan_id) => {
                            db.staging_upsert_batch(&items, scan_id)
                        }
                        DbCommand::StagingDeleteNotSeen(scan_id) => {
                            db.staging_delete_not_seen(scan_id)
                        }
                        DbCommand::StagingDeleteByPath(path) => db.staging_delete_by_path(&path),

                        other => {
                            eprintln!(
                                "[db-write-worker] received a read command — \
                                 route it to get_read_db() instead: {:?}",
                                std::mem::discriminant(&other)
                            );
                        }
                    }))
                {
                    eprintln!("[db-write-worker] panic: {:?}", e);
                }
            }
        })
        .expect("Failed to spawn DB write worker thread");

    (tx, ready_rx)
}

fn start_read_worker(write_ready_rx: crossbeam_channel::Receiver<()>) -> Sender<DbCommand> {
    let (tx, rx) = crossbeam_channel::bounded::<DbCommand>(256);

    std::thread::Builder::new()
        .name("nexa-db-read".into())
        .spawn(move || {
            let _ = write_ready_rx.recv();

            let db = Database::new();
            for cmd in rx {
                if let Err(e) =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match cmd {
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

                        DbCommand::QueryStatsForValues {
                            copyrights,
                            artists,
                            tags,
                            resp,
                        } => {
                            let _ =
                                resp.send(db.query_stats_for_values(&copyrights, &artists, &tags));
                        }

                        DbCommand::QueryAutocomplete { resp } => {
                            let _ = resp.send(db.query_autocomplete());
                        }

                        DbCommand::StagingQuery { resp } => {
                            let items = db.staging_query();
                            let arced: Vec<Arc<_>> = items.into_iter().map(Arc::new).collect();
                            let _ = resp.send(arced);
                        }

                        DbCommand::QueryGroup {
                            base_stem,
                            dir,
                            resp,
                        } => {
                            let items = db.query_group(&base_stem, &dir);
                            let arced: Vec<Arc<_>> = items.into_iter().map(Arc::new).collect();
                            let _ = resp.send(arced);
                        }

                        other => {
                            eprintln!(
                                "[db-read-worker] received a write command — \
                                 route it to get_db() instead: {:?}",
                                std::mem::discriminant(&other)
                            );
                        }
                    }))
                {
                    eprintln!("[db-read-worker] panic: {:?}", e);
                }
            }
        })
        .expect("Failed to spawn DB read worker thread");

    tx
}

pub fn init_db() {
    let (write_tx, ready_rx) = start_write_worker();
    let read_tx = start_read_worker(ready_rx);
    WRITE_DB
        .set(write_tx)
        .expect("DB write worker already initialised");
    READ_DB
        .set(read_tx)
        .expect("DB read worker already initialised");
}

pub fn get_db() -> &'static Sender<DbCommand> {
    WRITE_DB
        .get()
        .expect("DB not initialised — call init_db() first")
}

pub fn get_read_db() -> &'static Sender<DbCommand> {
    READ_DB
        .get()
        .expect("DB not initialised — call init_db() first")
}
