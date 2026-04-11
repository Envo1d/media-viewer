use crate::core::models::{FieldFilter, LibraryStats, MediaFilter, MediaItem, SortOrder};
use crate::data::migrations::{init_schema_version, run_migrations};
use crate::infra::config::AppConfig;
use crate::utils::{build_search_query, map_media_item};
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
                "UPDATE media
                    SET last_seen_scan = ?3
                  WHERE path     = ?1
                    AND modified = ?2",
            ) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("touch prepare error: {e}");
                    return;
                }
            };

            let mut upsert = match tx.prepare_cached(
                "INSERT INTO media
                    (path, name, copyright, artist, media_type,
                     modified, last_seen_scan, characters, tags)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, '')
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
                    eprintln!("upsert prepare error: {e}");
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
            eprintln!("upsert_batch commit error: {e}");
        }
    }

    pub fn update_tags(&self, path: &str, tags: &str) {
        if let Err(e) = self.conn.execute(
            "UPDATE media SET tags = ?2 WHERE path = ?1",
            rusqlite::params![path, tags],
        ) {
            eprintln!("update_tags error: {e}");
        }
    }

    pub fn update_characters(&self, path: &str, characters: &str) {
        if let Err(e) = self.conn.execute(
            "UPDATE media SET characters = ?2 WHERE path = ?1",
            rusqlite::params![path, characters],
        ) {
            eprintln!("update_characters error: {e}");
        }
    }

    pub fn query_stats(&self) -> LibraryStats {
        let top_artists: Vec<(String, u32)> = self
            .conn
            .prepare(
                "SELECT artist, COUNT(*) as cnt
                   FROM media
                  WHERE artist != ''
                  GROUP BY artist
                  ORDER BY cnt DESC
                  LIMIT 3",
            )
            .and_then(|mut s| {
                s.query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
                })
                .map(|rows| rows.filter_map(Result::ok).collect())
            })
            .unwrap_or_default();

        let top_copyrights: Vec<(String, u32)> = self
            .conn
            .prepare(
                "SELECT copyright, COUNT(*) as cnt
                   FROM media
                  WHERE copyright != ''
                  GROUP BY copyright
                  ORDER BY cnt DESC
                  LIMIT 3",
            )
            .and_then(|mut s| {
                s.query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
                })
                .map(|rows| rows.filter_map(Result::ok).collect())
            })
            .unwrap_or_default();

        let all_tag_strings: Vec<String> = self
            .conn
            .prepare("SELECT tags FROM media WHERE tags != ''")
            .and_then(|mut s| {
                s.query_map([], |row| row.get::<_, String>(0))
                    .map(|rows| rows.filter_map(Result::ok).collect())
            })
            .unwrap_or_default();

        let mut tag_counts: HashMap<String, u32> = HashMap::new();
        for tags_str in all_tag_strings {
            for tag in tags_str.split('|').map(str::trim).filter(|t| !t.is_empty()) {
                *tag_counts.entry(tag.to_owned()).or_insert(0) += 1;
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

    pub fn query(
        &self,
        limit: usize,
        offset: usize,
        filter: &MediaFilter,
        sort: &SortOrder,
        field_filter: &Option<FieldFilter>,
    ) -> Vec<MediaItem> {
        let ff_sql = field_filter
            .as_ref()
            .map(|f| f.to_where_sql())
            .unwrap_or("");

        let sql = format!(
            "SELECT {cols}
               FROM media
              WHERE 1=1 {filter} {ff_sql}
              {sort}
              LIMIT ? OFFSET ?",
            cols = SELECT_COLS,
            filter = filter.to_sql(),
            ff_sql = ff_sql,
            sort = sort.to_sql(),
        );

        let mut stmt = match self.conn.prepare_cached(&sql) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("query prepare error: {e}");
                return Vec::new();
            }
        };

        let limit_i = limit as i64;
        let offset_i = offset as i64;

        let result = match field_filter {
            None => stmt.query_map(rusqlite::params![limit_i, offset_i], map_media_item),
            Some(ff) => {
                let val = ff.param_value();
                stmt.query_map(rusqlite::params![val, limit_i, offset_i], map_media_item)
            }
        };

        result
            .map(|rows| rows.filter_map(Result::ok).collect())
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
        let raw_fts = build_search_query(input);
        if raw_fts.is_empty() {
            return self.query(limit, offset, filter, sort, field_filter);
        }

        let fts_query = format!("{}: {}", FTS_COLUMN_FILTER, raw_fts);

        let ff_sql = field_filter
            .as_ref()
            .map(|f| f.to_where_sql_fts())
            .unwrap_or("");

        let sql = format!(
            "SELECT {cols}
               FROM media m
               JOIN media_fts ON m.rowid = media_fts.rowid
              WHERE media_fts MATCH ? {filter} {ff_sql}
              {sort}
              LIMIT ? OFFSET ?",
            cols = SELECT_COLS_FTS,
            filter = filter.to_sql_fts(),
            ff_sql = ff_sql,
            sort = sort.to_sql_fts(),
        );

        let mut stmt = match self.conn.prepare_cached(&sql) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("search prepare error: {e}");
                return Vec::new();
            }
        };

        let limit_i = limit as i64;
        let offset_i = offset as i64;

        let result = match field_filter {
            None => stmt.query_map(
                rusqlite::params![fts_query, limit_i, offset_i],
                map_media_item,
            ),
            Some(ff) => {
                let val = ff.param_value();
                stmt.query_map(
                    rusqlite::params![fts_query, val, limit_i, offset_i],
                    map_media_item,
                )
            }
        };

        result
            .map(|rows| rows.filter_map(Result::ok).collect())
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
            eprintln!("delete_not_seen error: {e}");
        }
    }

    pub fn delete_by_path(&self, path: &str) {
        if let Err(e) = self
            .conn
            .execute("DELETE FROM media WHERE path = ?1", rusqlite::params![path])
        {
            eprintln!("delete_by_path error: {e}");
        }
    }
}
