use std::path::Path;
use tabless::storage::connection::open_connection;
use tabless::storage::migrations::MigrationRunner;

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
}
