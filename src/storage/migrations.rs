use rusqlite::Connection;

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
