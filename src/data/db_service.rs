use crate::core::models::{
    AutocompleteData, DbCommand, FieldFilter, LibraryStats, MediaFilter, MediaItem, SortOrder,
    StagingItem,
};
use crate::data::db_worker::{get_db, get_read_db};
use crossbeam_channel::Receiver;
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

        get_read_db()
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

        get_read_db()
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

    pub fn query_stats_for_values(
        copyrights: Vec<String>,
        artists: Vec<String>,
        tags: Vec<String>,
    ) -> Receiver<LibraryStats> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        get_read_db()
            .send(DbCommand::QueryStatsForValues {
                copyrights,
                artists,
                tags,
                resp: tx,
            })
            .ok();
        rx
    }

    pub fn query_autocomplete() -> Receiver<AutocompleteData> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        get_read_db()
            .send(DbCommand::QueryAutocomplete { resp: tx })
            .ok();
        rx
    }

    pub fn staging_query() -> Receiver<Vec<Arc<StagingItem>>> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        get_read_db()
            .send(DbCommand::StagingQuery { resp: tx })
            .ok();
        rx
    }

    pub fn update_metadata(
        path: String,
        copyright: String,
        artist: String,
        characters: Vec<String>,
        tags: Vec<String>,
    ) {
        get_db()
            .send(DbCommand::UpdateMetadata {
                path,
                copyright,
                artist,
                characters: characters.join("|"),
                tags: tags.join("|"),
            })
            .ok();
    }

    pub fn insert_distributed(item: Arc<MediaItem>) {
        get_db().send(DbCommand::InsertDistributed { item }).ok();
    }

    pub fn rename_media_path(old_path: String, new_path: String, new_name: String) {
        get_db()
            .send(DbCommand::RenameMediaPath {
                old_path,
                new_path,
                new_name,
            })
            .ok();
    }

    pub fn delete_by_path(path: String) {
        get_db().send(DbCommand::DeleteByPath(path)).ok();
    }

    pub fn rename_group_batch(renames: Vec<(String, String, String, String)>) {
        get_db().send(DbCommand::RenameGroupBatch(renames)).ok();
    }

    pub fn staging_delete_by_path(path: String) {
        get_db().send(DbCommand::StagingDeleteByPath(path)).ok();
    }

    pub fn query_group(
        base_stem: String,
        ext: String,
        dir: String,
    ) -> Receiver<Vec<Arc<MediaItem>>> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        get_read_db()
            .send(DbCommand::QueryGroup {
                base_stem,
                ext,
                dir,
                resp: tx,
            })
            .ok();
        rx
    }
}
