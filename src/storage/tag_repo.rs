use rusqlite::Connection;

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
            .execute("INSERT INTO tags (name) VALUES (?1)", [name])
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
        let mut stmt = self
            .conn
            .prepare(
                "SELECT t.id, t.name FROM tags t
                 JOIN url_tags ut ON t.id = ut.tag_id
                 WHERE ut.url_id = ?1
                 ORDER BY t.name",
            )
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        let rows = stmt
            .query_map([url_id], |row| {
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
}
