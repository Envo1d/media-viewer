use crate::core::models::{
    AutocompleteData, FieldFilter, LibraryStats, MediaFilter, MediaItem, SortOrder, StagingItem,
};
use crate::data::migrations::{init_schema_version, run_migrations};
use crate::infra::config::AppConfig;
use crate::utils::file_helpers::natural_cmp;
use crate::utils::{build_search_query, map_media_item, map_staging_item};
use rusqlite::Connection;
use std::sync::Arc;

const SELECT_COLS: &str = "path, name, copyright, artist, media_type, modified, characters, tags";
const SELECT_COLS_FTS: &str =
    "m.path, m.name, m.copyright, m.artist, m.media_type, m.modified, m.characters, m.tags";
const FTS_COLUMN_FILTER: &str = "{copyright artist characters tags}";

fn split_cte(col: &str) -> String {
    format!(
        "WITH RECURSIVE split(word, rest) AS (
            SELECT
                CASE WHEN instr({col},'|') > 0
                     THEN substr({col}, 1, instr({col},'|') - 1)
                     ELSE {col} END,
                CASE WHEN instr({col},'|') > 0
                     THEN substr({col}, instr({col},'|') + 1)
                     ELSE '' END
            FROM media WHERE {col} != ''
            UNION ALL
            SELECT
                CASE WHEN instr(rest,'|') > 0
                     THEN substr(rest, 1, instr(rest,'|') - 1)
                     ELSE rest END,
                CASE WHEN instr(rest,'|') > 0
                     THEN substr(rest, instr(rest,'|') + 1)
                     ELSE '' END
            FROM split WHERE rest != ''
        )",
        col = col
    )
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Self {
        let path = AppConfig::get_db_path();
        let mut conn = Connection::open(path).expect("Cannot open SQLite database");

        conn.busy_timeout(std::time::Duration::from_secs(30))
            .expect("Failed to set SQLite busy timeout");

        conn.execute_batch(
            "PRAGMA journal_mode        = WAL;
             PRAGMA synchronous         = NORMAL;
             PRAGMA cache_size          = -65536;
             PRAGMA temp_store          = MEMORY;
             PRAGMA mmap_size           = 268435456;
             PRAGMA wal_autocheckpoint  = 1000;",
        )
        .expect("Failed to configure SQLite pragmas");

        conn.create_collation("NATURALSORT", natural_cmp)
            .expect("Failed to register NATURALSORT collation");

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
                    last_seen_scan = excluded.last_seen_scan",
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

    pub fn rename_group_batch(&mut self, renames: &[(String, String, String, String)]) {
        if renames.is_empty() {
            return;
        }
        let tx = match self.conn.transaction() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("rename_group_batch begin tx: {e}");
                return;
            }
        };

        for (orig_path, temp_path, _final_path, _final_name) in renames {
            let temp_name = std::path::Path::new(temp_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            if let Err(e) = tx.execute(
                "UPDATE media SET path = ?2, name = ?3 WHERE path = ?1",
                rusqlite::params![orig_path, temp_path, temp_name],
            ) {
                eprintln!("rename_group_batch phase-A error ({orig_path} → {temp_path}): {e}");
            }
        }

        for (_orig_path, temp_path, final_path, final_name) in renames {
            if let Err(e) = tx.execute(
                "UPDATE media SET path = ?2, name = ?3 WHERE path = ?1",
                rusqlite::params![temp_path, final_path, final_name],
            ) {
                eprintln!("rename_group_batch phase-B error ({temp_path} → {final_path}): {e}");
            }
        }

        if let Err(e) = tx.commit() {
            eprintln!("rename_group_batch commit: {e}");
        }
    }

    pub fn query_stats_for_values(
        &self,
        copyrights: &[String],
        artists: &[String],
        tags: &[String],
    ) -> LibraryStats {
        fn placeholders(n: usize) -> String {
            (0..n).map(|_| "?").collect::<Vec<_>>().join(",")
        }

        let top_copyrights: Vec<(String, u32)> = if copyrights.is_empty() {
            Vec::new()
        } else {
            let sql = format!(
                "SELECT copyright, COUNT(*) FROM media \
                 WHERE copyright IN ({}) GROUP BY copyright",
                placeholders(copyrights.len())
            );
            self.conn
                .prepare(&sql)
                .and_then(|mut s| {
                    s.query_map(rusqlite::params_from_iter(copyrights.iter()), |r| {
                        Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?))
                    })
                    .map(|it| it.filter_map(Result::ok).collect())
                })
                .unwrap_or_default()
        };

        let top_artists: Vec<(String, u32)> = if artists.is_empty() {
            Vec::new()
        } else {
            let sql = format!(
                "SELECT artist, COUNT(*) FROM media \
                 WHERE artist IN ({}) GROUP BY artist",
                placeholders(artists.len())
            );
            self.conn
                .prepare(&sql)
                .and_then(|mut s| {
                    s.query_map(rusqlite::params_from_iter(artists.iter()), |r| {
                        Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?))
                    })
                    .map(|it| it.filter_map(Result::ok).collect())
                })
                .unwrap_or_default()
        };

        let top_tags: Vec<(String, u32)> = tags
            .iter()
            .filter(|v| !v.is_empty())
            .filter_map(|tag| {
                self.conn
                    .query_row(
                        "SELECT COUNT(*) FROM media WHERE ('|' || tags || '|') LIKE ?1",
                        rusqlite::params![format!("%|{}|%", tag)],
                        |r| r.get::<_, u32>(0),
                    )
                    .ok()
                    .map(|cnt| (tag.clone(), cnt))
            })
            .collect();

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

        let chars_sql = format!(
            "{cte}
             SELECT DISTINCT trim(word) FROM split WHERE trim(word) != '' ORDER BY 1",
            cte = split_cte("characters")
        );
        let characters: Vec<String> = self
            .conn
            .prepare(&chars_sql)
            .and_then(|mut s| {
                s.query_map([], |r| r.get::<_, String>(0))
                    .map(|it| it.filter_map(Result::ok).collect())
            })
            .unwrap_or_default();

        let tags_sql = format!(
            "{cte}
             SELECT DISTINCT trim(word) FROM split WHERE trim(word) != '' ORDER BY 1",
            cte = split_cte("tags")
        );
        let tags: Vec<String> = self
            .conn
            .prepare(&tags_sql)
            .and_then(|mut s| {
                s.query_map([], |r| r.get::<_, String>(0))
                    .map(|it| it.filter_map(Result::ok).collect())
            })
            .unwrap_or_default();

        AutocompleteData {
            artists,
            copyrights,
            characters,
            tags,
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

    pub fn query_group(&self, base_stem: &str, ext: &str, dir: &str) -> Vec<MediaItem> {
        let esc = |s: &str| s.replace('|', "||").replace('%', "|%").replace('_', "|_");

        let plain_name = format!("{}.{}", base_stem, ext);
        let suffixed_like = format!("{} - %.{}", esc(base_stem), esc(ext));

        let sep = if dir.ends_with(['/', '\\']) {
            ""
        } else {
            if dir.contains('\\') { "\\" } else { "/" }
        };
        let dir_prefix_like = format!("{}{}%", esc(dir), sep);

        let sql = format!(
            "SELECT {c} FROM media \
             WHERE path LIKE ? ESCAPE '|' \
               AND (name = ? OR name LIKE ? ESCAPE '|') \
             ORDER BY path ASC",
            c = SELECT_COLS,
        );

        let mut stmt = match self.conn.prepare_cached(&sql) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("query_group prepare: {e}");
                return Vec::new();
            }
        };

        stmt.query_map(
            rusqlite::params![dir_prefix_like, plain_name, suffixed_like],
            map_media_item,
        )
        .map(|it| it.filter_map(Result::ok).collect())
        .unwrap_or_else(|e| {
            eprintln!("query_group: {e}");
            Vec::new()
        })
    }
}
