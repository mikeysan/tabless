use std::io;
use std::path::PathBuf;
use std::process::Child;

use super::error::{DiscoveryError, LaunchError};
use super::identity::BrowserIdentity;
use super::info::BrowserInfo;
use super::platform::PlatformBrowser;

/// Windows-specific browser discovery and launching.
pub struct WindowsBrowser;

impl Default for WindowsBrowser {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsBrowser {
    pub fn new() -> Self {
        WindowsBrowser
    }

    /// Registry hives to search for installed browsers.
    const REG_HIVES: &[&str] = &["HKEY_LOCAL_MACHINE", "HKEY_CURRENT_USER"];

    /// Base registry path for StartMenuInternet browser entries.
    const REG_BASE: &str = "SOFTWARE\\Clients\\StartMenuInternet";

    /// Extract the executable path from a raw registry command string.
    ///
    /// Registry command values typically look like:
    /// - `"C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe" -- "%1"`
    /// - `C:\\Program Files\\BraveSoftware\\Brave-Browser\\Application\\brave.exe --single-argument %1`
    ///
    /// This parser isolates only the executable path, ignoring arguments.
    fn extract_executable(command: &str) -> Option<PathBuf> {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return None;
        }

        if trimmed.starts_with('"') {
            // Quoted path: find the matching closing quote.
            let rest = &trimmed[1..];
            if let Some(end) = rest.find('"') {
                let path = &rest[..end];
                if !path.is_empty() {
                    return Some(PathBuf::from(path));
                }
            }
        } else {
            // Unquoted path: try to locate a known executable extension first
            // (handles malformed but occasionally seen unquoted paths with spaces).
            let lower = trimmed.to_lowercase();
            for ext in [".exe", ".com", ".bat", ".cmd"] {
                if let Some(pos) = lower.find(ext) {
                    let path = &trimmed[..pos + ext.len()];
                    if !path.is_empty() {
                        return Some(PathBuf::from(path));
                    }
                }
            }
            // Fall back to the first whitespace-delimited token.
            let path = trimmed.split_whitespace().next().unwrap_or("");
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }

        None
    }

    /// Guess a `BrowserIdentity` from the registry subkey name and the
    /// executable file name. Falls back to `Custom` so unknown browsers
    /// are never silently dropped.
    fn guess_identity(reg_name: &str, exe_name: &str) -> BrowserIdentity {
        let reg_lower = reg_name.to_lowercase();
        let exe_lower = exe_name.to_lowercase();

        // Check executable name first (most reliable).
        if exe_lower.contains("firefox") {
            return BrowserIdentity::Firefox;
        }
        if exe_lower.contains("librewolf") {
            return BrowserIdentity::LibreWolf;
        }
        if exe_lower.contains("zen") && !exe_lower.contains("chromium") {
            // "zen" alone is ambiguous; avoid matching chrome.exe in a Zen folder.
            return BrowserIdentity::Zen;
        }
        if exe_lower.contains("brave") {
            return BrowserIdentity::Brave;
        }
        if exe_lower.contains("chrome") && !exe_lower.contains("chromium") {
            return BrowserIdentity::Chrome;
        }
        if exe_lower.contains("edge") {
            return BrowserIdentity::Custom("Edge".to_string());
        }
        if exe_lower.contains("opera") {
            return BrowserIdentity::Custom("Opera".to_string());
        }
        if exe_lower.contains("vivaldi") {
            return BrowserIdentity::Custom("Vivaldi".to_string());
        }

        // Fall back to registry key name heuristics.
        if reg_lower.contains("firefox") {
            return BrowserIdentity::Firefox;
        }
        if reg_lower.contains("librewolf") {
            return BrowserIdentity::LibreWolf;
        }
        if reg_lower.contains("zen") {
            return BrowserIdentity::Zen;
        }
        if reg_lower.contains("brave") {
            return BrowserIdentity::Brave;
        }
        if reg_lower.contains("chrome") && !reg_lower.contains("chromium") {
            return BrowserIdentity::Chrome;
        }
        if reg_lower.contains("edge") {
            return BrowserIdentity::Custom("Edge".to_string());
        }

        BrowserIdentity::Custom(reg_name.to_string())
    }

    /// Query the default HTTP handler command from the registry.
    fn default_browser_command() -> Option<String> {
        let output = std::process::Command::new("reg")
            .arg("query")
            .arg(r"HKEY_CLASSES_ROOT\http\shell\open\command")
            .arg("/ve")
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_reg_sz_value(&stdout)
    }

    /// Parse the `(Default) REG_SZ <value>` line produced by `reg query /ve`.
    fn parse_reg_sz_value(stdout: &str) -> Option<String> {
        let line = stdout.lines().find(|l| l.contains("(Default)"))?;
        let val = line.split("REG_SZ").nth(1)?;
        let cleaned = val.trim().to_string();
        if cleaned.is_empty() {
            return None;
        }
        Some(cleaned)
    }

    /// Enumerate all subkeys under `SOFTWARE\Clients\StartMenuInternet`
    /// for both HKLM and HKCU, returning each subkey name along with the
    /// parsed command value.
    fn enumerate_registry_browsers() -> Vec<(String, String)> {
        let mut results = Vec::new();

        for hive in Self::REG_HIVES {
            let key = format!("{}\\{}", hive, Self::REG_BASE);
            let output = match std::process::Command::new("reg")
                .arg("query")
                .arg(&key)
                .output()
            {
                Ok(o) if o.status.success() => o,
                _ => continue,
            };

            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let line = line.trim();
                // Each subkey line looks like:
                // HKEY_LOCAL_MACHINE\SOFTWARE\Clients\StartMenuInternet\FIREFOX
                let Some(subkey) = line.rsplit_once('\\').map(|(_, s)| s) else {
                    continue;
                };
                if subkey.is_empty() || subkey == Self::REG_BASE {
                    continue;
                }

                let command_key = format!("{}\\{}\\shell\\open\\command", key, subkey);
                let cmd_output = match std::process::Command::new("reg")
                    .arg("query")
                    .arg(&command_key)
                    .arg("/ve")
                    .output()
                {
                    Ok(o) if o.status.success() => o,
                    _ => continue,
                };

                let cmd_stdout = String::from_utf8_lossy(&cmd_output.stdout);
                if let Some(command) = Self::parse_reg_sz_value(&cmd_stdout) {
                    results.push((subkey.to_string(), command));
                }
            }
        }

        results
    }

    fn discover_from_registry() -> Vec<BrowserInfo> {
        let mut found = Vec::new();
        let default_cmd = Self::default_browser_command();
        let default_cmd_lower = default_cmd.as_ref().map(|s| s.to_lowercase());

        for (reg_name, command) in Self::enumerate_registry_browsers() {
            let Some(exe_path) = Self::extract_executable(&command) else {
                continue;
            };

            let exe_name = exe_path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let identity = Self::guess_identity(&reg_name, &exe_name);

            // Determine if this browser is the system default by comparing
            // the default command string against this entry's command.
            let is_default = default_cmd_lower
                .as_ref()
                .map(|d| {
                    let cmd_lower = command.to_lowercase();
                    // Match on executable name (most reliable) or full command.
                    d.contains(&exe_name.to_lowercase()) || cmd_lower == *d
                })
                .unwrap_or(false);

            found.push(BrowserInfo {
                identity,
                executable_path: exe_path,
                version: None,
                is_default,
            });
        }

        found
    }
}

impl PlatformBrowser for WindowsBrowser {
    fn discover_browsers(&self) -> Result<Vec<BrowserInfo>, DiscoveryError> {
        let browsers = Self::discover_from_registry();
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

    fn open_default(&self, url: &str) -> Result<(), LaunchError> {
        // "start" treats the first quoted string as a window title, so we
        // pass an empty title to ensure the URL is interpreted as the target.
        std::process::Command::new("cmd")
            .arg("/c")
            .arg("start")
            .arg("")
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
    fn extract_executable_quoted_with_args() {
        let cmd = r#""C:\Program Files\Google\Chrome\Application\chrome.exe" -- "%1""#;
        let result = WindowsBrowser::extract_executable(cmd);
        assert_eq!(
            result,
            Some(PathBuf::from(
                r"C:\Program Files\Google\Chrome\Application\chrome.exe"
            ))
        );
    }

    #[test]
    fn extract_executable_unquoted_with_args() {
        let cmd = r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe --single-argument %1";
        let result = WindowsBrowser::extract_executable(cmd);
        assert_eq!(
            result,
            Some(PathBuf::from(
                r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe"
            ))
        );
    }

    #[test]
    fn extract_executable_quoted_no_args() {
        let cmd = r#""C:\Apps\Firefox\firefox.exe""#;
        let result = WindowsBrowser::extract_executable(cmd);
        assert_eq!(result, Some(PathBuf::from(r"C:\Apps\Firefox\firefox.exe")));
    }

    #[test]
    fn extract_executable_empty() {
        assert!(WindowsBrowser::extract_executable("").is_none());
        assert!(WindowsBrowser::extract_executable("   ").is_none());
    }

    #[test]
    fn extract_executable_no_closing_quote() {
        let cmd = r#""C:\Apps\browser.exe"#;
        // No closing quote means we fall through and take the first token.
        let result = WindowsBrowser::extract_executable(cmd);
        // The string starts with '"' so we enter the quoted branch.
        // `rest.find('"')` returns None, so we return None.
        assert!(result.is_none());
    }

    #[test]
    fn guess_identity_from_exe_name() {
        assert_eq!(
            WindowsBrowser::guess_identity("Foo", "firefox.exe"),
            BrowserIdentity::Firefox
        );
        assert_eq!(
            WindowsBrowser::guess_identity("Foo", "chrome.exe"),
            BrowserIdentity::Chrome
        );
        assert_eq!(
            WindowsBrowser::guess_identity("Foo", "brave.exe"),
            BrowserIdentity::Brave
        );
    }

    #[test]
    fn guess_identity_from_reg_name_fallback() {
        assert_eq!(
            WindowsBrowser::guess_identity("Google Chrome", "unknown.exe"),
            BrowserIdentity::Chrome
        );
        assert_eq!(
            WindowsBrowser::guess_identity("FIREFOX", "unknown.exe"),
            BrowserIdentity::Firefox
        );
    }

    #[test]
    fn guess_identity_custom_for_unknown() {
        assert_eq!(
            WindowsBrowser::guess_identity("MysteryBrowser", "app.exe"),
            BrowserIdentity::Custom("MysteryBrowser".to_string())
        );
    }

    #[test]
    fn guess_identity_edge_maps_to_custom() {
        assert_eq!(
            WindowsBrowser::guess_identity("MSEdge", "msedge.exe"),
            BrowserIdentity::Custom("Edge".to_string())
        );
    }

    #[test]
    fn parse_reg_sz_value_standard_format() {
        let stdout = "    (Default)    REG_SZ    \"C:\\browser.exe\" -- %1\n";
        let result = WindowsBrowser::parse_reg_sz_value(stdout);
        assert_eq!(result, Some(r#""C:\browser.exe" -- %1"#.to_string()));
    }

    #[test]
    fn parse_reg_sz_value_missing_line() {
        let stdout = "Some other output\n";
        assert!(WindowsBrowser::parse_reg_sz_value(stdout).is_none());
    }

    #[test]
    fn windows_browser_is_platform_browser() {
        let _browser: Box<dyn PlatformBrowser> = Box::new(WindowsBrowser::new());
    }
}
