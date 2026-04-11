use rusqlite::Transaction;

pub fn init_schema_version(tx: &Transaction) {
    tx.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL)",
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

    // === MIGRATION 4 ===
    if version < 4 {
        tx.execute(
            "CREATE VIRTUAL TABLE media_fts USING fts5(path, name, category, author)",
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
        tx.execute(
            "INSERT INTO media_fts(rowid, path, name, category, author)
             SELECT rowid, path, name, category, author FROM media",
            [],
        )
        .unwrap();
        version = 4;
        set_version(tx, version);
    }

    // === MIGRATION 5 ===
    if version < 5 {
        tx.execute(
            "ALTER TABLE media ADD COLUMN last_seen_scan INTEGER DEFAULT 0",
            [],
        )
        .unwrap();
        tx.execute("CREATE INDEX idx_last_seen ON media(last_seen_scan)", [])
            .unwrap();
        version = 5;
        set_version(tx, version);
    }

    // === MIGRATION 6 ===
    if version < 6 {
        tx.execute("DROP TRIGGER IF EXISTS trg_media_au", [])
            .unwrap();
        tx.execute(
            "CREATE TRIGGER trg_media_au AFTER UPDATE ON media
             WHEN old.name     != new.name
               OR old.category != new.category
               OR old.author   != new.author
               OR old.path     != new.path
             BEGIN
                 DELETE FROM media_fts WHERE rowid = old.rowid;
                 INSERT INTO media_fts(rowid, path, name, category, author)
                 VALUES (new.rowid, new.path, new.name, new.category, new.author);
             END;",
            [],
        )
        .unwrap();
        version = 6;
        set_version(tx, version);
    }

    // === MIGRATION 7 ===
    // • Rename category → copyright, author → artist
    // • Add characters TEXT (scanner + user-editable)
    // • Add tags TEXT (user-managed only; never clobbered by scanner)
    // • Rebuild FTS5 table and all three triggers with new column set
    // • FTS triggers use replace(col,'|',' ') so unicode61 tokenises each
    // • pipe-separated value as an independent search token
    if version < 7 {
        tx.execute("ALTER TABLE media RENAME COLUMN category TO copyright", [])
            .unwrap();
        tx.execute("ALTER TABLE media RENAME COLUMN author TO artist", [])
            .unwrap();

        tx.execute("DROP INDEX IF EXISTS idx_category", []).unwrap();
        tx.execute("DROP INDEX IF EXISTS idx_author", []).unwrap();
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_copyright ON media(copyright)",
            [],
        )
        .unwrap();
        tx.execute("CREATE INDEX IF NOT EXISTS idx_artist ON media(artist)", [])
            .unwrap();

        tx.execute(
            "ALTER TABLE media ADD COLUMN characters TEXT NOT NULL DEFAULT ''",
            [],
        )
        .unwrap();
        tx.execute(
            "ALTER TABLE media ADD COLUMN tags TEXT NOT NULL DEFAULT ''",
            [],
        )
        .unwrap();

        tx.execute("DROP TRIGGER IF EXISTS trg_media_ai", [])
            .unwrap();
        tx.execute("DROP TRIGGER IF EXISTS trg_media_ad", [])
            .unwrap();
        tx.execute("DROP TRIGGER IF EXISTS trg_media_au", [])
            .unwrap();
        tx.execute("DROP TABLE IF EXISTS media_fts", []).unwrap();

        tx.execute(
            "CREATE VIRTUAL TABLE media_fts USING fts5(
                path, name, copyright, artist, characters, tags
            )",
            [],
        )
        .unwrap();

        tx.execute(
            "CREATE TRIGGER trg_media_ai AFTER INSERT ON media BEGIN
                INSERT INTO media_fts(rowid, path, name, copyright, artist, characters, tags)
                VALUES (
                    new.rowid, new.path, new.name, new.copyright, new.artist,
                    replace(new.characters, '|', ' '),
                    replace(new.tags,       '|', ' ')
                );
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
            "CREATE TRIGGER trg_media_au AFTER UPDATE ON media
             WHEN old.name       != new.name
               OR old.copyright  != new.copyright
               OR old.artist     != new.artist
               OR old.path       != new.path
               OR old.characters != new.characters
               OR old.tags       != new.tags
             BEGIN
                 DELETE FROM media_fts WHERE rowid = old.rowid;
                 INSERT INTO media_fts(rowid, path, name, copyright, artist, characters, tags)
                 VALUES (
                     new.rowid, new.path, new.name, new.copyright, new.artist,
                     replace(new.characters, '|', ' '),
                     replace(new.tags,       '|', ' ')
                 );
             END;",
            [],
        )
        .unwrap();

        tx.execute(
            "INSERT INTO media_fts(rowid, path, name, copyright, artist, characters, tags)
             SELECT rowid, path, name, copyright, artist, '', '' FROM media",
            [],
        )
        .unwrap();

        version = 7;
        set_version(tx, version);
    }
}
