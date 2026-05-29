use std::path::Path;
use tabless::storage::collection_repo::CollectionRepository;
use tabless::storage::connection::open_connection;
use tabless::storage::migrations::MigrationRunner;
use tabless::storage::tag_repo::TagRepository;
use tabless::storage::url_repo::UrlRepository;
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
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 2);
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
fn url_repo_duplicate_bumps_to_top() {
    let conn = setup();
    let repo = UrlRepository::new(&conn);
    let url = ValidatedUrl::parse("https://example.com").unwrap();

    let id1 = repo.insert(&url, None).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let id2 = repo.insert(&url, None).unwrap();

    assert_eq!(id1, id2);
    let record = repo.find_by_id(id1).unwrap().unwrap();
    assert!(record.updated_at >= record.created_at);
}

#[test]
fn url_repo_archive_and_list() {
    let conn = setup();
    let repo = UrlRepository::new(&conn);
    let url = ValidatedUrl::parse("https://example.com").unwrap();

    let id = repo.insert(&url, None).unwrap();
    repo.set_archived(id, true).unwrap();

    let main = repo.list_main().unwrap();
    assert!(main.is_empty());

    let archived = repo.list_archived().unwrap();
    assert_eq!(archived.len(), 1);
}

#[test]
fn url_repo_favorite_and_list() {
    let conn = setup();
    let repo = UrlRepository::new(&conn);
    let url = ValidatedUrl::parse("https://example.com").unwrap();

    let id = repo.insert(&url, None).unwrap();
    repo.set_favorite(id, true).unwrap();

    let favorites = repo.list_favorites().unwrap();
    assert_eq!(favorites.len(), 1);
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
fn collection_repo_detach() {
    let conn = setup();
    let url_repo = UrlRepository::new(&conn);
    let coll_repo = CollectionRepository::new(&conn);
    let url = ValidatedUrl::parse("https://example.com").unwrap();

    let url_id = url_repo.insert(&url, None).unwrap();
    let coll_id = coll_repo.create("work").unwrap();
    coll_repo.attach_to_url(url_id, coll_id).unwrap();
    coll_repo.detach_from_url(url_id, coll_id).unwrap();

    let collections = coll_repo.list_for_url(url_id).unwrap();
    assert!(collections.is_empty());
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

use tabless::storage::Storage;

#[test]
fn storage_open_in_memory() {
    let storage = Storage::open(Path::new(":memory:")).unwrap();
    let url = ValidatedUrl::parse("https://example.com").unwrap();
    let id = storage.urls().insert(&url, Some("Example")).unwrap();
    let found = storage.urls().find_by_id(id).unwrap();
    assert!(found.is_some());
}

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
    assert_eq!(results[0].text, "https://rust-lang.org/");
}
