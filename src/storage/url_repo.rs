use rusqlite::{params, Connection, OptionalExtension};

use crate::url::ValidatedUrl;

use super::error::StorageError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlRecord {
    pub id: i64,
    pub canonical_url: String,
    pub original_url: String,
    pub title: Option<String>,
    pub favicon_path: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub archived: bool,
    pub pinned: bool,
}

pub struct UrlRepository<'a> {
    conn: &'a Connection,
}

impl<'a> UrlRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        UrlRepository { conn }
    }

    pub fn insert(
        &self,
        url: &ValidatedUrl,
        title: Option<&str>,
    ) -> Result<i64, StorageError> {
        let now = Self::now();
        self.conn
            .execute(
                "INSERT INTO urls (canonical_url, original_url, title, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![url.canonical(), url.original(), title, now, now],
            )
            .map_err(|e| {
                if e.to_string().contains("UNIQUE constraint failed") {
                    StorageError::ConstraintViolation {
                        table: "urls".to_string(),
                        reason: "duplicate canonical_url".to_string(),
                    }
                } else {
                    StorageError::QueryFailed {
                        reason: e.to_string(),
                    }
                }
            })?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn find_by_id(&self, id: i64) -> Result<Option<UrlRecord>, StorageError> {
        self.conn
            .query_row(
                "SELECT id, canonical_url, original_url, title, favicon_path,
                        created_at, updated_at, archived, pinned
                 FROM urls WHERE id = ?1",
                [id],
                |row| {
                    Ok(UrlRecord {
                        id: row.get(0)?,
                        canonical_url: row.get(1)?,
                        original_url: row.get(2)?,
                        title: row.get(3)?,
                        favicon_path: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                        archived: row.get::<_, i64>(7)? != 0,
                        pinned: row.get::<_, i64>(8)? != 0,
                    })
                },
            )
            .optional()
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })
    }

    pub fn list_inbox(&self) -> Result<Vec<UrlRecord>, StorageError> {
        self.list_where("archived = 0")
    }

    pub fn list_archived(&self) -> Result<Vec<UrlRecord>, StorageError> {
        self.list_where("archived = 1")
    }

    pub fn list_pinned(&self) -> Result<Vec<UrlRecord>, StorageError> {
        self.list_where("pinned = 1")
    }

    fn list_where(
        &self,
        condition: &str,
    ) -> Result<Vec<UrlRecord>, StorageError> {
        let sql = format!(
            "SELECT id, canonical_url, original_url, title, favicon_path,
                    created_at, updated_at, archived, pinned
             FROM urls WHERE {} ORDER BY created_at DESC",
            condition
        );
        let mut stmt = self
            .conn
            .prepare(&sql)
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        let rows = stmt
            .query_map([], |row| {
                Ok(UrlRecord {
                    id: row.get(0)?,
                    canonical_url: row.get(1)?,
                    original_url: row.get(2)?,
                    title: row.get(3)?,
                    favicon_path: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    archived: row.get::<_, i64>(7)? != 0,
                    pinned: row.get::<_, i64>(8)? != 0,
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

    pub fn set_archived(
        &self,
        id: i64,
        archived: bool,
    ) -> Result<(), StorageError> {
        self.conn
            .execute(
                "UPDATE urls SET archived = ?1, updated_at = ?2 WHERE id = ?3",
                params![archived as i64, Self::now(), id],
            )
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        Ok(())
    }

    pub fn set_pinned(
        &self,
        id: i64,
        pinned: bool,
    ) -> Result<(), StorageError> {
        self.conn
            .execute(
                "UPDATE urls SET pinned = ?1, updated_at = ?2 WHERE id = ?3",
                params![pinned as i64, Self::now(), id],
            )
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        Ok(())
    }

    pub fn delete(&self,
        id: i64,
    ) -> Result<(), StorageError> {
        self.conn
            .execute("DELETE FROM urls WHERE id = ?1", [id])
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        Ok(())
    }

    pub fn exists(&self,
        canonical: &str,
    ) -> Result<bool, StorageError> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM urls WHERE canonical_url = ?1",
                [canonical],
                |row| row.get(0),
            )
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        Ok(count > 0)
    }

    pub fn search_fts(
        &self,
        query: &str,
    ) -> Result<Vec<UrlRecord>, StorageError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT u.id, u.canonical_url, u.original_url, u.title, u.favicon_path,
                        u.created_at, u.updated_at, u.archived, u.pinned
                 FROM urls u
                 JOIN fts_urls f ON u.id = f.rowid
                 WHERE fts_urls MATCH ?1
                 ORDER BY rank",
            )
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        let rows = stmt
            .query_map([query], |row| {
                Ok(UrlRecord {
                    id: row.get(0)?,
                    canonical_url: row.get(1)?,
                    original_url: row.get(2)?,
                    title: row.get(3)?,
                    favicon_path: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    archived: row.get::<_, i64>(7)? != 0,
                    pinned: row.get::<_, i64>(8)? != 0,
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

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }
}
