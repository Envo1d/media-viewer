use crate::core::models::DbCommand;
use crate::data::db::Database;
use crossbeam_channel::Sender;

pub fn start_db_worker() -> Sender<DbCommand> {
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
                    resp,
                } => {
                    let result = db.query(limit, offset);
                    let _ = resp.send((id, result));
                }

                DbCommand::Search {
                    id,
                    query,
                    limit,
                    offset,
                    resp,
                } => {
                    let result = db.search(&query, limit, offset);
                    let _ = resp.send((id, result));
                }
            })) {
                eprintln!("DB worker panic: {:?}", e);
            }
        }
    });

    tx
}
