# Storage Layer Design

## Overview

The storage layer persists validated URLs, tags, and collections in SQLite. It provides a
synchronous, type-safe API that downstream modules (UI, protocol handlers, search) use
without knowledge of the underlying database.

## Goals

- Persist `ValidatedUrl` instances with full metadata (title, favicon, timestamps).
- Support lightweight organisation: tags, collections, pinning, archiving.
- Provide fast full-text search via FTS5 and lightweight in-memory fuzzy matching.
- Use explicit, versioned migrations from the start.
- Return fine-grained, typed errors for every failure mode.
- Contain zero panics and zero unwraps in production paths.

## Non-Goals

- Cloud sync or remote storage.
- Complex relational queries beyond the MVP scope.
- Async/await APIs (the storage layer is synchronous; callers use a dedicated thread if needed).

## Tech Stack

- **Database:** SQLite (via `rusqlite` with `bundled` feature for FTS5).
- **Migrations:** Manual `.sql` files executed by a Rust migration runner.
- **Search:**
  - FTS5 for fast full-text queries.
  - `sublime_fuzzy` for lightweight in-memory fuzzy matching on titles and canonical URLs.

## Module Structure

```
src/
  storage/
    mod.rs           — public API: Storage, re-exports
    connection.rs    — Connection management + database path resolution
    migrations.rs    — Versioned migration runner
    error.rs         — StorageError enum
    url_repo.rs      — UrlRepository: CRUD, archive/pin, search
    tag_repo.rs      — TagRepository: create/list/delete, attach/detach
    collection_repo.rs — CollectionRepository: same pattern as tags
```

## Schema (Migration V1)

### URLs Table

```sql
CREATE TABLE urls (
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
```

### Tags Table

```sql
CREATE TABLE tags (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);
```

### Collections Table

```sql
CREATE TABLE collections (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);
```

### Junction Tables

```sql
CREATE TABLE url_tags (
    url_id INTEGER NOT NULL REFERENCES urls(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (url_id, tag_id)
);

CREATE TABLE url_collections (
    url_id        INTEGER NOT NULL REFERENCES urls(id) ON DELETE CASCADE,
    collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    PRIMARY KEY (url_id, collection_id)
);
```

### FTS5 Virtual Table

```sql
CREATE VIRTUAL TABLE fts_urls USING fts5(
    canonical_url,
    title,
    content='urls',
    content_rowid='id'
);
```

### Triggers (Keep FTS Index Synchronized)

```sql
CREATE TRIGGER urls_fts_insert AFTER INSERT ON urls BEGIN
    INSERT INTO fts_urls(rowid, canonical_url, title)
    VALUES (new.id, new.canonical_url, new.title);
END;

CREATE TRIGGER urls_fts_update AFTER UPDATE ON urls BEGIN
    INSERT INTO fts_urls(fts_urls, rowid, canonical_url, title)
    VALUES ('delete', old.id, old.canonical_url, old.title);
    INSERT INTO fts_urls(rowid, canonical_url, title)
    VALUES (new.id, new.canonical_url, new.title);
END;

CREATE TRIGGER urls_fts_delete AFTER DELETE ON urls BEGIN
    INSERT INTO fts_urls(fts_urls, rowid, canonical_url, title)
    VALUES ('delete', old.id, old.canonical_url, old.title);
END;
```

## Public API

### Storage

```rust
pub struct Storage {
    conn: Connection,
    pub urls: UrlRepository,
    pub tags: TagRepository,
    pub collections: CollectionRepository,
}

impl Storage {
    pub fn open(path: &Path) -> Result<Self, StorageError>;
}
```

### UrlRepository

```rust
pub struct UrlRepository<'a> { conn: &'a Connection }

impl<'a> UrlRepository<'a> {
    pub fn insert(&self, url: &ValidatedUrl, title: Option<&str>)
        -> Result<i64, StorageError>;
    pub fn find_by_id(&self, id: i64)
        -> Result<Option<UrlRecord>, StorageError>;
    pub fn list_inbox(&self)
        -> Result<Vec<UrlRecord>, StorageError>;
    pub fn list_archived(&self)
        -> Result<Vec<UrlRecord>, StorageError>;
    pub fn list_pinned(&self)
        -> Result<Vec<UrlRecord>, StorageError>;
    pub fn set_archived(&self, id: i64, archived: bool)
        -> Result<(), StorageError>;
    pub fn set_pinned(&self, id: i64, pinned: bool)
        -> Result<(), StorageError>;
    pub fn delete(&self, id: i64)
        -> Result<(), StorageError>;
    pub fn exists(&self, canonical: &str)
        -> Result<bool, StorageError>;
    pub fn search_fts(&self, query: &str)
        -> Result<Vec<UrlRecord>, StorageError>;
}
```

### TagRepository

```rust
pub struct TagRepository<'a> { conn: &'a Connection }

impl<'a> TagRepository<'a> {
    pub fn create(&self, name: &str)
        -> Result<i64, StorageError>;
    pub fn list_all(&self)
        -> Result<Vec<TagRecord>, StorageError>;
    pub fn delete(&self, id: i64)
        -> Result<(), StorageError>;
    pub fn attach_to_url(&self, url_id: i64, tag_id: i64)
        -> Result<(), StorageError>;
    pub fn detach_from_url(&self, url_id: i64, tag_id: i64)
        -> Result<(), StorageError>;
    pub fn list_for_url(&self, url_id: i64)
        -> Result<Vec<TagRecord>, StorageError>;
}
```

### CollectionRepository

Identical shape to `TagRepository` but operates on the `collections` and `url_collections` tables.

## Record Types

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlRecord {
    pub id: i64,
    pub canonical_url: String,
    pub original_url: String,
    pub title: Option<String>,
    pub favicon_path: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub archived: bool,
    pub pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagRecord {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionRecord {
    pub id: i64,
    pub name: String,
}
```

## Error Types

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    ConnectionFailed { reason: String },
    MigrationFailed { version: u32, reason: String },
    ConstraintViolation { table: String, reason: String },
    NotFound { table: String, id: i64 },
    QueryFailed { reason: String },
}
```

Implements `Display` and `std::error::Error`.

## Search Strategy

### Tier 1 — FTS5

Used for broad text filtering. Queries like `example` match `example.com` and titles containing "example".

### Tier 2 — In-Memory Fuzzy Matching

On startup, load all non-archived URL titles and canonical URLs into memory. During search, score each candidate with `sublime_fuzzy::FuzzySearch`. Return top-N results sorted by score.

The fuzzy layer is deliberately lightweight: two `Vec<String>` plus `sublime_fuzzy::FuzzySearch` state. For thousands of URLs, memory footprint is under 1MB and rebuild is instantaneous on insert/update/delete.

## Migrations

Each migration is a numbered `.sql` file in `src/storage/migrations/`:

```
migrations/
  0001_init.sql
```

The migration runner:

1. Creates a `_migrations` table tracking applied versions.
2. On startup, applies pending migrations in order.
3. Runs each migration inside a transaction.
4. Fails fast on any error — never auto-destructs data.

## Data Flow

```
Raw URL string
        |
        v
ValidatedUrl::parse(input)
        |
        v
UrlRepository::insert(&validated, title)
        |
        v
SQLite (urls table + fts_urls index)
        |
        v
Search or UI query returns UrlRecord
```

## Testing Strategy

- Unit tests for each repository method in `#[cfg(test)]` modules.
- Use an in-memory SQLite database for test isolation (`:memory:`).
- Test constraint violations (duplicate canonical URLs, foreign key failures).
- Test migration runner with mock migration files.
- Test FTS search and fuzzy matching with known data sets.

## Security Considerations

- All queries are parameterized — no string interpolation into SQL.
- The database is local-only and encrypted at rest by the OS (no application-level encryption in MVP).
- Foreign keys with `ON DELETE CASCADE` prevent orphaned junction records.

## Success Criteria

- `Storage::open` creates or migrates the database on first run.
- `UrlRepository::insert` accepts a `ValidatedUrl` and returns an ID.
- Duplicate canonical URLs are rejected with `ConstraintViolation`.
- FTS search returns relevant results for title and URL text.
- Archive/pin state changes persist and are reflected in list queries.
- Tags and collections can be created, attached, detached, and listed.
- All repository operations return typed errors, never panic.
