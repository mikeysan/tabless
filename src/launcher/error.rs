use std::fmt;
use std::path::PathBuf;

use super::identity::BrowserIdentity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryError {
    PlatformNotSupported,
    PathNotFound { path: PathBuf },
    PermissionDenied { path: PathBuf },
    ReadFailed { source: String },
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiscoveryError::PlatformNotSupported => {
                write!(f, "browser discovery is not supported on this platform")
            }
            DiscoveryError::PathNotFound { path } => {
                write!(f, "browser path not found: {}", path.display())
            }
            DiscoveryError::PermissionDenied { path } => {
                write!(
                    f,
                    "permission denied reading browser path: {}",
                    path.display()
                )
            }
            DiscoveryError::ReadFailed { source } => {
                write!(f, "failed to read browser discovery data: {}", source)
            }
        }
    }
}

impl std::error::Error for DiscoveryError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchError {
    BrowserNotFound { identity: BrowserIdentity },
    NoPreferredBrowser,
    InvalidExecutable { path: PathBuf, reason: String },
    SpawnFailed { source: String },
    AlreadyRunningButTabFailed,
}

impl fmt::Display for LaunchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LaunchError::BrowserNotFound { identity } => {
                write!(f, "browser not found: {:?}", identity)
            }
            LaunchError::NoPreferredBrowser => {
                write!(f, "no preferred browser configured")
            }
            LaunchError::InvalidExecutable { path, reason } => {
                write!(f, "invalid executable at {}: {}", path.display(), reason)
            }
            LaunchError::SpawnFailed { source } => {
                write!(f, "failed to spawn browser process: {}", source)
            }
            LaunchError::AlreadyRunningButTabFailed => {
                write!(f, "browser is running but could not open a new tab")
            }
        }
    }
}

impl std::error::Error for LaunchError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_error_display() {
        let err = DiscoveryError::PathNotFound {
            path: PathBuf::from("/usr/bin/firefox"),
        };
        assert_eq!(err.to_string(), "browser path not found: /usr/bin/firefox");
    }

    #[test]
    fn launch_error_display_browser_not_found() {
        let err = LaunchError::BrowserNotFound {
            identity: BrowserIdentity::Zen,
        };
        assert!(err.to_string().contains("Zen"));
    }

    #[test]
    fn launch_error_display_invalid_executable() {
        let err = LaunchError::InvalidExecutable {
            path: PathBuf::from("/not/a/browser"),
            reason: "not executable".to_string(),
        };
        assert!(err.to_string().contains("/not/a/browser"));
        assert!(err.to_string().contains("not executable"));
    }
}
