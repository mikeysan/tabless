PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS _migrations (
    version INTEGER PRIMARY KEY,
    applied_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS urls (
    id            INTEGER PRIMARY KEY,
    canonical_url TEXT NOT NULL UNIQUE,
    original_url  TEXT NOT NULL,
    title         TEXT,
    favicon_path  TEXT,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL,
    archived      INTEGER NOT NULL DEFAULT 0,
    pinned        INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS tags (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS collections (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS url_tags (
    url_id INTEGER NOT NULL REFERENCES urls(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (url_id, tag_id)
);

CREATE TABLE IF NOT EXISTS url_collections (
    url_id        INTEGER NOT NULL REFERENCES urls(id) ON DELETE CASCADE,
    collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    PRIMARY KEY (url_id, collection_id)
);

CREATE VIRTUAL TABLE IF NOT EXISTS fts_urls USING fts5(
    canonical_url,
    title,
    content='urls',
    content_rowid='id'
);

CREATE TRIGGER IF NOT EXISTS urls_fts_insert AFTER INSERT ON urls BEGIN
    INSERT INTO fts_urls(rowid, canonical_url, title)
    VALUES (new.id, new.canonical_url, new.title);
END;

CREATE TRIGGER IF NOT EXISTS urls_fts_update AFTER UPDATE ON urls BEGIN
    INSERT INTO fts_urls(fts_urls, rowid, canonical_url, title)
    VALUES ('delete', old.id, old.canonical_url, old.title);
    INSERT INTO fts_urls(rowid, canonical_url, title)
    VALUES (new.id, new.canonical_url, new.title);
END;

CREATE TRIGGER IF NOT EXISTS urls_fts_delete AFTER DELETE ON urls BEGIN
    INSERT INTO fts_urls(fts_urls, rowid, canonical_url, title)
    VALUES ('delete', old.id, old.canonical_url, old.title);
END;
