use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum UrlValidationError {
    EmptyInput,
    InvalidScheme { found: String },
    MalformedUrl { reason: String },
    EmptyHost,
    InvalidPort { port: u16 },
}

impl fmt::Display for UrlValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UrlValidationError::EmptyInput => write!(f, "URL input is empty"),
            UrlValidationError::InvalidScheme { found } => {
                write!(f, "invalid URL scheme: {}", found)
            }
            UrlValidationError::MalformedUrl { reason } => {
                write!(f, "malformed URL: {}", reason)
            }
            UrlValidationError::EmptyHost => write!(f, "URL has no host"),
            UrlValidationError::InvalidPort { port } => {
                write!(f, "invalid port number: {}", port)
            }
        }
    }
}

impl std::error::Error for UrlValidationError {}
