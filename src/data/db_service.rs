use crossbeam_channel::{bounded, Receiver};

use crate::core::models::{DbCommand, MediaFilter, MediaItem, SortOrder};
use crate::data::db_worker::get_db;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

static QUERY_ID: AtomicU64 = AtomicU64::new(0);

fn next_id() -> u64 {
    QUERY_ID.fetch_add(1, Ordering::Relaxed)
}

pub struct DbService;

impl DbService {
    pub fn query(
        limit: usize,
        offset: usize,
        filter: MediaFilter,
        sort: SortOrder,
    ) -> (u64, Receiver<(u64, Vec<Arc<MediaItem>>)>) {
        let (tx, rx) = bounded(1);
        let id = next_id();

        get_db()
            .send(DbCommand::Query {
                id,
                limit,
                offset,
                filter,
                sort,
                resp: tx,
            })
            .ok();

        (id, rx)
    }

    pub fn search(
        query: String,
        limit: usize,
        offset: usize,
        filter: MediaFilter,
        sort: SortOrder,
    ) -> (u64, Receiver<(u64, Vec<Arc<MediaItem>>)>) {
        let (tx, rx) = bounded(1);
        let id = next_id();

        get_db()
            .send(DbCommand::Search {
                id,
                query,
                limit,
                offset,
                filter,
                sort,
                resp: tx,
            })
            .ok();

        (id, rx)
    }

    pub fn update_tags(path: String, tags: Vec<String>) {
        get_db()
            .send(DbCommand::UpdateTags {
                path,
                tags: tags.join("|"),
            })
            .ok();
    }
}
