# Storage Layer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a SQLite-backed storage layer with migrations, CRUD repositories for URLs/tags/collections, and FTS5 + fuzzy search.

**Architecture:** Domain-split repositories (`UrlRepository`, `TagRepository`, `CollectionRepository`) share a single `rusqlite::Connection` via the `Storage` struct. Explicit SQL migrations with a Rust runner. Synchronous API throughout.

**Tech Stack:** Rust, `rusqlite` (bundled), `sublime_fuzzy`.

---

## File Structure

```
src/
  lib.rs                        -- add `pub mod storage;`
  storage/
    mod.rs                      -- Storage struct, re-exports
    connection.rs               -- open_connection(path) -> Result<Connection, StorageError>
    error.rs                    -- StorageError enum
    migrations.rs               -- MigrationRunner
    migrations/
      0001_init.sql             -- schema V1
    url_repo.rs                 -- UrlRepository
    tag_repo.rs                 -- TagRepository
    collection_repo.rs          -- CollectionRepository
    search.rs                   -- FuzzySearchIndex
tests/
  storage_integration_tests.rs  -- end-to-end storage tests
```

---

### Task 1: Add rusqlite Dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Update Cargo.toml**

Add to `[dependencies]`:

```toml
rusqlite = { version = "0.32", features = ["bundled"] }
sublime_fuzzy = "0.7"
```

- [ ] **Step 2: Verify project compiles**

Run: `cargo check`
Expected: Compiles successfully (new deps download).

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: add rusqlite and sublime_fuzzy dependencies"
```

---

### Task 2: Define StorageError

**Files:**
- Create: `src/storage/error.rs`
- Create: `src/storage/mod.rs` (minimal stub)

- [ ] **Step 1: Write failing test**

Create `tests/storage_integration_tests.rs`:

```rust
use tabless::storage::StorageError;

#[test]
fn error_variants_exist() {
    let _e = StorageError::ConnectionFailed {
        reason: "test".to_string(),
    };
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test storage_integration_tests`
Expected: FAIL — `storage` module not found.

- [ ] **Step 3: Implement StorageError**

Create `src/storage/error.rs`:

```rust
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    ConnectionFailed { reason: String },
    MigrationFailed { version: u32, reason: String },
    ConstraintViolation { table: String, reason: String },
    NotFound { table: String, id: i64 },
    QueryFailed { reason: String },
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::ConnectionFailed { reason } => {
                write!(f, "database connection failed: {}", reason)
            }
            StorageError::MigrationFailed { version, reason } => {
                write!(f, "migration {} failed: {}", version, reason)
            }
            StorageError::ConstraintViolation { table, reason } => {
                write!(f, "constraint violation in {}: {}", table, reason)
            }
            StorageError::NotFound { table, id } => {
                write!(f, "record not found in {} with id {}", table, id)
            }
            StorageError::QueryFailed { reason } => {
                write!(f, "query failed: {}", reason)
            }
        }
    }
}

impl std::error::Error for StorageError {}
```

- [ ] **Step 4: Wire up storage module**

Create `src/storage/mod.rs`:

```rust
pub mod error;

pub use error::StorageError;
```

- [ ] **Step 5: Wire up lib.rs**

Modify `src/lib.rs`:

```rust
pub mod storage;
pub mod url;
```

- [ ] **Step 6: Run tests**

Run: `cargo test --test storage_integration_tests`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml src/lib.rs src/storage/error.rs src/storage/mod.rs tests/storage_integration_tests.rs
git commit -m "feat: define StorageError enum"
```

---

### Task 3: Create Migration SQL and Runner

**Files:**
- Create: `src/storage/migrations/0001_init.sql`
- Create: `src/storage/migrations.rs`
- Modify: `src/storage/mod.rs`

- [ ] **Step 1: Write migration SQL**

Create `src/storage/migrations/0001_init.sql`:

```sql
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
```

- [ ] **Step 2: Write failing test for migration runner**

Add to `tests/storage_integration_tests.rs` (replace contents):

```rust
use tabless::storage::migrations::MigrationRunner;
use tabless::storage::StorageError;

#[test]
fn migration_runner_applies_schema() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let runner = MigrationRunner::new(&conn);
    let result = runner.run_all();
    assert!(result.is_ok());
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test --test storage_integration_tests migration_runner_applies_schema`
Expected: FAIL — `migrations` module not found.

- [ ] **Step 4: Implement MigrationRunner**

Create `src/storage/migrations.rs`:

```rust
use rusqlite::{Connection, OptionalExtension};

use super::error::StorageError;

const INIT_SQL: &str = include_str!("migrations/0001_init.sql");

pub struct MigrationRunner<'a> {
    conn: &'a Connection,
}

impl<'a> MigrationRunner<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        MigrationRunner { conn }
    }

    pub fn run_all(&self) -> Result<(), StorageError> {
        self.conn
            .execute_batch(INIT_SQL)
            .map_err(|e| StorageError::MigrationFailed {
                version: 1,
                reason: e.to_string(),
            })?;

        self.conn
            .execute(
                "INSERT OR IGNORE INTO _migrations (version, applied_at) VALUES (1, ?1)",
                [Self::now()],
            )
            .map_err(|e| StorageError::MigrationFailed {
                version: 1,
                reason: e.to_string(),
            })?;

        Ok(())
    }

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }
}
```

- [ ] **Step 5: Wire up migrations module**

Modify `src/storage/mod.rs`:

```rust
pub mod error;
pub mod migrations;

pub use error::StorageError;
```

- [ ] **Step 6: Run tests**

Run: `cargo test --test storage_integration_tests migration_runner_applies_schema`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/storage/migrations.rs src/storage/migrations/0001_init.sql src/storage/mod.rs tests/storage_integration_tests.rs
git commit -m "feat: add migration runner and V1 schema"
```

---

### Task 4: Implement Connection Manager

**Files:**
- Create: `src/storage/connection.rs`
- Modify: `src/storage/mod.rs`

- [ ] **Step 1: Write failing test**

Add to `tests/storage_integration_tests.rs`:

```rust
use tabless::storage::connection::open_connection;
use std::path::Path;

#[test]
fn open_in_memory_connection() {
    let result = open_connection(Path::new(":memory:"));
    assert!(result.is_ok());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test storage_integration_tests open_in_memory_connection`
Expected: FAIL — `connection` module not found.

- [ ] **Step 3: Implement connection manager**

Create `src/storage/connection.rs`:

```rust
use std::path::Path;

use rusqlite::Connection;

use super::error::StorageError;

pub fn open_connection(path: &Path) -> Result<Connection, StorageError> {
    let conn = Connection::open(path).map_err(|e| StorageError::ConnectionFailed {
        reason: e.to_string(),
    })?;

    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| StorageError::ConnectionFailed {
            reason: e.to_string(),
        })?;

    Ok(conn)
}
```

- [ ] **Step 4: Wire up connection module**

Modify `src/storage/mod.rs`:

```rust
pub mod connection;
pub mod error;
pub mod migrations;

pub use error::StorageError;
```

- [ ] **Step 5: Run tests**

Run: `cargo test --test storage_integration_tests open_in_memory_connection`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/storage/connection.rs src/storage/mod.rs tests/storage_integration_tests.rs
git commit -m "feat: add connection manager with foreign keys pragma"
```

---

### Task 5: Implement UrlRepository

**Files:**
- Create: `src/storage/url_repo.rs`
- Modify: `src/storage/mod.rs`
- Modify: `tests/storage_integration_tests.rs`

- [ ] **Step 1: Write failing integration tests**

Replace `tests/storage_integration_tests.rs`:

```rust
use tabless::storage::migrations::MigrationRunner;
use tabless::storage::url_repo::UrlRepository;
use tabless::storage::StorageError;
use tabless::url::ValidatedUrl;

fn setup() -> rusqlite::Connection {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let runner = MigrationRunner::new(&conn);
    runner.run_all().unwrap();
    conn
}

#[test]
fn url_repo_insert_and_find() {
    let conn = setup();
    let repo = UrlRepository::new(&conn);
    let url = ValidatedUrl::parse("https://example.com").unwrap();

    let id = repo.insert(&url, Some("Example")).unwrap();
    let found = repo.find_by_id(id).unwrap();
    assert!(found.is_some());
    let record = found.unwrap();
    assert_eq!(record.canonical_url, "https://example.com/");
    assert_eq!(record.original_url, "https://example.com");
    assert_eq!(record.title, Some("Example".to_string()));
}

#[test]
fn url_repo_rejects_duplicate_canonical() {
    let conn = setup();
    let repo = UrlRepository::new(&conn);
    let url = ValidatedUrl::parse("https://example.com").unwrap();

    repo.insert(&url, None).unwrap();
    let result = repo.insert(&url, None);
    assert!(matches!(
        result,
        Err(StorageError::ConstraintViolation { table, .. })
        if table == "urls"
    ));
}

#[test]
fn url_repo_archive_and_list() {
    let conn = setup();
    let repo = UrlRepository::new(&conn);
    let url = ValidatedUrl::parse("https://example.com").unwrap();

    let id = repo.insert(&url, None).unwrap();
    repo.set_archived(id, true).unwrap();

    let inbox = repo.list_inbox().unwrap();
    assert!(inbox.is_empty());

    let archived = repo.list_archived().unwrap();
    assert_eq!(archived.len(), 1);
}

#[test]
fn url_repo_pin_and_list() {
    let conn = setup();
    let repo = UrlRepository::new(&conn);
    let url = ValidatedUrl::parse("https://example.com").unwrap();

    let id = repo.insert(&url, None).unwrap();
    repo.set_pinned(id, true).unwrap();

    let pinned = repo.list_pinned().unwrap();
    assert_eq!(pinned.len(), 1);
}

#[test]
fn url_repo_delete() {
    let conn = setup();
    let repo = UrlRepository::new(&conn);
    let url = ValidatedUrl::parse("https://example.com").unwrap();

    let id = repo.insert(&url, None).unwrap();
    repo.delete(id).unwrap();
    let found = repo.find_by_id(id).unwrap();
    assert!(found.is_none());
}

#[test]
fn url_repo_exists() {
    let conn = setup();
    let repo = UrlRepository::new(&conn);
    let url = ValidatedUrl::parse("https://example.com").unwrap();

    repo.insert(&url, None).unwrap();
    assert!(repo.exists("https://example.com/").unwrap());
    assert!(!repo.exists("https://other.com/").unwrap());
}

#[test]
fn url_repo_search_fts() {
    let conn = setup();
    let repo = UrlRepository::new(&conn);
    let url = ValidatedUrl::parse("https://example.com").unwrap();

    repo.insert(&url, Some("Example Site")).unwrap();
    let results = repo.search_fts("example").unwrap();
    assert_eq!(results.len(), 1);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test storage_integration_tests`
Expected: FAIL — `url_repo` module not found.

- [ ] **Step 3: Implement UrlRepository**

Create `src/storage/url_repo.rs`:

```rust
use rusqlite::{params, Connection, OptionalExtension};

use crate::url::ValidatedUrl;

use super::error::StorageError;

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

pub struct UrlRepository<'a> {
    conn: &'a Connection,
}

impl<'a> UrlRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        UrlRepository { conn }
    }

    pub fn insert(
        &self,
        url: &ValidatedUrl,
        title: Option<&str>,
    ) -> Result<i64, StorageError> {
        let now = Self::now();
        self.conn
            .execute(
                "INSERT INTO urls (canonical_url, original_url, title, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![url.canonical(), url.original(), title, now, now],
            )
            .map_err(|e| {
                if e.to_string().contains("UNIQUE constraint failed") {
                    StorageError::ConstraintViolation {
                        table: "urls".to_string(),
                        reason: "duplicate canonical_url".to_string(),
                    }
                } else {
                    StorageError::QueryFailed {
                        reason: e.to_string(),
                    }
                }
            })?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn find_by_id(&self, id: i64) -> Result<Option<UrlRecord>, StorageError> {
        self.conn
            .query_row(
                "SELECT id, canonical_url, original_url, title, favicon_path,
                        created_at, updated_at, archived, pinned
                 FROM urls WHERE id = ?1",
                [id],
                |row| {
                    Ok(UrlRecord {
                        id: row.get(0)?,
                        canonical_url: row.get(1)?,
                        original_url: row.get(2)?,
                        title: row.get(3)?,
                        favicon_path: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                        archived: row.get::<_, i64>(7)? != 0,
                        pinned: row.get::<_, i64>(8)? != 0,
                    })
                },
            )
            .optional()
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })
    }

    pub fn list_inbox(&self) -> Result<Vec<UrlRecord>, StorageError> {
        self.list_where("archived = 0")
    }

    pub fn list_archived(&self) -> Result<Vec<UrlRecord>, StorageError> {
        self.list_where("archived = 1")
    }

    pub fn list_pinned(&self) -> Result<Vec<UrlRecord>, StorageError> {
        self.list_where("pinned = 1")
    }

    fn list_where(&self, condition: &str) -> Result<Vec<UrlRecord>, StorageError> {
        let sql = format!(
            "SELECT id, canonical_url, original_url, title, favicon_path,
                    created_at, updated_at, archived, pinned
             FROM urls WHERE {} ORDER BY created_at DESC",
            condition
        );
        let mut stmt = self
            .conn
            .prepare(&sql)
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        let rows = stmt
            .query_map([], |row| {
                Ok(UrlRecord {
                    id: row.get(0)?,
                    canonical_url: row.get(1)?,
                    original_url: row.get(2)?,
                    title: row.get(3)?,
                    favicon_path: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    archived: row.get::<_, i64>(7)? != 0,
                    pinned: row.get::<_, i64>(8)? != 0,
                })
            })
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })
    }

    pub fn set_archived(&self, id: i64, archived: bool) -> Result<(), StorageError> {
        self.conn
            .execute(
                "UPDATE urls SET archived = ?1, updated_at = ?2 WHERE id = ?3",
                params![archived as i64, Self::now(), id],
            )
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        Ok(())
    }

    pub fn set_pinned(&self, id: i64, pinned: bool) -> Result<(), StorageError> {
        self.conn
            .execute(
                "UPDATE urls SET pinned = ?1, updated_at = ?2 WHERE id = ?3",
                params![pinned as i64, Self::now(), id],
            )
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        Ok(())
    }

    pub fn delete(&self, id: i64) -> Result<(), StorageError> {
        self.conn
            .execute("DELETE FROM urls WHERE id = ?1", [id])
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        Ok(())
    }

    pub fn exists(&self, canonical: &str) -> Result<bool, StorageError> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM urls WHERE canonical_url = ?1",
                [canonical],
                |row| row.get(0),
            )
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        Ok(count > 0)
    }

    pub fn search_fts(&self, query: &str) -> Result<Vec<UrlRecord>, StorageError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT u.id, u.canonical_url, u.original_url, u.title, u.favicon_path,
                        u.created_at, u.updated_at, u.archived, u.pinned
                 FROM urls u
                 JOIN fts_urls f ON u.id = f.rowid
                 WHERE fts_urls MATCH ?1
                 ORDER BY rank",
            )
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        let rows = stmt
            .query_map([query], |row| {
                Ok(UrlRecord {
                    id: row.get(0)?,
                    canonical_url: row.get(1)?,
                    original_url: row.get(2)?,
                    title: row.get(3)?,
                    favicon_path: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    archived: row.get::<_, i64>(7)? != 0,
                    pinned: row.get::<_, i64>(8)? != 0,
                })
            })
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })
    }

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }
}
```

- [ ] **Step 4: Wire up url_repo module**

Modify `src/storage/mod.rs`:

```rust
pub mod connection;
pub mod error;
pub mod migrations;
pub mod url_repo;

pub use error::StorageError;
pub use url_repo::{UrlRecord, UrlRepository};
```

- [ ] **Step 5: Run tests**

Run: `cargo test --test storage_integration_tests`
Expected: All 8 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add src/storage/url_repo.rs src/storage/mod.rs tests/storage_integration_tests.rs
git commit -m "feat: implement UrlRepository with CRUD, archive/pin, and FTS search"
```

---

### Task 6: Implement TagRepository

**Files:**
- Create: `src/storage/tag_repo.rs`
- Modify: `src/storage/mod.rs`
- Modify: `tests/storage_integration_tests.rs`

- [ ] **Step 1: Write failing integration tests**

Add to `tests/storage_integration_tests.rs`:

```rust
use tabless::storage::tag_repo::TagRepository;

#[test]
fn tag_repo_create_and_list() {
    let conn = setup();
    let repo = TagRepository::new(&conn);

    let id = repo.create("rust").unwrap();
    let tags = repo.list_all().unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].name, "rust");
    assert_eq!(tags[0].id, id);
}

#[test]
fn tag_repo_attach_and_list_for_url() {
    let conn = setup();
    let url_repo = UrlRepository::new(&conn);
    let tag_repo = TagRepository::new(&conn);
    let url = ValidatedUrl::parse("https://example.com").unwrap();

    let url_id = url_repo.insert(&url, None).unwrap();
    let tag_id = tag_repo.create("rust").unwrap();
    tag_repo.attach_to_url(url_id, tag_id).unwrap();

    let tags = tag_repo.list_for_url(url_id).unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].name, "rust");
}

#[test]
fn tag_repo_detach() {
    let conn = setup();
    let url_repo = UrlRepository::new(&conn);
    let tag_repo = TagRepository::new(&conn);
    let url = ValidatedUrl::parse("https://example.com").unwrap();

    let url_id = url_repo.insert(&url, None).unwrap();
    let tag_id = tag_repo.create("rust").unwrap();
    tag_repo.attach_to_url(url_id, tag_id).unwrap();
    tag_repo.detach_from_url(url_id, tag_id).unwrap();

    let tags = tag_repo.list_for_url(url_id).unwrap();
    assert!(tags.is_empty());
}

#[test]
fn tag_repo_delete() {
    let conn = setup();
    let repo = TagRepository::new(&conn);

    let id = repo.create("rust").unwrap();
    repo.delete(id).unwrap();
    let tags = repo.list_all().unwrap();
    assert!(tags.is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test storage_integration_tests tag_repo`
Expected: FAIL — `tag_repo` module not found.

- [ ] **Step 3: Implement TagRepository**

Create `src/storage/tag_repo.rs`:

```rust
use rusqlite::{Connection, OptionalExtension};

use super::error::StorageError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagRecord {
    pub id: i64,
    pub name: String,
}

pub struct TagRepository<'a> {
    conn: &'a Connection,
}

impl<'a> TagRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        TagRepository { conn }
    }

    pub fn create(&self, name: &str) -> Result<i64, StorageError> {
        self.conn
            .execute(
                "INSERT INTO tags (name) VALUES (?1)",
                [name],
            )
            .map_err(|e| {
                if e.to_string().contains("UNIQUE constraint failed") {
                    StorageError::ConstraintViolation {
                        table: "tags".to_string(),
                        reason: "duplicate name".to_string(),
                    }
                } else {
                    StorageError::QueryFailed {
                        reason: e.to_string(),
                    }
                }
            })?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_all(&self) -> Result<Vec<TagRecord>, StorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name FROM tags ORDER BY name")
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        let rows = stmt
            .query_map([], |row| {
                Ok(TagRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })
    }

    pub fn delete(&self, id: i64) -> Result<(), StorageError> {
        self.conn
            .execute("DELETE FROM tags WHERE id = ?1", [id])
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        Ok(())
    }

    pub fn attach_to_url(&self, url_id: i64, tag_id: i64) -> Result<(), StorageError> {
        self.conn
            .execute(
                "INSERT INTO url_tags (url_id, tag_id) VALUES (?1, ?2)",
                [url_id, tag_id],
            )
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        Ok(())
    }

    pub fn detach_from_url(&self, url_id: i64, tag_id: i64) -> Result<(), StorageError> {
        self.conn
            .execute(
                "DELETE FROM url_tags WHERE url_id = ?1 AND tag_id = ?2",
                [url_id, tag_id],
            )
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        Ok(())
    }

    pub fn list_for_url(&self, url_id: i64) -> Result<Vec<TagRecord>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.name FROM tags t
             JOIN url_tags ut ON t.id = ut.tag_id
             WHERE ut.url_id = ?1
             ORDER BY t.name"
        ).map_err(|e| StorageError::QueryFailed {
            reason: e.to_string(),
        })?;
        let rows = stmt.query_map([url_id], |row| {
            Ok(TagRecord {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        }).map_err(|e| StorageError::QueryFailed {
            reason: e.to_string(),
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })
    }
}
```

- [ ] **Step 4: Wire up tag_repo module**

Modify `src/storage/mod.rs`:

```rust
pub mod connection;
pub mod error;
pub mod migrations;
pub mod tag_repo;
pub mod url_repo;

pub use error::StorageError;
pub use tag_repo::{TagRecord, TagRepository};
pub use url_repo::{UrlRecord, UrlRepository};
```

- [ ] **Step 5: Run tests**

Run: `cargo test --test storage_integration_tests tag_repo`
Expected: All 4 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add src/storage/tag_repo.rs src/storage/mod.rs tests/storage_integration_tests.rs
git commit -m "feat: implement TagRepository with CRUD and URL attachment"
```

---

### Task 7: Implement CollectionRepository

**Files:**
- Create: `src/storage/collection_repo.rs`
- Modify: `src/storage/mod.rs`
- Modify: `tests/storage_integration_tests.rs`

- [ ] **Step 1: Write failing integration tests**

Add to `tests/storage_integration_tests.rs`:

```rust
use tabless::storage::collection_repo::CollectionRepository;

#[test]
fn collection_repo_create_and_list() {
    let conn = setup();
    let repo = CollectionRepository::new(&conn);

    let id = repo.create("work").unwrap();
    let collections = repo.list_all().unwrap();
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0].name, "work");
    assert_eq!(collections[0].id, id);
}

#[test]
fn collection_repo_attach_and_list_for_url() {
    let conn = setup();
    let url_repo = UrlRepository::new(&conn);
    let coll_repo = CollectionRepository::new(&conn);
    let url = ValidatedUrl::parse("https://example.com").unwrap();

    let url_id = url_repo.insert(&url, None).unwrap();
    let coll_id = coll_repo.create("work").unwrap();
    coll_repo.attach_to_url(url_id, coll_id).unwrap();

    let collections = coll_repo.list_for_url(url_id).unwrap();
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0].name, "work");
}

#[test]
fn collection_repo_delete() {
    let conn = setup();
    let repo = CollectionRepository::new(&conn);

    let id = repo.create("work").unwrap();
    repo.delete(id).unwrap();
    let collections = repo.list_all().unwrap();
    assert!(collections.is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test storage_integration_tests collection_repo`
Expected: FAIL — `collection_repo` module not found.

- [ ] **Step 3: Implement CollectionRepository**

Create `src/storage/collection_repo.rs`:

```rust
use rusqlite::Connection;

use super::error::StorageError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionRecord {
    pub id: i64,
    pub name: String,
}

pub struct CollectionRepository<'a> {
    conn: &'a Connection,
}

impl<'a> CollectionRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        CollectionRepository { conn }
    }

    pub fn create(&self, name: &str) -> Result<i64, StorageError> {
        self.conn
            .execute(
                "INSERT INTO collections (name) VALUES (?1)",
                [name],
            )
            .map_err(|e| {
                if e.to_string().contains("UNIQUE constraint failed") {
                    StorageError::ConstraintViolation {
                        table: "collections".to_string(),
                        reason: "duplicate name".to_string(),
                    }
                } else {
                    StorageError::QueryFailed {
                        reason: e.to_string(),
                    }
                }
            })?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_all(&self) -> Result<Vec<CollectionRecord>, StorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name FROM collections ORDER BY name")
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        let rows = stmt
            .query_map([], |row| {
                Ok(CollectionRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })
    }

    pub fn delete(&self, id: i64) -> Result<(), StorageError> {
        self.conn
            .execute("DELETE FROM collections WHERE id = ?1", [id])
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        Ok(())
    }

    pub fn attach_to_url(&self, url_id: i64, collection_id: i64) -> Result<(), StorageError> {
        self.conn
            .execute(
                "INSERT INTO url_collections (url_id, collection_id) VALUES (?1, ?2)",
                [url_id, collection_id],
            )
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        Ok(())
    }

    pub fn detach_from_url(&self, url_id: i64, collection_id: i64) -> Result<(), StorageError> {
        self.conn
            .execute(
                "DELETE FROM url_collections WHERE url_id = ?1 AND collection_id = ?2",
                [url_id, collection_id],
            )
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        Ok(())
    }

    pub fn list_for_url(&self, url_id: i64) -> Result<Vec<CollectionRecord>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT c.id, c.name FROM collections c
             JOIN url_collections uc ON c.id = uc.collection_id
             WHERE uc.url_id = ?1
             ORDER BY c.name"
        ).map_err(|e| StorageError::QueryFailed {
            reason: e.to_string(),
        })?;
        let rows = stmt.query_map([url_id], |row| {
            Ok(CollectionRecord {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        }).map_err(|e| StorageError::QueryFailed {
            reason: e.to_string(),
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })
    }
}
```

- [ ] **Step 4: Wire up collection_repo module**

Modify `src/storage/mod.rs`:

```rust
pub mod collection_repo;
pub mod connection;
pub mod error;
pub mod migrations;
pub mod tag_repo;
pub mod url_repo;

pub use collection_repo::{CollectionRecord, CollectionRepository};
pub use error::StorageError;
pub use tag_repo::{TagRecord, TagRepository};
pub use url_repo::{UrlRecord, UrlRepository};
```

- [ ] **Step 5: Run tests**

Run: `cargo test --test storage_integration_tests collection_repo`
Expected: All 3 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add src/storage/collection_repo.rs src/storage/mod.rs tests/storage_integration_tests.rs
git commit -m "feat: implement CollectionRepository with CRUD and URL attachment"
```

---

### Task 8: Implement Storage Struct

**Files:**
- Modify: `src/storage/mod.rs`
- Modify: `tests/storage_integration_tests.rs`

- [ ] **Step 1: Write failing integration test**

Add to `tests/storage_integration_tests.rs`:

```rust
use tabless::storage::Storage;
use std::path::Path;

#[test]
fn storage_open_in_memory() {
    let storage = Storage::open(Path::new(":memory:")).unwrap();
    let url = ValidatedUrl::parse("https://example.com").unwrap();
    let id = storage.urls.insert(&url, Some("Example")).unwrap();
    let found = storage.urls.find_by_id(id).unwrap();
    assert!(found.is_some());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test storage_integration_tests storage_open_in_memory`
Expected: FAIL — `Storage` struct not found.

- [ ] **Step 3: Implement Storage struct**

Modify `src/storage/mod.rs` to add the `Storage` struct (append at end):

```rust
use std::path::Path;

use rusqlite::Connection;

pub struct Storage {
    conn: Connection,
    pub urls: UrlRepository<'static>,
    pub tags: TagRepository<'static>,
    pub collections: CollectionRepository<'static>,
}
```

Wait — that's not valid Rust. `UrlRepository<'static>` requires a `'static` connection which we don't have. We need to use `unsafe` to extend the lifetime, or restructure. Actually, a better pattern for a small project is to store the connection and access repositories through methods:

```rust
pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        let conn = connection::open_connection(path)?;
        let runner = migrations::MigrationRunner::new(&conn);
        runner.run_all()?;
        Ok(Storage { conn })
    }

    pub fn urls(&self) -> UrlRepository<'_> {
        UrlRepository::new(&self.conn)
    }

    pub fn tags(&self) -> TagRepository<'_> {
        TagRepository::new(&self.conn)
    }

    pub fn collections(&self) -> CollectionRepository<'_> {
        CollectionRepository::new(&self.conn)
    }
}
```

Actually, this is cleaner. Let me write the full `src/storage/mod.rs`:

```rust
pub mod collection_repo;
pub mod connection;
pub mod error;
pub mod migrations;
pub mod tag_repo;
pub mod url_repo;

pub use collection_repo::{CollectionRecord, CollectionRepository};
pub use error::StorageError;
pub use tag_repo::{TagRecord, TagRepository};
pub use url_repo::{UrlRecord, UrlRepository};

use std::path::Path;

use rusqlite::Connection;

pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        let conn = connection::open_connection(path)?;
        let runner = migrations::MigrationRunner::new(&conn);
        runner.run_all()?;
        Ok(Storage { conn })
    }

    pub fn urls(&self) -> UrlRepository<'_> {
        UrlRepository::new(&self.conn)
    }

    pub fn tags(&self) -> TagRepository<'_> {
        TagRepository::new(&self.conn)
    }

    pub fn collections(&self) -> CollectionRepository<'_> {
        CollectionRepository::new(&self.conn)
    }
}
```

- [ ] **Step 4: Update integration test for Storage API**

Modify `tests/storage_integration_tests.rs`:

```rust
#[test]
fn storage_open_in_memory() {
    let storage = Storage::open(Path::new(":memory:")).unwrap();
    let url = ValidatedUrl::parse("https://example.com").unwrap();
    let id = storage.urls().insert(&url, Some("Example")).unwrap();
    let found = storage.urls().find_by_id(id).unwrap();
    assert!(found.is_some());
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test --test storage_integration_tests storage_open_in_memory`
Expected: PASS.

- [ ] **Step 6: Run full test suite**

Run: `cargo test`
Expected: All tests PASS.

- [ ] **Step 7: Commit**

```bash
git add src/storage/mod.rs tests/storage_integration_tests.rs
git commit -m "feat: implement Storage struct composing all repositories"
```

---

### Task 9: Add Fuzzy Search

**Files:**
- Create: `src/storage/search.rs`
- Modify: `src/storage/mod.rs`
- Modify: `src/storage/url_repo.rs`

- [ ] **Step 1: Write failing integration test**

Add to `tests/storage_integration_tests.rs`:

```rust
use tabless::storage::search::FuzzySearchIndex;

#[test]
fn fuzzy_search_finds_match() {
    let conn = setup();
    let repo = UrlRepository::new(&conn);
    let url1 = ValidatedUrl::parse("https://rust-lang.org").unwrap();
    let url2 = ValidatedUrl::parse("https://example.com").unwrap();

    repo.insert(&url1, Some("Rust Language")).unwrap();
    repo.insert(&url2, Some("Example")).unwrap();

    let mut index = FuzzySearchIndex::new();
    index.rebuild(&repo).unwrap();

    let results = index.search("rust");
    assert!(!results.is_empty());
    assert_eq!(results[0].canonical_url, "https://rust-lang.org/");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test storage_integration_tests fuzzy_search_finds_match`
Expected: FAIL — `search` module not found.

- [ ] **Step 3: Implement FuzzySearchIndex**

Create `src/storage/search.rs`:

```rust
use sublime_fuzzy::{FuzzySearch, Scoring};

use super::error::StorageError;
use super::url_repo::{UrlRecord, UrlRepository};

pub struct FuzzySearchIndex {
    candidates: Vec<String>,
    scoring: Scoring,
}

impl FuzzySearchIndex {
    pub fn new() -> Self {
        FuzzySearchIndex {
            candidates: Vec::new(),
            scoring: Scoring::default(),
        }
    }

    pub fn rebuild(&mut self, repo: &UrlRepository<'_>) -> Result<(), StorageError> {
        self.candidates.clear();
        let urls = repo.list_inbox()?;
        for url in urls {
            self.candidates.push(url.canonical_url.clone());
            if let Some(title) = url.title {
                self.candidates.push(title);
            }
        }
        Ok(())
    }

    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = self
            .candidates
            .iter()
            .filter_map(|candidate| {
                FuzzySearch::new(query, candidate)
                    .score_with(&self.scoring)
                    .best_match()
                    .map(|m| SearchResult {
                        text: candidate.clone(),
                        score: m.score(),
                    })
            })
            .collect();

        results.sort_by(|a, b| b.score.cmp(&a.score));
        results.truncate(20);
        results
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub text: String,
    pub score: i64,
}
```

- [ ] **Step 4: Wire up search module**

Modify `src/storage/mod.rs`:

```rust
pub mod collection_repo;
pub mod connection;
pub mod error;
pub mod migrations;
pub mod search;
pub mod tag_repo;
pub mod url_repo;

pub use collection_repo::{CollectionRecord, CollectionRepository};
pub use error::StorageError;
pub use search::{FuzzySearchIndex, SearchResult};
pub use tag_repo::{TagRecord, TagRepository};
pub use url_repo::{UrlRecord, UrlRepository};
```

- [ ] **Step 5: Run tests**

Run: `cargo test --test storage_integration_tests fuzzy_search_finds_match`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/storage/search.rs src/storage/mod.rs tests/storage_integration_tests.rs
git commit -m "feat: add FuzzySearchIndex with sublime_fuzzy"
```

---

### Task 10: Add Unit Tests for Edge Cases

**Files:**
- Modify: `src/storage/url_repo.rs`
- Modify: `src/storage/tag_repo.rs`
- Modify: `src/storage/collection_repo.rs`
- Modify: `src/storage/migrations.rs`

- [ ] **Step 1: Add unit tests to url_repo.rs**

Append to `src/storage/url_repo.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrations::MigrationRunner;
    use crate::url::ValidatedUrl;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        let runner = MigrationRunner::new(&conn);
        runner.run_all().unwrap();
        conn
    }

    #[test]
    fn insert_returns_id() {
        let conn = setup();
        let repo = UrlRepository::new(&conn);
        let url = ValidatedUrl::parse("https://example.com").unwrap();
        let id = repo.insert(&url, None).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn find_missing_returns_none() {
        let conn = setup();
        let repo = UrlRepository::new(&conn);
        let result = repo.find_by_id(999).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_inbox_excludes_archived() {
        let conn = setup();
        let repo = UrlRepository::new(&conn);
        let url = ValidatedUrl::parse("https://example.com").unwrap();
        let id = repo.insert(&url, None).unwrap();
        repo.set_archived(id, true).unwrap();
        let inbox = repo.list_inbox().unwrap();
        assert!(inbox.is_empty());
    }

    #[test]
    fn exists_false_for_unknown() {
        let conn = setup();
        let repo = UrlRepository::new(&conn);
        assert!(!repo.exists("https://unknown.com/").unwrap());
    }
}
```

- [ ] **Step 2: Add unit tests to tag_repo.rs**

Append to `src/storage/tag_repo.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrations::MigrationRunner;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        let runner = MigrationRunner::new(&conn);
        runner.run_all().unwrap();
        conn
    }

    #[test]
    fn create_duplicate_fails() {
        let conn = setup();
        let repo = TagRepository::new(&conn);
        repo.create("rust").unwrap();
        let result = repo.create("rust");
        assert!(matches!(result, Err(StorageError::ConstraintViolation { .. })));
    }

    #[test]
    fn list_for_url_empty_when_none_attached() {
        let conn = setup();
        let repo = TagRepository::new(&conn);
        let tags = repo.list_for_url(1).unwrap();
        assert!(tags.is_empty());
    }
}
```

- [ ] **Step 3: Add unit tests to collection_repo.rs**

Append to `src/storage/collection_repo.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrations::MigrationRunner;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        let runner = MigrationRunner::new(&conn);
        runner.run_all().unwrap();
        conn
    }

    #[test]
    fn create_duplicate_fails() {
        let conn = setup();
        let repo = CollectionRepository::new(&conn);
        repo.create("work").unwrap();
        let result = repo.create("work");
        assert!(matches!(result, Err(StorageError::ConstraintViolation { .. })));
    }

    #[test]
    fn list_for_url_empty_when_none_attached() {
        let conn = setup();
        let repo = CollectionRepository::new(&conn);
        let collections = repo.list_for_url(1).unwrap();
        assert!(collections.is_empty());
    }
}
```

- [ ] **Step 4: Add unit tests to migrations.rs**

Append to `src/storage/migrations.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_all_creates_tables() {
        let conn = Connection::open_in_memory().unwrap();
        let runner = MigrationRunner::new(&conn);
        runner.run_all().unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(count > 0);
    }

    #[test]
    fn run_all_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        let runner = MigrationRunner::new(&conn);
        runner.run_all().unwrap();
        runner.run_all().unwrap(); // should not fail
    }
}
```

- [ ] **Step 5: Run all tests**

Run: `cargo test`
Expected: All unit and integration tests PASS.

- [ ] **Step 6: Commit**

```bash
git add src/storage/url_repo.rs src/storage/tag_repo.rs src/storage/collection_repo.rs src/storage/migrations.rs
git commit -m "test: add edge-case unit tests for repositories and migrations"
```

---

### Task 11: Final Verification

- [ ] **Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests PASS.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings.

- [ ] **Step 3: Run formatter**

Run: `cargo fmt`
Expected: Formats cleanly, no changes needed (or changes applied).

- [ ] **Step 4: Build release**

Run: `cargo build --release`
Expected: Compiles successfully.

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "chore: final verification and formatting"
```

---

## Self-Review Checklist

| Spec Requirement | Task | Status |
|---|---|---|
| SQLite persistence via rusqlite | Task 1, 4 | Covered |
| Explicit migrations | Task 3 | Covered |
| URLs table with canonical, original, title, favicon, timestamps | Task 5 | Covered |
| Tags table | Task 6 | Covered |
| Collections table | Task 7 | Covered |
| Junction tables with foreign keys | Task 3 | Covered |
| Pin and archive states | Task 5 | Covered |
| FTS5 search | Task 3, 5 | Covered |
| Fuzzy search | Task 9 | Covered |
| Typed errors | Task 2 | Covered |
| Zero panics/unwraps in production | All tasks | Covered |

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-05-28-storage-layer.md`. Two execution options:

1. **Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks, fast iteration
2. **Inline Execution** — execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach would you prefer?
