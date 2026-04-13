use crate::core::models::{
    AutocompleteData, FieldFilter, LibraryStats, MediaFilter, MediaItem, SortOrder, StagingItem,
};
use crate::data::migrations::{init_schema_version, run_migrations};
use crate::infra::config::AppConfig;
use crate::utils::{build_search_query, map_media_item, map_staging_item};
use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::Arc;

const SELECT_COLS: &str = "path, name, copyright, artist, media_type, modified, characters, tags";
const SELECT_COLS_FTS: &str =
    "m.path, m.name, m.copyright, m.artist, m.media_type, m.modified, m.characters, m.tags";
const FTS_COLUMN_FILTER: &str = "{copyright artist characters tags}";

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Self {
        let path = AppConfig::get_db_path();
        let mut conn = Connection::open(path).expect("Cannot open SQLite database");
        conn.execute_batch(
            "PRAGMA journal_mode        = WAL;
             PRAGMA synchronous         = NORMAL;
             PRAGMA cache_size          = -65536;
             PRAGMA temp_store          = MEMORY;
             PRAGMA mmap_size           = 268435456;
             PRAGMA wal_autocheckpoint  = 1000;",
        )
        .expect("Failed to configure SQLite pragmas");
        let tx = conn
            .transaction()
            .expect("Cannot begin migration transaction");
        init_schema_version(&tx);
        run_migrations(&tx);
        tx.commit().expect("Cannot commit migrations");
        Self { conn }
    }

    pub fn upsert_batch(&mut self, items: &[Arc<MediaItem>], scan_id: i64) {
        let tx = match self.conn.transaction() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("upsert_batch tx error: {e}");
                return;
            }
        };
        {
            let mut touch = match tx.prepare_cached(
                "UPDATE media SET last_seen_scan = ?3 WHERE path = ?1 AND modified = ?2",
            ) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("touch prepare: {e}");
                    return;
                }
            };

            let mut upsert = match tx.prepare_cached(
                "INSERT INTO media (path, name, copyright, artist, media_type,
                                    modified, last_seen_scan, characters, tags)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,'')
                 ON CONFLICT(path) DO UPDATE SET
                    name           = excluded.name,
                    copyright      = excluded.copyright,
                    artist         = excluded.artist,
                    media_type     = excluded.media_type,
                    modified       = excluded.modified,
                    last_seen_scan = excluded.last_seen_scan,
                    characters     = excluded.characters",
            ) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("upsert prepare: {e}");
                    return;
                }
            };

            for item in items {
                let touched = touch
                    .execute(rusqlite::params![item.path, item.modified, scan_id])
                    .unwrap_or(0);
                if touched == 0 {
                    upsert
                        .execute(rusqlite::params![
                            item.path,
                            item.name,
                            item.copyright,
                            item.artist,
                            item.media_type.as_str(),
                            item.modified,
                            scan_id,
                            item.characters_db(),
                        ])
                        .ok();
                }
            }
        }
        if let Err(e) = tx.commit() {
            eprintln!("upsert_batch commit: {e}");
        }
    }

    pub fn insert_distributed(&self, item: &MediaItem) {
        if let Err(e) = self.conn.execute(
            "INSERT OR REPLACE INTO media
                (path, name, copyright, artist, media_type, modified,
                 last_seen_scan, characters, tags)
             VALUES (?1,?2,?3,?4,?5,?6,
                     (SELECT COALESCE(MAX(last_seen_scan),0) FROM media),
                     ?7,?8)",
            rusqlite::params![
                item.path,
                item.name,
                item.copyright,
                item.artist,
                item.media_type.as_str(),
                item.modified,
                item.characters_db(),
                item.tags_db(),
            ],
        ) {
            eprintln!("insert_distributed error: {e}");
        }
    }

    pub fn update_metadata(
        &self,
        path: &str,
        copyright: &str,
        artist: &str,
        characters: &str,
        tags: &str,
    ) {
        if let Err(e) = self.conn.execute(
            "UPDATE media
                SET copyright  = ?2,
                    artist     = ?3,
                    characters = ?4,
                    tags       = ?5
              WHERE path = ?1",
            rusqlite::params![path, copyright, artist, characters, tags],
        ) {
            eprintln!("update_metadata error: {e}");
        }
    }

    pub fn rename_media_path(&self, old_path: &str, new_path: &str, new_name: &str) {
        if let Err(e) = self.conn.execute(
            "UPDATE media SET path = ?2, name = ?3 WHERE path = ?1",
            rusqlite::params![old_path, new_path, new_name],
        ) {
            eprintln!("rename_media_path error: {e}");
        }
    }

    pub fn query_stats(&self) -> LibraryStats {
        let top_artists: Vec<(String, u32)> = self.conn
            .prepare("SELECT artist, COUNT(*) cnt FROM media WHERE artist != '' GROUP BY artist ORDER BY cnt DESC LIMIT 3")
            .and_then(|mut s| s.query_map([], |r| Ok((r.get::<_,String>(0)?, r.get::<_,u32>(1)?))).map(|it| it.filter_map(Result::ok).collect()))
            .unwrap_or_default();

        let top_copyrights: Vec<(String, u32)> = self.conn
            .prepare("SELECT copyright, COUNT(*) cnt FROM media WHERE copyright != '' GROUP BY copyright ORDER BY cnt DESC LIMIT 3")
            .and_then(|mut s| s.query_map([], |r| Ok((r.get::<_,String>(0)?, r.get::<_,u32>(1)?))).map(|it| it.filter_map(Result::ok).collect()))
            .unwrap_or_default();

        let all_tag_strings: Vec<String> = self
            .conn
            .prepare("SELECT tags FROM media WHERE tags != ''")
            .and_then(|mut s| {
                s.query_map([], |r| r.get::<_, String>(0))
                    .map(|it| it.filter_map(Result::ok).collect())
            })
            .unwrap_or_default();

        let mut tag_counts: HashMap<String, u32> = HashMap::new();
        for ts in all_tag_strings {
            for t in ts.split('|').map(str::trim).filter(|t| !t.is_empty()) {
                *tag_counts.entry(t.to_owned()).or_insert(0) += 1;
            }
        }
        let mut top_tags: Vec<(String, u32)> = tag_counts.into_iter().collect();
        top_tags.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        top_tags.truncate(10);

        LibraryStats {
            top_artists,
            top_copyrights,
            top_tags,
        }
    }

    pub fn query_autocomplete(&self) -> AutocompleteData {
        let artists: Vec<String> = self
            .conn
            .prepare("SELECT DISTINCT artist FROM media WHERE artist != '' ORDER BY artist")
            .and_then(|mut s| {
                s.query_map([], |r| r.get::<_, String>(0))
                    .map(|it| it.filter_map(Result::ok).collect())
            })
            .unwrap_or_default();

        let copyrights: Vec<String> = self
            .conn
            .prepare(
                "SELECT DISTINCT copyright FROM media WHERE copyright != '' ORDER BY copyright",
            )
            .and_then(|mut s| {
                s.query_map([], |r| r.get::<_, String>(0))
                    .map(|it| it.filter_map(Result::ok).collect())
            })
            .unwrap_or_default();

        let raw_chars: Vec<String> = self
            .conn
            .prepare("SELECT characters FROM media WHERE characters != ''")
            .and_then(|mut s| {
                s.query_map([], |r| r.get::<_, String>(0))
                    .map(|it| it.filter_map(Result::ok).collect())
            })
            .unwrap_or_default();
        let mut char_set: std::collections::BTreeSet<String> = Default::default();
        for cs in raw_chars {
            for c in cs.split('|').map(str::trim).filter(|v| !v.is_empty()) {
                char_set.insert(c.to_owned());
            }
        }

        let raw_tags: Vec<String> = self
            .conn
            .prepare("SELECT tags FROM media WHERE tags != ''")
            .and_then(|mut s| {
                s.query_map([], |r| r.get::<_, String>(0))
                    .map(|it| it.filter_map(Result::ok).collect())
            })
            .unwrap_or_default();
        let mut tag_set: std::collections::BTreeSet<String> = Default::default();
        for ts in raw_tags {
            for t in ts.split('|').map(str::trim).filter(|v| !v.is_empty()) {
                tag_set.insert(t.to_owned());
            }
        }

        AutocompleteData {
            artists,
            copyrights,
            characters: char_set.into_iter().collect(),
            tags: tag_set.into_iter().collect(),
        }
    }

    pub fn query(
        &self,
        limit: usize,
        offset: usize,
        filter: &MediaFilter,
        sort: &SortOrder,
        field_filter: &Option<FieldFilter>,
    ) -> Vec<MediaItem> {
        let ff = field_filter
            .as_ref()
            .map(|f| f.to_where_sql())
            .unwrap_or("");
        let sql = format!(
            "SELECT {c} FROM media WHERE 1=1 {f} {ff} {s} LIMIT ? OFFSET ?",
            c = SELECT_COLS,
            f = filter.to_sql(),
            ff = ff,
            s = sort.to_sql(),
        );
        let mut stmt = match self.conn.prepare_cached(&sql) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("query prepare: {e}");
                return Vec::new();
            }
        };
        let (li, oi) = (limit as i64, offset as i64);
        match field_filter {
            None => stmt.query_map(rusqlite::params![li, oi], map_media_item),
            Some(ff) => {
                let v = ff.param_value();
                stmt.query_map(rusqlite::params![v, li, oi], map_media_item)
            }
        }
        .map(|it| it.filter_map(Result::ok).collect())
        .unwrap_or_else(|e| {
            eprintln!("query error: {e}");
            Vec::new()
        })
    }

    pub fn search(
        &self,
        input: &str,
        limit: usize,
        offset: usize,
        filter: &MediaFilter,
        sort: &SortOrder,
        field_filter: &Option<FieldFilter>,
    ) -> Vec<MediaItem> {
        let raw = build_search_query(input);
        if raw.is_empty() {
            return self.query(limit, offset, filter, sort, field_filter);
        }
        let fts_q = format!("{}: {}", FTS_COLUMN_FILTER, raw);
        let ff = field_filter
            .as_ref()
            .map(|f| f.to_where_sql_fts())
            .unwrap_or("");
        let sql = format!(
            "SELECT {c} FROM media m JOIN media_fts ON m.rowid = media_fts.rowid \
             WHERE media_fts MATCH ? {f} {ff} {s} LIMIT ? OFFSET ?",
            c = SELECT_COLS_FTS,
            f = filter.to_sql_fts(),
            ff = ff,
            s = sort.to_sql_fts(),
        );
        let mut stmt = match self.conn.prepare_cached(&sql) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("search prepare: {e}");
                return Vec::new();
            }
        };
        let (li, oi) = (limit as i64, offset as i64);
        match field_filter {
            None => stmt.query_map(rusqlite::params![fts_q, li, oi], map_media_item),
            Some(ff) => {
                let v = ff.param_value();
                stmt.query_map(rusqlite::params![fts_q, v, li, oi], map_media_item)
            }
        }
        .map(|it| it.filter_map(Result::ok).collect())
        .unwrap_or_else(|e| {
            eprintln!("search error: {e}");
            Vec::new()
        })
    }

    pub fn delete_not_seen(&self, scan_id: i64) {
        if let Err(e) = self.conn.execute(
            "DELETE FROM media WHERE last_seen_scan < ?1",
            rusqlite::params![scan_id],
        ) {
            eprintln!("delete_not_seen: {e}");
        }
    }

    pub fn delete_by_path(&self, path: &str) {
        if let Err(e) = self
            .conn
            .execute("DELETE FROM media WHERE path = ?1", rusqlite::params![path])
        {
            eprintln!("delete_by_path: {e}");
        }
    }

    pub fn staging_upsert_batch(&mut self, items: &[Arc<StagingItem>], scan_id: i64) {
        let tx = match self.conn.transaction() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("staging_upsert tx: {e}");
                return;
            }
        };
        {
            let mut touch = match tx.prepare_cached(
                "UPDATE staging SET last_seen_scan = ?3 WHERE path = ?1 AND modified = ?2",
            ) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("staging touch: {e}");
                    return;
                }
            };
            let mut ins = match tx.prepare_cached(
                "INSERT OR IGNORE INTO staging (path, name, media_type, modified, last_seen_scan) VALUES (?1,?2,?3,?4,?5)",
            ) { Ok(s) => s, Err(e) => { eprintln!("staging ins: {e}"); return; } };
            for item in items {
                let t = touch
                    .execute(rusqlite::params![item.path, item.modified, scan_id])
                    .unwrap_or(0);
                if t == 0 {
                    ins.execute(rusqlite::params![
                        item.path,
                        item.name,
                        item.media_type.as_str(),
                        item.modified,
                        scan_id
                    ])
                    .ok();
                }
            }
        }
        if let Err(e) = tx.commit() {
            eprintln!("staging_upsert commit: {e}");
        }
    }

    pub fn staging_delete_not_seen(&self, scan_id: i64) {
        if let Err(e) = self.conn.execute(
            "DELETE FROM staging WHERE last_seen_scan < ?1",
            rusqlite::params![scan_id],
        ) {
            eprintln!("staging_delete_not_seen: {e}");
        }
    }

    pub fn staging_delete_by_path(&self, path: &str) {
        if let Err(e) = self.conn.execute(
            "DELETE FROM staging WHERE path = ?1",
            rusqlite::params![path],
        ) {
            eprintln!("staging_delete_by_path: {e}");
        }
    }

    pub fn staging_query(&self) -> Vec<StagingItem> {
        let mut stmt = match self.conn.prepare_cached(
            "SELECT path, name, media_type, modified FROM staging ORDER BY name ASC",
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("staging_query prepare: {e}");
                return Vec::new();
            }
        };
        stmt.query_map([], map_staging_item)
            .map(|it| it.filter_map(Result::ok).collect())
            .unwrap_or_else(|e| {
                eprintln!("staging_query: {e}");
                Vec::new()
            })
    }
}
