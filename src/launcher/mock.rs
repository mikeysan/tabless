use std::collections::HashMap;
use std::io;
use std::process::Child;

use super::error::DiscoveryError;
use super::error::LaunchError;
use super::identity::BrowserIdentity;
use super::info::BrowserInfo;
use super::platform::PlatformBrowser;

/// A test double for PlatformBrowser. Never spawns real processes.
pub struct MockPlatform {
    browsers: Vec<BrowserInfo>,
    running: HashMap<BrowserIdentity, bool>,
}

impl MockPlatform {
    pub fn new(browsers: Vec<BrowserInfo>) -> Self {
        MockPlatform {
            browsers,
            running: HashMap::new(),
        }
    }

    pub fn set_running(&mut self, identity: BrowserIdentity, is_running: bool) {
        self.running.insert(identity, is_running);
    }
}

impl PlatformBrowser for MockPlatform {
    fn discover_browsers(&self) -> Result<Vec<BrowserInfo>, DiscoveryError> {
        Ok(self.browsers.clone())
    }

    fn is_browser_running(&self, info: &BrowserInfo) -> Result<bool, io::Error> {
        Ok(*self.running.get(&info.identity).unwrap_or(&false))
    }

    fn launch_url(&self, _info: &BrowserInfo, _url: &str) -> Result<Child, LaunchError> {
        Err(LaunchError::SpawnFailed {
            source: "mock: launch_url called".to_string(),
        })
    }

    fn launch_new_tab(&self, _info: &BrowserInfo, _url: &str) -> Result<Child, LaunchError> {
        Err(LaunchError::SpawnFailed {
            source: "mock: launch_new_tab called".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_info(identity: BrowserIdentity) -> BrowserInfo {
        BrowserInfo {
            identity,
            executable_path: PathBuf::from("/usr/bin/mock"),
            version: None,
            is_default: false,
        }
    }

    #[test]
    fn mock_discover_returns_browsers() {
        let browsers = vec![make_info(BrowserIdentity::Firefox)];
        let mock = MockPlatform::new(browsers.clone());
        let discovered = mock.discover_browsers().unwrap();
        assert_eq!(discovered.len(), 1);
    }

    #[test]
    fn mock_running_false_by_default() {
        let browsers = vec![make_info(BrowserIdentity::Firefox)];
        let mock = MockPlatform::new(browsers);
        let info = make_info(BrowserIdentity::Firefox);
        assert!(!mock.is_browser_running(&info).unwrap());
    }

    #[test]
    fn mock_running_true_when_set() {
        let browsers = vec![make_info(BrowserIdentity::Firefox)];
        let mut mock = MockPlatform::new(browsers);
        mock.set_running(BrowserIdentity::Firefox, true);
        let info = make_info(BrowserIdentity::Firefox);
        assert!(mock.is_browser_running(&info).unwrap());
    }
}
