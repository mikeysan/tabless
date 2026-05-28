use std::collections::HashSet;
use std::io;
use std::process::Child;

use super::error::{DiscoveryError, LaunchError};
use super::identity::BrowserIdentity;
use super::info::BrowserInfo;
use super::platform::PlatformBrowser;

/// Linux-specific browser discovery and launching.
pub struct LinuxBrowser;

impl Default for LinuxBrowser {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxBrowser {
    pub fn new() -> Self {
        LinuxBrowser
    }

    /// Known executable names mapped to BrowserIdentity.
    fn known_executables() -> Vec<(&'static str, BrowserIdentity)> {
        vec![
            ("brave", BrowserIdentity::Brave),
            ("firefox", BrowserIdentity::Firefox),
            ("zen", BrowserIdentity::Zen),
            ("librewolf", BrowserIdentity::LibreWolf),
            ("google-chrome-stable", BrowserIdentity::Chrome),
            ("google-chrome", BrowserIdentity::Chrome),
            ("chrome", BrowserIdentity::Chrome),
        ]
    }

    fn discover_from_path() -> Vec<BrowserInfo> {
        let mut found = Vec::new();
        let mut seen = HashSet::new();

        for (name, identity) in Self::known_executables() {
            if seen.contains(&identity) {
                continue;
            }
            if let Ok(path) = which::which(name) {
                seen.insert(identity.clone());
                found.push(BrowserInfo {
                    identity,
                    executable_path: path,
                    version: None,
                    is_default: false,
                });
            }
        }

        found
    }
}

impl PlatformBrowser for LinuxBrowser {
    fn discover_browsers(&self) -> Result<Vec<BrowserInfo>, DiscoveryError> {
        let mut browsers = Self::discover_from_path();

        // Mark default if we can detect it via xdg-settings
        let output = match std::process::Command::new("xdg-settings")
            .arg("get")
            .arg("default-web-browser")
            .output()
        {
            Ok(o) if o.status.success() => o,
            _ => return Ok(browsers),
        };

        let default = String::from_utf8_lossy(&output.stdout);
        let default_lower = default.trim().to_lowercase();
        for browser in &mut browsers {
            let name = format!("{:?}", browser.identity).to_lowercase();
            if default_lower.contains(&name) {
                browser.is_default = true;
            }
        }

        Ok(browsers)
    }

    fn is_browser_running(&self, info: &BrowserInfo) -> Result<bool, io::Error> {
        let exe_name = info
            .executable_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if exe_name.is_empty() {
            return Ok(false);
        }

        let output = std::process::Command::new("pgrep")
            .arg("-x")
            .arg(exe_name)
            .output()?;

        Ok(output.status.success())
    }

    fn launch_url(&self, info: &BrowserInfo, url: &str) -> Result<Child, LaunchError> {
        if !info.executable_path.exists() {
            return Err(LaunchError::InvalidExecutable {
                path: info.executable_path.clone(),
                reason: "path does not exist".to_string(),
            });
        }

        std::process::Command::new(&info.executable_path)
            .arg(url)
            .spawn()
            .map_err(|e| LaunchError::SpawnFailed {
                source: e.to_string(),
            })
    }

    fn launch_new_tab(&self, info: &BrowserInfo, url: &str) -> Result<Child, LaunchError> {
        // On Linux, most browsers open a new tab when given a URL if already running.
        // We use the same command as launch_url.
        self.launch_url(info, url)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn known_executables_includes_firefox() {
        let executables = LinuxBrowser::known_executables();
        let names: Vec<_> = executables.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"firefox"));
        assert!(names.contains(&"brave"));
        assert!(names.contains(&"chrome"));
    }

    #[test]
    fn linux_browser_is_platform_browser() {
        // This test just verifies the trait is implemented and compiles.
        let _browser: Box<dyn PlatformBrowser> = Box::new(LinuxBrowser::new());
    }

    #[test]
    fn launch_url_fails_for_nonexistent_path() {
        let browser = LinuxBrowser::new();
        let info = BrowserInfo {
            identity: BrowserIdentity::Firefox,
            executable_path: PathBuf::from("/does/not/exist"),
            version: None,
            is_default: false,
        };
        let result = browser.launch_url(&info, "https://example.com");
        assert!(
            matches!(result, Err(LaunchError::InvalidExecutable { path, .. }) if path == std::path::Path::new("/does/not/exist"))
        );
    }
}
