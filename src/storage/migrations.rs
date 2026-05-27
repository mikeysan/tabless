use rusqlite::Connection;

use super::error::StorageError;

const INIT_SQL: &str = include_str!("migrations/0001_init.sql");

#[derive(Debug)]
pub struct MigrationRunner<'a> {
    conn: &'a mut Connection,
}

impl<'a> MigrationRunner<'a> {
    pub fn new(conn: &'a mut Connection) -> Self {
        MigrationRunner { conn }
    }

    pub fn run_all(&mut self) -> Result<(), StorageError> {
        let tx = self
            .conn
            .transaction()
            .map_err(|e| StorageError::MigrationFailed {
                version: 1,
                reason: e.to_string(),
            })?;

        tx.execute_batch(INIT_SQL)
            .map_err(|e| StorageError::MigrationFailed {
                version: 1,
                reason: e.to_string(),
            })?;

        tx.execute(
            "INSERT OR IGNORE INTO _migrations (version, applied_at) VALUES (1, ?1)",
            [Self::now()],
        )
        .map_err(|e| StorageError::MigrationFailed {
            version: 1,
            reason: e.to_string(),
        })?;

        tx.commit().map_err(|e| StorageError::MigrationFailed {
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

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;

    #[test]
    fn run_all_creates_tables() {
        let mut conn = Connection::open_in_memory().unwrap();
        let mut runner = MigrationRunner::new(&mut conn);
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
        let mut conn = Connection::open_in_memory().unwrap();
        let mut runner = MigrationRunner::new(&mut conn);
        runner.run_all().unwrap();
        runner.run_all().unwrap(); // should not fail
    }
}
