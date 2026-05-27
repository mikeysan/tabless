use std::path::Path;
use tabless::storage::connection::open_connection;
use tabless::storage::migrations::MigrationRunner;
use tabless::storage::url_repo::UrlRepository;
use tabless::storage::StorageError;
use tabless::url::ValidatedUrl;

fn setup() -> rusqlite::Connection {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    let mut runner = MigrationRunner::new(&mut conn);
    runner.run_all().unwrap();
    conn
}

#[test]
fn migration_runner_applies_schema() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    let mut runner = MigrationRunner::new(&mut conn);
    let result = runner.run_all();
    assert!(result.is_ok());

    // Verify tables were created
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'urls'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);

    // Verify _migrations tracking
    let version: i64 = conn
        .query_row(
            "SELECT version FROM _migrations LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(version, 1);
}

#[test]
fn migration_runner_is_idempotent() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    let mut runner = MigrationRunner::new(&mut conn);
    runner.run_all().unwrap();
    runner.run_all().unwrap(); // should not fail
}

#[test]
fn open_in_memory_connection() {
    let result = open_connection(Path::new(":memory:"));
    assert!(result.is_ok());

    let conn = result.unwrap();
    let fk: i32 = conn
        .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
        .unwrap();
    assert_eq!(fk, 1);
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
