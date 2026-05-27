use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    ConnectionFailed { reason: String },
    MigrationFailed { version: u32, reason: String },
    ConstraintViolation { table: String, reason: String },
    NotFound { table: String, id: i64 },
    QueryFailed { reason: String },
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::ConnectionFailed { reason } => {
                write!(f, "database connection failed: {}", reason)
            }
            StorageError::MigrationFailed { version, reason } => {
                write!(f, "migration {} failed: {}", version, reason)
            }
            StorageError::ConstraintViolation { table, reason } => {
                write!(f, "constraint violation in {}: {}", table, reason)
            }
            StorageError::NotFound { table, id } => {
                write!(f, "record not found in {} with id {}", table, id)
            }
            StorageError::QueryFailed { reason } => {
                write!(f, "query failed: {}", reason)
            }
        }
    }
}

impl std::error::Error for StorageError {}
