use std::process::Child;

use super::error::LaunchError;
use super::identity::BrowserIdentity;
use super::info::BrowserInfo;
use super::platform::PlatformBrowser;
use super::registry::BrowserRegistry;

pub struct Launcher<P: PlatformBrowser> {
    platform: P,
    registry: BrowserRegistry,
}

impl<P: PlatformBrowser> Launcher<P> {
    pub fn new(platform: P, discovered: Vec<BrowserInfo>) -> Self {
        let registry = BrowserRegistry::new(discovered);
        Launcher { platform, registry }
    }

    pub fn registry(&self) -> &BrowserRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut BrowserRegistry {
        &mut self.registry
    }

    /// Launch a URL in the user's preferred browser.
    pub fn launch(&self, url: &str) -> Result<Child, LaunchError> {
        let browser = self
            .registry
            .preferred_browser()
            .ok_or_else(|| LaunchError::BrowserNotFound {
                identity: BrowserIdentity::Custom("preferred".to_string()),
            })?;
        self.launch_in_browser(browser, url)
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

    fn launch_in_browser(&self, browser: &BrowserInfo, url: &str
    ) -> Result<Child, LaunchError> {
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
    use crate::launcher::mock::MockPlatform;
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
    fn launch_fails_when_no_preferred_set() {
        let launcher = make_launcher();
        let result = launcher.launch("https://example.com");
        assert!(matches!(result, Err(LaunchError::BrowserNotFound { .. })));
    }

    #[test]
    fn launch_with_identity_uses_launch_url_when_not_running() {
        let launcher = make_launcher();
        let result = launcher.launch_with_identity(
            &BrowserIdentity::Firefox,
            "https://example.com",
        );
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
        let result = launcher.launch_with_identity(
            &BrowserIdentity::Firefox,
            "https://example.com",
        );
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
        let result = launcher.launch_with_identity(
            &BrowserIdentity::Firefox,
            "https://example.com",
        );
        assert!(
            matches!(result, Err(LaunchError::SpawnFailed { source }) if source.contains("launch_url"))
        );
    }

    #[test]
    fn launch_with_identity_fails_for_unknown_browser() {
        let launcher = make_launcher();
        let result = launcher.launch_with_identity(
            &BrowserIdentity::Zen,
            "https://example.com",
        );
        assert!(
            matches!(result, Err(LaunchError::BrowserNotFound { identity }) if identity == BrowserIdentity::Zen)
        );
    }
}
