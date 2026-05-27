pub mod collection_repo;
pub mod connection;
pub mod error;
pub mod migrations;
pub mod search;
pub mod tag_repo;
pub mod url_repo;

pub use collection_repo::{CollectionRecord, CollectionRepository};
pub use error::StorageError;
pub use search::{FuzzySearchIndex, SearchResult};
pub use tag_repo::{TagRecord, TagRepository};
pub use url_repo::{UrlRecord, UrlRepository};

use std::path::Path;

use rusqlite::Connection;

pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        let mut conn = connection::open_connection(path)?;
        let mut runner = migrations::MigrationRunner::new(&mut conn);
        runner.run_all()?;
        Ok(Storage { conn })
    }

    pub fn urls(&self) -> UrlRepository<'_> {
        UrlRepository::new(&self.conn)
    }

    pub fn tags(&self) -> TagRepository<'_> {
        TagRepository::new(&self.conn)
    }

    pub fn collections(&self) -> CollectionRepository<'_> {
        CollectionRepository::new(&self.conn)
    }
}
