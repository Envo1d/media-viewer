use crate::core::models::MediaItem;
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

        let mut conn = Connection::open(path).unwrap();

        let tx = conn.transaction().unwrap();

        init_schema_version(&tx);
        run_migrations(&tx);

        tx.commit().unwrap();

        Self { conn }
    }

    pub fn upsert_batch(&mut self, items: &[Arc<MediaItem>], scan_id: i64) {
        let tx = self.conn.transaction().unwrap();

        {
            let mut stmt = tx.prepare(
                "INSERT INTO media (path, name, category, author, media_type, modified, last_seen_scan)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(path) DO UPDATE SET
                name=excluded.name,
                category=excluded.category,
                author=excluded.author,
                media_type=excluded.media_type,
                modified=excluded.modified,
                last_seen_scan=?7"
            ).unwrap();

            for item in items {
                let item = item.as_ref();

                stmt.execute(rusqlite::params![
                    item.path,
                    item.name,
                    item.category,
                    item.author,
                    format!("{:?}", item.media_type),
                    item.modified,
                    scan_id
                ])
                .ok();
            }
        }

        tx.commit().unwrap();
    }

    pub fn query(&self, limit: usize, offset: usize) -> Vec<MediaItem> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT path, name, category, author, media_type, modified
         FROM media
         ORDER BY name
         LIMIT ?1 OFFSET ?2",
            )
            .unwrap();

        let rows = stmt
            .query_map(
                rusqlite::params![limit as i64, offset as i64],
                map_media_item,
            )
            .unwrap();

        rows.filter_map(Result::ok).collect()
    }

    pub fn search(&self, input: &str, limit: usize, offset: usize) -> Vec<MediaItem> {
        let query = build_search_query(input);

        let mut stmt = self
            .conn
            .prepare(
                "SELECT m.path, m.name, m.category, m.author, m.media_type, m.modified
             FROM media m
             JOIN media_fts ON m.rowid = media_fts.rowid
             WHERE media_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2 OFFSET ?3",
            )
            .unwrap();

        let rows = stmt
            .query_map(
                rusqlite::params![query, limit as i64, offset as i64],
                map_media_item,
            )
            .unwrap();

        rows.filter_map(Result::ok).collect()
    }

    pub fn delete_not_seen(&self, scan_id: i64) {
        self.conn
            .execute(
                "DELETE FROM media WHERE last_seen_scan != ?1",
                rusqlite::params![scan_id],
            )
            .ok();
    }
}
