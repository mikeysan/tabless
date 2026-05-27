use std::io;
use std::path::PathBuf;
use std::process::Child;

use super::error::{DiscoveryError, LaunchError};
use super::identity::BrowserIdentity;
use super::info::BrowserInfo;
use super::platform::PlatformBrowser;

/// Windows-specific browser discovery and launching.
pub struct WindowsBrowser;

impl WindowsBrowser {
    pub fn new() -> Self {
        WindowsBrowser
    }

    /// Known registry subkeys under `SOFTWARE\Clients\StartMenuInternet`.
    fn known_registry_names() -> Vec<(&'static str, BrowserIdentity)> {
        vec![
            ("Brave", BrowserIdentity::Brave),
            ("FIREFOX", BrowserIdentity::Firefox),
            ("ZEN", BrowserIdentity::Zen),
            ("LibreWolf", BrowserIdentity::LibreWolf),
            ("Google Chrome", BrowserIdentity::Chrome),
        ]
    }

    fn discover_from_registry() -> Vec<BrowserInfo> {
        let mut found = Vec::new();

        for (reg_name, identity) in Self::known_registry_names() {
            for hive in &["HKEY_LOCAL_MACHINE", "HKEY_CURRENT_USER"] {
                let key = format!(
                    "{}\\SOFTWARE\\Clients\\StartMenuInternet\\{}\\shell\\open\\command",
                    hive, reg_name
                );
                if let Ok(output) = std::process::Command::new("reg")
                    .arg("query")
                    .arg(&key)
                    .arg("/ve")
                    .output()
                {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        if let Some(line) = stdout.lines().find(|l| l.contains("(Default)")) {
                            if let Some(val) = line.split("REG_SZ").nth(1) {
                                let path = val.trim().trim_matches('"').to_string();
                                if !path.is_empty() {
                                    found.push(BrowserInfo {
                                        identity: identity.clone(),
                                        executable_path: PathBuf::from(path),
                                        version: None,
                                        is_default: false,
                                    });
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        found
    }
}

impl PlatformBrowser for WindowsBrowser {
    fn discover_browsers(&self) -> Result<Vec<BrowserInfo>, DiscoveryError> {
        let mut browsers = Self::discover_from_registry();

        if let Ok(output) = std::process::Command::new("reg")
            .arg("query")
            .arg("HKEY_CLASSES_ROOT\\http\\shell\\open\\command")
            .arg("/ve")
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
                for browser in &mut browsers {
                    let name = format!("{:?}", browser.identity).to_lowercase();
                    if stdout.contains(&name) {
                        browser.is_default = true;
                    }
                }
            }
        }

        Ok(browsers)
    }

    fn is_browser_running(&self, info: &BrowserInfo) -> Result<bool, io::Error> {
        let exe_name = info
            .executable_path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if exe_name.is_empty() {
            return Ok(false);
        }

        let output = std::process::Command::new("tasklist")
            .arg("/FI")
            .arg(format!("IMAGENAME eq {}.exe", exe_name))
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.contains(&format!("{}.exe", exe_name)))
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
        self.launch_url(info, url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_registry_names_includes_firefox() {
        let names = WindowsBrowser::known_registry_names();
        let reg_names: Vec<_> = names.iter().map(|(n, _)| *n).collect();
        assert!(reg_names.contains(&"FIREFOX"));
        assert!(reg_names.contains(&"Google Chrome"));
    }

    #[test]
    fn windows_browser_is_platform_browser() {
        let _browser: Box<dyn PlatformBrowser> = Box::new(WindowsBrowser::new());
    }
}
