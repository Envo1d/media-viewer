use crate::core::models::DbCommand;
use crate::data::db::Database;
use crossbeam_channel::Sender;

pub fn start_db_worker() -> Sender<DbCommand> {
    let (tx, rx) = crossbeam_channel::unbounded::<DbCommand>();

    std::thread::spawn(move || {
        let mut db = Database::new();

        for cmd in rx {
            match cmd {
                DbCommand::UpsertBatch(items, scan_id) => {
                    db.upsert_batch(&items, scan_id);
                }

                DbCommand::DeleteNotSeen(scan_id) => {
                    db.delete_not_seen(scan_id);
                }

                DbCommand::Query {
                    limit,
                    offset,
                    resp,
                } => {
                    let result = db.query(limit, offset);
                    let _ = resp.send(result);
                }

                DbCommand::Search {
                    query,
                    limit,
                    offset,
                    resp,
                } => {
                    let result = db.search(&query, limit, offset);
                    let _ = resp.send(result);
                }
            }
        }
    });

    tx
}
