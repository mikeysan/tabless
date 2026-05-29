use std::io;
use std::path::{Path, PathBuf};
use std::process::Child;

use super::error::{DiscoveryError, LaunchError};
use super::identity::BrowserIdentity;
use super::info::BrowserInfo;
use super::platform::PlatformBrowser;

/// macOS-specific browser discovery and launching.
pub struct MacBrowser;

impl Default for MacBrowser {
    fn default() -> Self {
        Self::new()
    }
}

impl MacBrowser {
    pub fn new() -> Self {
        MacBrowser
    }

    /// Known bundle identifiers mapped to BrowserIdentity.
    fn known_bundle_ids() -> Vec<(&'static str, BrowserIdentity)> {
        vec![
            ("com.brave.Browser", BrowserIdentity::Brave),
            ("org.mozilla.firefox", BrowserIdentity::Firefox),
            ("net.zen-browser.zen", BrowserIdentity::Zen),
            ("io.gitlab.librewolf-community", BrowserIdentity::LibreWolf),
            ("com.google.Chrome", BrowserIdentity::Chrome),
        ]
    }

    fn discover_from_applications() -> Vec<BrowserInfo> {
        let mut found = Vec::new();

        for (bundle_id, identity) in Self::known_bundle_ids() {
            let app_path = format!(
                "/Applications/{}.app",
                Self::app_name_for_identity(&identity)
            );
            let app_path_buf = PathBuf::from(&app_path);
            if app_path_buf.exists() {
                let executable = Self::find_executable_in_bundle(&app_path_buf);
                found.push(BrowserInfo {
                    identity: identity.clone(),
                    executable_path: executable.unwrap_or(app_path_buf),
                    version: None,
                    is_default: false,
                });
            } else {
                let output = match std::process::Command::new("mdfind")
                    .arg(format!("kMDItemCFBundleIdentifier == '{}'", bundle_id))
                    .output()
                {
                    Ok(o) if o.status.success() => o,
                    _ => continue,
                };
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if path.is_empty() {
                    continue;
                }
                let path_buf = PathBuf::from(path);
                let executable = Self::find_executable_in_bundle(&path_buf);
                found.push(BrowserInfo {
                    identity: identity.clone(),
                    executable_path: executable.unwrap_or(path_buf),
                    version: None,
                    is_default: false,
                });
            }
        }

        found
    }

    fn app_name_for_identity(identity: &BrowserIdentity) -> &'static str {
        match identity {
            BrowserIdentity::Brave => "Brave Browser",
            BrowserIdentity::Firefox => "Firefox",
            BrowserIdentity::Zen => "Zen Browser",
            BrowserIdentity::LibreWolf => "LibreWolf",
            BrowserIdentity::Chrome => "Google Chrome",
            BrowserIdentity::Custom(_) => "",
        }
    }

    fn find_executable_in_bundle(bundle_path: &Path) -> Option<PathBuf> {
        let contents = bundle_path.join("Contents/MacOS");
        if let Ok(entries) = std::fs::read_dir(contents) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    return Some(path);
                }
            }
        }
        None
    }
}

impl PlatformBrowser for MacBrowser {
    fn discover_browsers(&self) -> Result<Vec<BrowserInfo>, DiscoveryError> {
        let mut browsers = Self::discover_from_applications();

        if let Ok(output) = std::process::Command::new("defaults")
            .arg("read")
            .arg("com.apple.LaunchServices/com.apple.launchservices.secure")
            .arg("LSHandlers")
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for browser in &mut browsers {
                let bundle = format!("{:?}", browser.identity);
                if stdout.to_lowercase().contains(&bundle.to_lowercase()) {
                    browser.is_default = true;
                }
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

        std::process::Command::new("open")
            .arg("-a")
            .arg(&info.executable_path)
            .arg(url)
            .spawn()
            .map_err(|e| LaunchError::SpawnFailed {
                source: e.to_string(),
            })
    }

    fn launch_new_tab(&self, info: &BrowserInfo, url: &str) -> Result<Child, LaunchError> {
        self.launch_url(info, url)
    }

    fn open_default(&self, url: &str) -> Result<(), LaunchError> {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|e| LaunchError::SpawnFailed {
                source: e.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_bundle_ids_includes_firefox() {
        let ids = MacBrowser::known_bundle_ids();
        let bundles: Vec<_> = ids.iter().map(|(id, _)| *id).collect();
        assert!(bundles.contains(&"org.mozilla.firefox"));
        assert!(bundles.contains(&"com.google.Chrome"));
    }

    #[test]
    fn app_name_for_identity() {
        assert_eq!(
            MacBrowser::app_name_for_identity(&BrowserIdentity::Firefox),
            "Firefox"
        );
        assert_eq!(
            MacBrowser::app_name_for_identity(&BrowserIdentity::Chrome),
            "Google Chrome"
        );
    }

    #[test]
    fn mac_browser_is_platform_browser() {
        let _browser: Box<dyn PlatformBrowser> = Box::new(MacBrowser::new());
    }
}
