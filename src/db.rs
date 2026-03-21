use crate::db_migrations::{init_schema_version, run_migrations};
use crate::models::{MediaItem, MediaType};
use crate::utils::functions::build_search_query;
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

    pub fn query(
        &self,
        category: &str,
        author: &str,
        limit: usize,
        offset: usize,
    ) -> Vec<MediaItem> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT path, name, category, author, media_type, modified
         FROM media
         WHERE category LIKE ?1 AND author LIKE ?2
         ORDER BY name
         LIMIT ?3 OFFSET ?4",
            )
            .unwrap();

        let category_pattern = format!("%{}%", category.to_lowercase());
        let author_pattern = format!("%{}%", author.to_lowercase());

        let rows = stmt
            .query_map(
                rusqlite::params![
                    category_pattern,
                    author_pattern,
                    limit as i64,
                    offset as i64
                ],
                |row| {
                    let media_type_str: String = row.get(4)?;

                    let media_type = match media_type_str.as_str() {
                        "Image" => MediaType::Image,
                        "Video" => MediaType::Video,
                        _ => MediaType::Image,
                    };

                    Ok(MediaItem {
                        path: row.get(0)?,
                        name: row.get(1)?,
                        category: row.get(2)?,
                        author: row.get(3)?,
                        media_type,
                        modified: row.get(5)?,
                    })
                },
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
                |row| {
                    let media_type_str: String = row.get(4)?;

                    let media_type = match media_type_str.as_str() {
                        "Image" => MediaType::Image,
                        "Video" => MediaType::Video,
                        _ => MediaType::Image,
                    };

                    Ok(MediaItem {
                        path: row.get(0)?,
                        name: row.get(1)?,
                        category: row.get(2)?,
                        author: row.get(3)?,
                        media_type,
                        modified: row.get(5)?,
                    })
                },
            )
            .unwrap();

        rows.filter_map(Result::ok).collect()
    }

    pub fn count(&self, category: &str, author: &str) -> usize {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT COUNT(*) FROM media
         WHERE category LIKE ?1 AND author LIKE ?2",
            )
            .unwrap();

        let category_pattern = format!("%{}%", category.to_lowercase());
        let author_pattern = format!("%{}%", author.to_lowercase());

        let count: i64 = stmt
            .query_row(rusqlite::params![category_pattern, author_pattern], |row| {
                row.get(0)
            })
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

    pub fn delete_missing(&self, existing_paths: &[String]) {
        let existing_set: HashSet<&String> = existing_paths.iter().collect();

        let mut stmt = self.conn.prepare("SELECT path FROM media").unwrap();
        let db_paths: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        let mut delete_media = self
            .conn
            .prepare("DELETE FROM media WHERE path = ?1")
            .unwrap();

        for path in db_paths {
            if !existing_set.contains(&path) {
                delete_media.execute(rusqlite::params![path]).unwrap();
            }
        }
    }
}
