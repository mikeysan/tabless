use tabless::storage::migrations::MigrationRunner;
use tabless::storage::StorageError;

#[test]
fn migration_runner_applies_schema() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let runner = MigrationRunner::new(&conn);
    let result = runner.run_all();
    assert!(result.is_ok());
}
