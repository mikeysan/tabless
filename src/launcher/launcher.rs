use std::process::Child;

use super::ChildReaper;
use super::error::LaunchError;
use super::identity::BrowserIdentity;
use super::info::BrowserInfo;
use super::platform::PlatformBrowser;
use super::registry::BrowserRegistry;

pub struct Launcher<P: PlatformBrowser> {
    platform: P,
    registry: BrowserRegistry,
    pub(super) reaper: ChildReaper,
}

impl<P: PlatformBrowser> Launcher<P> {
    pub fn new(platform: P, discovered: Vec<BrowserInfo>) -> Self {
        let registry = BrowserRegistry::new(discovered);
        Launcher {
            platform,
            registry,
            reaper: ChildReaper::new(),
        }
    }

    pub fn registry(&self) -> &BrowserRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut BrowserRegistry {
        &mut self.registry
    }

    /// Launch a URL in the system's default browser.
    ///
    /// This delegates directly to the operating system's native URL-opening
    /// mechanism (e.g. `xdg-open`, `open`, or `ShellExecute`) and does not
    /// require browser discovery to succeed. Discovery failures can never
    /// prevent a normal URL launch.
    pub fn launch(&self, url: &str) -> Result<(), LaunchError> {
        self.platform.open_default(url)
    }

    /// Launch a URL in a specific browser by identity.
    pub fn launch_with_identity(
        &self,
        identity: &BrowserIdentity,
        url: &str,
    ) -> Result<Child, LaunchError> {
        let browser = self
            .registry
            .find(identity)
            .ok_or_else(|| LaunchError::BrowserNotFound {
                identity: identity.clone(),
            })?;
        self.launch_in_browser(browser, url)
    }

    fn launch_in_browser(&self, browser: &BrowserInfo, url: &str) -> Result<Child, LaunchError> {
        match self.platform.is_browser_running(browser) {
            Ok(true) => self.platform.launch_new_tab(browser, url),
            Ok(false) => self.platform.launch_url(browser, url),
            Err(_) => {
                // Detection failed — don't trust it, fall back to plain launch.
                self.platform.launch_url(browser, url)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launcher::DiscoveryError;
    use crate::launcher::mock::MockPlatform;
    use std::io;
    use std::path::PathBuf;

    fn make_info(identity: BrowserIdentity) -> BrowserInfo {
        BrowserInfo {
            identity,
            executable_path: PathBuf::from("/usr/bin/mock"),
            version: None,
            is_default: false,
        }
    }

    fn make_launcher() -> Launcher<MockPlatform> {
        let browsers = vec![
            make_info(BrowserIdentity::Firefox),
            make_info(BrowserIdentity::Chrome),
        ];
        let platform = MockPlatform::new(browsers.clone());
        Launcher::new(platform, browsers)
    }

    #[test]
    fn launch_uses_open_default_even_with_empty_registry() {
        let launcher = Launcher::new(MockPlatform::new(vec![]), vec![]);
        let result = launcher.launch("https://example.com");
        assert!(
            matches!(result, Err(LaunchError::SpawnFailed { ref source }) if source.contains("open_default")),
            "expected open_default to be called when registry is empty, got: {:?}",
            result
        );
    }

    #[test]
    fn launch_succeeds_when_open_default_succeeds() {
        struct SucceedingPlatform;

        impl PlatformBrowser for SucceedingPlatform {
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
            fn launch_new_tab(
                &self,
                _info: &BrowserInfo,
                _url: &str,
            ) -> Result<Child, LaunchError> {
                Err(LaunchError::SpawnFailed {
                    source: "dummy".to_string(),
                })
            }
            fn open_default(&self, _url: &str) -> Result<(), LaunchError> {
                Ok(())
            }
        }

        let launcher = Launcher::new(SucceedingPlatform, vec![]);
        let result = launcher.launch("https://example.com");
        assert!(
            result.is_ok(),
            "expected launch to succeed via open_default, got: {:?}",
            result
        );
    }

    #[test]
    fn launch_with_identity_uses_launch_url_when_not_running() {
        let launcher = make_launcher();
        let result =
            launcher.launch_with_identity(&BrowserIdentity::Firefox, "https://example.com");
        assert!(
            matches!(result, Err(LaunchError::SpawnFailed { ref source }) if source.contains("launch_url")),
            "expected launch_url to be called when browser is not running, got: {:?}",
            result
        );
    }

    #[test]
    fn launch_with_identity_uses_new_tab_when_running() {
        let browsers = vec![make_info(BrowserIdentity::Firefox)];
        let mut platform = MockPlatform::new(browsers.clone());
        platform.set_running(BrowserIdentity::Firefox, true);
        let launcher = Launcher::new(platform, browsers);
        let result =
            launcher.launch_with_identity(&BrowserIdentity::Firefox, "https://example.com");
        assert!(
            matches!(result, Err(LaunchError::SpawnFailed { ref source }) if source.contains("launch_new_tab")),
            "expected launch_new_tab to be called when browser is running, got: {:?}",
            result
        );
    }

    #[test]
    fn launch_falls_back_to_launch_url_when_detection_fails() {
        let browsers = vec![make_info(BrowserIdentity::Firefox)];
        let platform = MockPlatform::new(browsers.clone());
        let launcher = Launcher::new(platform, browsers);
        let result =
            launcher.launch_with_identity(&BrowserIdentity::Firefox, "https://example.com");
        assert!(
            matches!(result, Err(LaunchError::SpawnFailed { source }) if source.contains("launch_url"))
        );
    }

    #[test]
    fn launch_with_identity_fails_for_unknown_browser() {
        let launcher = make_launcher();
        let result = launcher.launch_with_identity(&BrowserIdentity::Zen, "https://example.com");
        assert!(
            matches!(result, Err(LaunchError::BrowserNotFound { identity }) if identity == BrowserIdentity::Zen)
        );
    }
}
