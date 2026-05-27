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
