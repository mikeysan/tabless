use std::fmt;

use crate::storage::error::StorageError;
use crate::url::error::UrlValidationError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    InvalidUrl { reason: String },
    UrlValidationFailed { source: UrlValidationError },
    StorageFailed { source: StorageError },
    IpcBindFailed { reason: String },
    IpcConnectFailed { reason: String },
    RegistrationFailed { platform: String, reason: String },
    AlreadyRegistered,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtocolError::InvalidUrl { reason } => write!(f, "invalid protocol URL: {}", reason),
            ProtocolError::UrlValidationFailed { source } => {
                write!(f, "URL validation failed: {}", source)
            }
            ProtocolError::StorageFailed { source } => {
                write!(f, "storage error: {}", source)
            }
            ProtocolError::IpcBindFailed { reason } => {
                write!(f, "IPC bind failed: {}", reason)
            }
            ProtocolError::IpcConnectFailed { reason } => {
                write!(f, "IPC connect failed: {}", reason)
            }
            ProtocolError::RegistrationFailed { platform, reason } => {
                write!(f, "protocol registration failed on {}: {}", platform, reason)
            }
            ProtocolError::AlreadyRegistered => {
                write!(f, "protocol already registered")
            }
        }
    }
}

impl std::error::Error for ProtocolError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProtocolError::UrlValidationFailed { source } => Some(source),
            ProtocolError::StorageFailed { source } => Some(source),
            _ => None,
        }
    }
}
