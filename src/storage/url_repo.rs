use rusqlite::{Connection, OptionalExtension, params};

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

    pub fn insert(&self, url: &ValidatedUrl, title: Option<&str>) -> Result<i64, StorageError> {
        let now = Self::now();
        self.conn
            .execute(
                "INSERT INTO urls (canonical_url, original_url, title, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(canonical_url) DO UPDATE SET updated_at = excluded.updated_at",
                params![url.canonical(), url.original(), title, now, now],
            )
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        let id: i64 = self
            .conn
            .query_row(
                "SELECT id FROM urls WHERE canonical_url = ?1",
                [url.canonical()],
                |row| row.get(0),
            )
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        Ok(id)
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

    pub fn list_main(&self) -> Result<Vec<UrlRecord>, StorageError> {
        self.list_where("archived = 0 AND pinned = 0", "updated_at DESC")
    }

    pub fn list_archived(&self) -> Result<Vec<UrlRecord>, StorageError> {
        self.list_where("archived = 1", "updated_at DESC")
    }

    pub fn list_favorites(&self) -> Result<Vec<UrlRecord>, StorageError> {
        self.list_where("archived = 0 AND pinned = 1", "updated_at DESC")
    }

    fn list_where(&self, condition: &str, order_by: &str) -> Result<Vec<UrlRecord>, StorageError> {
        let sql = format!(
            "SELECT id, canonical_url, original_url, title, favicon_path,
                    created_at, updated_at, archived, pinned
             FROM urls WHERE {} ORDER BY {}",
            condition, order_by
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

    pub fn set_archived(&self, id: i64, archived: bool) -> Result<(), StorageError> {
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

    pub fn set_pinned(&self, id: i64, pinned: bool) -> Result<(), StorageError> {
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

    pub fn delete(&self, id: i64) -> Result<(), StorageError> {
        self.conn
            .execute("DELETE FROM urls WHERE id = ?1", [id])
            .map_err(|e| StorageError::QueryFailed {
                reason: e.to_string(),
            })?;
        Ok(())
    }

    pub fn exists(&self, canonical: &str) -> Result<bool, StorageError> {
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

    pub fn search_fts(&self, query: &str) -> Result<Vec<UrlRecord>, StorageError> {
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

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::storage::migrations::MigrationRunner;
    use crate::url::ValidatedUrl;

    fn setup() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        let mut runner = MigrationRunner::new(&mut conn);
        runner.run_all().unwrap();
        conn
    }

    #[test]
    fn insert_returns_id() {
        let conn = setup();
        let repo = UrlRepository::new(&conn);
        let url = ValidatedUrl::parse("https://example.com").unwrap();
        let id = repo.insert(&url, None).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn find_missing_returns_none() {
        let conn = setup();
        let repo = UrlRepository::new(&conn);
        let result = repo.find_by_id(999).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_main_excludes_archived_and_pinned() {
        let conn = setup();
        let repo = UrlRepository::new(&conn);
        let url = ValidatedUrl::parse("https://example.com").unwrap();
        let id = repo.insert(&url, None).unwrap();
        repo.set_archived(id, true).unwrap();
        let main = repo.list_main().unwrap();
        assert!(main.is_empty());

        // pinned should also be excluded from main
        let url2 = ValidatedUrl::parse("https://rust-lang.org").unwrap();
        let id2 = repo.insert(&url2, None).unwrap();
        repo.set_pinned(id2, true).unwrap();
        let main2 = repo.list_main().unwrap();
        assert!(main2.is_empty());
    }

    #[test]
    fn insert_duplicate_bumps_updated_at() {
        let conn = setup();
        let repo = UrlRepository::new(&conn);
        let url = ValidatedUrl::parse("https://example.com").unwrap();
        let id1 = repo.insert(&url, Some("First")).unwrap();

        std::thread::sleep(std::time::Duration::from_secs(1));

        let id2 = repo.insert(&url, Some("Second")).unwrap();
        assert_eq!(id1, id2);

        let record = repo.find_by_id(id1).unwrap().unwrap();
        assert!(record.updated_at > record.created_at);
    }

    #[test]
    fn exists_false_for_unknown() {
        let conn = setup();
        let repo = UrlRepository::new(&conn);
        assert!(!repo.exists("https://unknown.com/").unwrap());
    }
}
