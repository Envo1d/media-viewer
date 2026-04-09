use crate::core::models::{MediaFilter, MediaItem, SortOrder};
use crate::data::migrations::{init_schema_version, run_migrations};
use crate::infra::config::AppConfig;
use crate::utils::{build_search_query, map_media_item};
use rusqlite::Connection;
use std::sync::Arc;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Self {
        let path = AppConfig::get_db_path();
        let mut conn = Connection::open(path).expect("Cannot open SQLite database");

        conn.execute_batch(
            "PRAGMA journal_mode    = WAL;
             PRAGMA synchronous     = NORMAL;
             PRAGMA cache_size      = -65536;
             PRAGMA temp_store      = MEMORY;
             PRAGMA mmap_size       = 268435456;
             PRAGMA wal_autocheckpoint = 1000;",
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
                  WHERE path = ?1
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
                    (path, name, category, author, media_type, modified, last_seen_scan)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(path) DO UPDATE SET
                    name           = excluded.name,
                    category       = excluded.category,
                    author         = excluded.author,
                    media_type     = excluded.media_type,
                    modified       = excluded.modified,
                    last_seen_scan = excluded.last_seen_scan",
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
                            item.category,
                            item.author,
                            item.media_type.as_str(),
                            item.modified,
                            scan_id,
                        ])
                        .ok();
                }
            }
        }

        if let Err(e) = tx.commit() {
            eprintln!("upsert_batch commit error: {e}");
        }
    }

    pub fn query(
        &self,
        limit: usize,
        offset: usize,
        filter: &MediaFilter,
        sort: &SortOrder,
    ) -> Vec<MediaItem> {
        let sql = format!(
            "SELECT path, name, category, author, media_type, modified
               FROM media
              WHERE 1=1 {filter}
              {sort}
              LIMIT ?1 OFFSET ?2",
            filter = filter.to_sql(),
            sort = sort.to_sql(),
        );

        let mut stmt = match self.conn.prepare(&sql) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("query prepare error: {e}");
                return Vec::new();
            }
        };

        stmt.query_map(
            rusqlite::params![limit as i64, offset as i64],
            map_media_item,
        )
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
    ) -> Vec<MediaItem> {
        let fts_query = build_search_query(input);
        if fts_query.is_empty() {
            return self.query(limit, offset, filter, sort);
        }

        let sql = format!(
            "SELECT m.path, m.name, m.category, m.author, m.media_type, m.modified
               FROM media m
               JOIN media_fts ON m.rowid = media_fts.rowid
              WHERE media_fts MATCH ?1 {filter}
              {sort}
              LIMIT ?2 OFFSET ?3",
            filter = filter.to_sql_fts(),
            sort = sort.to_sql_fts(),
        );

        let mut stmt = match self.conn.prepare(&sql) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("search prepare error: {e}");
                return Vec::new();
            }
        };

        stmt.query_map(
            rusqlite::params![fts_query, limit as i64, offset as i64],
            map_media_item,
        )
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
