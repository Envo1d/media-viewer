use crossbeam_channel::{bounded, Receiver};

use crate::core::models::{DbCommand, MediaItem};
use crate::data::db_worker::get_db;
use std::sync::Arc;

static mut QUERY_ID: u64 = 0;

fn next_id() -> u64 {
    unsafe {
        QUERY_ID += 1;
        QUERY_ID
    }
}

pub struct DbService;

impl DbService {
    pub fn query(limit: usize, offset: usize) -> (u64, Receiver<(u64, Vec<Arc<MediaItem>>)>) {
        let (tx, rx) = bounded(1);

        let id = next_id();

        get_db()
            .send(DbCommand::Query {
                id,
                limit,
                offset,
                resp: tx,
            })
            .ok();

        (id, rx)
    }

    pub fn search(
        query: String,
        limit: usize,
        offset: usize,
    ) -> (u64, Receiver<(u64, Vec<Arc<MediaItem>>)>) {
        let (tx, rx) = bounded(1);

        let id = next_id();

        get_db()
            .send(DbCommand::Search {
                id,
                query,
                limit,
                offset,
                resp: tx,
            })
            .ok();

        (id, rx)
    }
}
