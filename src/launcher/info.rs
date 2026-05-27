use std::path::PathBuf;

use super::identity::BrowserIdentity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserInfo {
    pub identity: BrowserIdentity,
    pub executable_path: PathBuf,
    pub version: Option<String>,
    pub is_default: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construct_browser_info() {
        let info = BrowserInfo {
            identity: BrowserIdentity::Firefox,
            executable_path: PathBuf::from("/usr/bin/firefox"),
            version: Some("124.0".to_string()),
            is_default: true,
        };
        assert_eq!(info.identity, BrowserIdentity::Firefox);
        assert_eq!(info.executable_path, PathBuf::from("/usr/bin/firefox"));
        assert_eq!(info.version, Some("124.0".to_string()));
        assert!(info.is_default);
    }

    #[test]
    fn browser_info_equality() {
        let a = BrowserInfo {
            identity: BrowserIdentity::Chrome,
            executable_path: PathBuf::from("/usr/bin/google-chrome"),
            version: None,
            is_default: false,
        };
        let b = BrowserInfo {
            identity: BrowserIdentity::Chrome,
            executable_path: PathBuf::from("/usr/bin/google-chrome"),
            version: None,
            is_default: false,
        };
        assert_eq!(a, b);
    }
}
