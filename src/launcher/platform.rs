use std::io;
use std::process::Child;

use super::error::{DiscoveryError, LaunchError};
use super::info::BrowserInfo;

pub trait PlatformBrowser: Send + Sync {
    fn discover_browsers(&self) -> Result<Vec<BrowserInfo>, DiscoveryError>;
    fn is_browser_running(&self, info: &BrowserInfo) -> Result<bool, io::Error>;
    fn launch_url(&self, info: &BrowserInfo, url: &str) -> Result<Child, LaunchError>;
    fn launch_new_tab(&self, info: &BrowserInfo, url: &str) -> Result<Child, LaunchError>;

    /// Open a URL in the system's default browser without requiring discovery.
    ///
    /// This delegates to the operating system's native URL-opening mechanism
    /// (e.g. `xdg-open`, `open`, or `ShellExecute`) and must succeed whenever
    /// the OS itself can open URLs.
    fn open_default(&self, url: &str) -> Result<(), LaunchError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyPlatform;

    impl PlatformBrowser for DummyPlatform {
        fn discover_browsers(&self) -> Result<Vec<BrowserInfo>, DiscoveryError> {
            Ok(vec![])
        }

        fn is_browser_running(&self, _info: &BrowserInfo) -> Result<bool, io::Error> {
            Ok(false)
        }

        fn launch_url(&self, _info: &BrowserInfo, _url: &str) -> Result<Child, LaunchError> {
            Err(LaunchError::SpawnFailed {
                source: "dummy".to_string(),
            })
        }

        fn launch_new_tab(&self, _info: &BrowserInfo, _url: &str) -> Result<Child, LaunchError> {
            Err(LaunchError::SpawnFailed {
                source: "dummy".to_string(),
            })
        }

        fn open_default(&self, _url: &str) -> Result<(), LaunchError> {
            Ok(())
        }
    }

    #[test]
    fn dummy_platform_implements_trait() {
        let dummy = DummyPlatform;
        let browsers = dummy.discover_browsers().unwrap();
        assert!(browsers.is_empty());
    }
}
