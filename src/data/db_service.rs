use crossbeam_channel::Receiver;

use crate::core::models::{
    DbCommand, FieldFilter, LibraryStats, MediaFilter, MediaItem, SortOrder,
};
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
        field_filter: Option<FieldFilter>,
    ) -> (u64, Receiver<(u64, Vec<Arc<MediaItem>>)>) {
        let (tx, rx) = crossbeam_channel::bounded(1);
        let id = next_id();

        get_db()
            .send(DbCommand::Query {
                id,
                limit,
                offset,
                filter,
                sort,
                field_filter,
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
        field_filter: Option<FieldFilter>,
    ) -> (u64, Receiver<(u64, Vec<Arc<MediaItem>>)>) {
        let (tx, rx) = crossbeam_channel::bounded(1);
        let id = next_id();

        get_db()
            .send(DbCommand::Search {
                id,
                query,
                limit,
                offset,
                filter,
                sort,
                field_filter,
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

    pub fn update_characters(path: String, characters: Vec<String>) {
        get_db()
            .send(DbCommand::UpdateCharacters {
                path,
                characters: characters.join("|"),
            })
            .ok();
    }

    pub fn query_stats() -> Receiver<LibraryStats> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        get_db().send(DbCommand::QueryStats { resp: tx }).ok();
        rx
    }
}
