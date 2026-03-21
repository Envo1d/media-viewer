use rusqlite::Transaction;

pub fn init_schema_version(tx: &Transaction) {
    tx.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER NOT NULL
        )",
        [],
    )
    .unwrap();

    let count: i64 = tx
        .query_row("SELECT COUNT(*) FROM schema_version", [], |row| row.get(0))
        .unwrap();

    if count == 0 {
        tx.execute("INSERT INTO schema_version (version) VALUES (0)", [])
            .unwrap();
    }
}

fn get_version(tx: &Transaction) -> i64 {
    tx.query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
        row.get(0)
    })
    .unwrap()
}

fn set_version(tx: &Transaction, version: i64) {
    tx.execute(
        "UPDATE schema_version SET version = ?1",
        rusqlite::params![version],
    )
    .unwrap();
}

pub fn run_migrations(tx: &Transaction) {
    let mut version = get_version(tx);

    // === MIGRATION 1 ===
    if version < 1 {
        tx.execute(
            "CREATE TABLE media (
                path TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                category TEXT NOT NULL,
                author TEXT NOT NULL,
                media_type TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        version = 1;
        set_version(tx, version);
    }

    // === MIGRATION 2 ===
    if version < 2 {
        tx.execute(
            "ALTER TABLE media ADD COLUMN modified INTEGER DEFAULT 0",
            [],
        )
        .unwrap();

        version = 2;
        set_version(tx, version);
    }

    // === MIGRATION 3 ===
    if version < 3 {
        tx.execute("CREATE INDEX idx_category ON media(category)", [])
            .unwrap();

        tx.execute("CREATE INDEX idx_author ON media(author)", [])
            .unwrap();

        version = 3;
        set_version(tx, version);
    }

    if version < 4 {
        tx.execute(
            "CREATE VIRTUAL TABLE media_fts USING fts5(
            path,
            name,
            category,
            author
        )",
            [],
        )
        .unwrap();

        tx.execute(
            "CREATE TRIGGER trg_media_ai AFTER INSERT ON media BEGIN
                INSERT INTO media_fts(rowid, path, name, category, author)
                VALUES (new.rowid, new.path, new.name, new.category, new.author);
            END;",
            [],
        )
        .unwrap();

        tx.execute(
            "CREATE TRIGGER trg_media_ad AFTER DELETE ON media BEGIN
                DELETE FROM media_fts WHERE rowid = old.rowid;
            END;",
            [],
        )
        .unwrap();

        tx.execute(
            "CREATE TRIGGER trg_media_au AFTER UPDATE ON media BEGIN
                DELETE FROM media_fts WHERE rowid = old.rowid;
                INSERT INTO media_fts(rowid, path, name, category, author)
                VALUES (new.rowid, new.path, new.name, new.category, new.author);
            END;",
            [],
        )
        .unwrap();

        version = 4;
        set_version(tx, version);
    }
}
