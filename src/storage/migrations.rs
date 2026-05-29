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
        let current = self.current_version()?;
        if current < 1 {
            self.run_migration(1, INIT_SQL)?;
        }
        if current < 2 {
            self.add_favorite_columns()?;
        }
        Ok(())
    }

    fn current_version(&mut self) -> Result<u32, StorageError> {
        let exists = match self.conn.query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '_migrations'",
            [],
            |_| Ok(true),
        ) {
            Ok(v) => v,
            Err(rusqlite::Error::QueryReturnedNoRows) => false,
            Err(e) => {
                return Err(StorageError::MigrationFailed {
                    version: 0,
                    reason: e.to_string(),
                });
            }
        };

        if !exists {
            return Ok(0);
        }

        match self
            .conn
            .query_row("SELECT MAX(version) FROM _migrations", [], |row| row.get(0))
        {
            Ok(Some(v)) => Ok(v),
            Ok(None) => Ok(0),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
            Err(e) => Err(StorageError::MigrationFailed {
                version: 0,
                reason: e.to_string(),
            }),
        }
    }

    fn run_migration(&mut self, version: u32, sql: &str) -> Result<(), StorageError> {
        let tx = self
            .conn
            .transaction()
            .map_err(|e| StorageError::MigrationFailed {
                version,
                reason: e.to_string(),
            })?;

        tx.execute_batch(sql)
            .map_err(|e| StorageError::MigrationFailed {
                version,
                reason: e.to_string(),
            })?;

        tx.execute(
            "INSERT OR IGNORE INTO _migrations (version, applied_at) VALUES (?1, ?2)",
            [version as i64, Self::now()],
        )
        .map_err(|e| StorageError::MigrationFailed {
            version,
            reason: e.to_string(),
        })?;

        tx.commit().map_err(|e| StorageError::MigrationFailed {
            version,
            reason: e.to_string(),
        })?;

        Ok(())
    }

    fn add_favorite_columns(&mut self) -> Result<(), StorageError> {
        let has_favorite = match self.conn.query_row(
            "SELECT 1 FROM pragma_table_info('urls') WHERE name = 'favorite'",
            [],
            |_| Ok(true),
        ) {
            Ok(v) => v,
            Err(rusqlite::Error::QueryReturnedNoRows) => false,
            Err(e) => {
                return Err(StorageError::MigrationFailed {
                    version: 2,
                    reason: e.to_string(),
                });
            }
        };

        if !has_favorite {
            self.conn
                .execute(
                    "ALTER TABLE urls ADD COLUMN favorite INTEGER NOT NULL DEFAULT 0",
                    [],
                )
                .map_err(|e| StorageError::MigrationFailed {
                    version: 2,
                    reason: e.to_string(),
                })?;

            self.conn
                .execute(
                    "ALTER TABLE urls ADD COLUMN favorite_order INTEGER NOT NULL DEFAULT 0",
                    [],
                )
                .map_err(|e| StorageError::MigrationFailed {
                    version: 2,
                    reason: e.to_string(),
                })?;
        }

        self.conn
            .execute(
                "INSERT OR IGNORE INTO _migrations (version, applied_at) VALUES (?1, ?2)",
                [2i64, Self::now()],
            )
            .map_err(|e| StorageError::MigrationFailed {
                version: 2,
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

    #[test]
    fn migration_2_adds_favorite_columns_to_legacy_schema() {
        let mut conn = Connection::open_in_memory().unwrap();

        // Simulate legacy schema (migration 1 without favorite columns)
        conn.execute_batch(
            "CREATE TABLE _migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL);
             INSERT INTO _migrations (version, applied_at) VALUES (1, 0);
             CREATE TABLE urls (
                 id INTEGER PRIMARY KEY,
                 canonical_url TEXT NOT NULL UNIQUE,
                 original_url TEXT NOT NULL,
                 title TEXT,
                 favicon_path TEXT,
                 created_at INTEGER NOT NULL,
                 updated_at INTEGER NOT NULL,
                 archived INTEGER NOT NULL DEFAULT 0,
                 pinned INTEGER NOT NULL DEFAULT 0
             );",
        )
        .unwrap();

        let mut runner = MigrationRunner::new(&mut conn);
        runner.run_all().unwrap();

        let has_favorite: bool = conn
            .query_row(
                "SELECT 1 FROM pragma_table_info('urls') WHERE name = 'favorite'",
                [],
                |_| Ok(true),
            )
            .unwrap();
        assert!(has_favorite);

        let has_favorite_order: bool = conn
            .query_row(
                "SELECT 1 FROM pragma_table_info('urls') WHERE name = 'favorite_order'",
                [],
                |_| Ok(true),
            )
            .unwrap();
        assert!(has_favorite_order);
    }
}
