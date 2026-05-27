use std::path::Path;

use rusqlite::Connection;

use super::error::StorageError;

/// Open a SQLite connection at `path` and enable foreign key enforcement.
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
