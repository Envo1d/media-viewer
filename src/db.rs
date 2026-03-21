use crate::db_migrations::{init_schema_version, run_migrations};
use crate::models::MediaItem;
use crate::utils::functions::{build_search_query, map_media_item};
use rusqlite::{params, Connection};
use std::collections::HashSet;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: &str) -> Self {
        let mut conn = Connection::open(path).unwrap();

        let tx = conn.transaction().unwrap();

        init_schema_version(&tx);
        run_migrations(&tx);

        tx.commit().unwrap();

        Self { conn }
    }

    pub fn upsert(&self, item: &MediaItem) {
        self.conn
            .execute(
                "INSERT INTO media (path, name, category, author, media_type, modified)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(path) DO UPDATE SET
                name=excluded.name,
                category=excluded.category,
                author=excluded.author,
                media_type=excluded.media_type,
                modified=excluded.modified",
                params![
                    item.path,
                    item.name,
                    item.category,
                    item.author,
                    format!("{:?}", item.media_type),
                    item.modified
                ],
            )
            .unwrap();
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

    pub fn count(&self) -> usize {
        let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM media").unwrap();

        let count: i64 = stmt
            .query_row(rusqlite::params![], |row| row.get(0))
            .unwrap();

        count as usize
    }

    pub fn search_count(&self, input: &str) -> usize {
        let query = build_search_query(input);

        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*)
         FROM media_fts
         WHERE media_fts MATCH ?1",
                rusqlite::params![query],
                |row| row.get(0),
            )
            .unwrap();

        count as usize
    }

    pub fn delete_missing(&mut self, existing_paths: &[String]) {
        let existing_set: HashSet<&String> = existing_paths.iter().collect();

        let paths_to_delete: Vec<String> = {
            let mut stmt = self.conn.prepare("SELECT path FROM media").unwrap();
            let iter = stmt.query_map([], |row| row.get::<_, String>(0)).unwrap();

            iter.filter_map(Result::ok)
                .filter(|path| !existing_set.contains(path))
                .collect()
        };

        if paths_to_delete.is_empty() {
            return;
        }

        let tx = self.conn.transaction().unwrap();
        {
            let mut del_stmt = tx.prepare("DELETE FROM media WHERE path = ?1").unwrap();
            for path in paths_to_delete {
                del_stmt.execute(params![path]).ok();
            }
        }
        tx.commit().unwrap();
    }
}
