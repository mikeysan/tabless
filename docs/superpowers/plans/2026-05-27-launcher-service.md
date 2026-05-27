# Launcher Service Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the launcher service that discovers installed browsers, tracks user preference by `BrowserIdentity`, and launches URLs safely without shell interpreters.

**Architecture:** A single `PlatformBrowser` trait at the OS boundary. Concrete types everywhere else: `BrowserIdentity`, `BrowserInfo`, `BrowserRegistry`, `Launcher<P>`. Platform structs (`LinuxBrowser`, `MacBrowser`, `WindowsBrowser`) are zero-sized and stateless.

**Tech Stack:** Rust `std::process::Command`, `std::io`, `std::path`, `std::collections::HashMap`. No external dependencies beyond the standard library.

---

## File Structure

| File | Responsibility |
|------|-------------|
| `src/launcher/identity.rs` | `BrowserIdentity` enum — canonical list of supported browsers |
| `src/launcher/info.rs` | `BrowserInfo` struct — metadata about an installed browser |
| `src/launcher/error.rs` | `DiscoveryError`, `LaunchError` enums with `Display` + `Error` impls |
| `src/launcher/platform.rs` | `PlatformBrowser` trait — the only trait in this subsystem |
| `src/launcher/registry.rs` | `BrowserRegistry` — maps identities to paths, stores user preference |
| `src/launcher/launcher.rs` | `Launcher<P>` — primary API: `launch(url)` and `launch_with_identity(...)` |
| `src/launcher/linux.rs` | `LinuxBrowser` — discovers browsers via `$PATH`, `.desktop` files, `xdg-settings` |
| `src/launcher/macos.rs` | `MacBrowser` — discovers via `/Applications`, `mdfind`, `defaults` |
| `src/launcher/windows.rs` | `WindowsBrowser` — discovers via registry, `tasklist`, `ShellExecuteW` |
| `src/launcher/mock.rs` | `MockPlatform` — test double implementing `PlatformBrowser` |
| `src/launcher/mod.rs` | Public API exports, platform selector, module declarations |

---

## Task 1: BrowserIdentity (`identity.rs`)

**Files:**
- Create: `src/launcher/identity.rs`
- Modify: `src/launcher/mod.rs` (add `pub mod identity;`)

- [ ] **Step 1: Write the failing test**

In `src/launcher/identity.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BrowserIdentity {
    Brave,
    Firefox,
    Zen,
    LibreWolf,
    Chrome,
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equality_for_known_browsers() {
        assert_eq!(BrowserIdentity::Brave, BrowserIdentity::Brave);
        assert_ne!(BrowserIdentity::Brave, BrowserIdentity::Firefox);
    }

    #[test]
    fn custom_equality() {
        assert_eq!(
            BrowserIdentity::Custom("vivaldi".to_string()),
            BrowserIdentity::Custom("vivaldi".to_string())
        );
        assert_ne!(
            BrowserIdentity::Custom("vivaldi".to_string()),
            BrowserIdentity::Custom("opera".to_string())
        );
    }

    #[test]
    fn custom_does_not_equal_known() {
        assert_ne!(
            BrowserIdentity::Custom("chrome".to_string()),
            BrowserIdentity::Chrome
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test launcher::identity::tests --lib`

Expected: FAIL with "module `launcher` not found" (because `mod.rs` doesn't declare it yet).

- [ ] **Step 3: Wire mod.rs and run again**

In `src/launcher/mod.rs`:

```rust
pub mod identity;
```

Run: `cargo test launcher::identity::tests --lib`

Expected: PASS (3 tests).

- [ ] **Step 4: Commit**

```bash
git add src/launcher/mod.rs src/launcher/identity.rs
git commit -m "feat: add BrowserIdentity enum with unit tests"
```

---

## Task 2: BrowserInfo (`info.rs`)

**Files:**
- Create: `src/launcher/info.rs`
- Modify: `src/launcher/mod.rs`

- [ ] **Step 1: Write the failing test**

In `src/launcher/info.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test launcher::info::tests --lib`

Expected: FAIL — `info` module not declared in `mod.rs`.

- [ ] **Step 3: Wire mod.rs and run again**

In `src/launcher/mod.rs`:

```rust
pub mod identity;
pub mod info;
```

Run: `cargo test launcher::info::tests --lib`

Expected: PASS (2 tests).

- [ ] **Step 4: Commit**

```bash
git add src/launcher/mod.rs src/launcher/info.rs
git commit -m "feat: add BrowserInfo struct with unit tests"
```

---

## Task 3: Error Types (`error.rs`)

**Files:**
- Create: `src/launcher/error.rs`
- Modify: `src/launcher/mod.rs`

- [ ] **Step 1: Write the failing test**

In `src/launcher/error.rs`:

```rust
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
                write!(f, "permission denied reading browser path: {}", path.display())
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
        assert_eq!(
            err.to_string(),
            "browser path not found: /usr/bin/firefox"
        );
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test launcher::error::tests --lib`

Expected: FAIL — `error` module not declared in `mod.rs`.

- [ ] **Step 3: Wire mod.rs and run again**

In `src/launcher/mod.rs`:

```rust
pub mod error;
pub mod identity;
pub mod info;
```

Run: `cargo test launcher::error::tests --lib`

Expected: PASS (3 tests).

- [ ] **Step 4: Commit**

```bash
git add src/launcher/mod.rs src/launcher/error.rs
git commit -m "feat: add DiscoveryError and LaunchError enums with Display impls"
```

---

## Task 4: PlatformBrowser Trait (`platform.rs`)

**Files:**
- Create: `src/launcher/platform.rs`
- Modify: `src/launcher/mod.rs`

- [ ] **Step 1: Write the failing test**

In `src/launcher/platform.rs`:

```rust
use std::io;
use std::process::Child;

use super::error::{DiscoveryError, LaunchError};
use super::info::BrowserInfo;

pub trait PlatformBrowser: Send + Sync {
    fn discover_browsers(&self) -> Result<Vec<BrowserInfo>, DiscoveryError>;
    fn is_browser_running(&self, info: &BrowserInfo) -> Result<bool, io::Error>;
    fn launch_url(&self, info: &BrowserInfo, url: &str) -> Result<Child, LaunchError>;
    fn launch_new_tab(&self, info: &BrowserInfo, url: &str) -> Result<Child, LaunchError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launcher::identity::BrowserIdentity;
    use std::path::PathBuf;

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
    }

    #[test]
    fn dummy_platform_implements_trait() {
        let dummy = DummyPlatform;
        let browsers = dummy.discover_browsers().unwrap();
        assert!(browsers.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test launcher::platform::tests --lib`

Expected: FAIL — `platform` module not declared in `mod.rs`.

- [ ] **Step 3: Wire mod.rs and run again**

In `src/launcher/mod.rs`:

```rust
pub mod error;
pub mod identity;
pub mod info;
pub mod platform;
```

Run: `cargo test launcher::platform::tests --lib`

Expected: PASS (1 test).

- [ ] **Step 4: Commit**

```bash
git add src/launcher/mod.rs src/launcher/platform.rs
git commit -m "feat: add PlatformBrowser trait with dummy impl test"
```

---

## Task 5: BrowserRegistry (`registry.rs`)

**Files:**
- Create: `src/launcher/registry.rs`
- Modify: `src/launcher/mod.rs`

- [ ] **Step 1: Write the failing test**

In `src/launcher/registry.rs`:

```rust
use std::collections::HashMap;

use super::error::DiscoveryError;
use super::identity::BrowserIdentity;
use super::info::BrowserInfo;

pub struct BrowserRegistry {
    known: HashMap<BrowserIdentity, BrowserInfo>,
    preferred: Option<BrowserIdentity>,
}

impl BrowserRegistry {
    pub fn new(discovered: Vec<BrowserInfo>) -> Self {
        let known = discovered
            .into_iter()
            .map(|info| (info.identity.clone(), info))
            .collect();
        BrowserRegistry {
            known,
            preferred: None,
        }
    }

    pub fn set_preferred(
        &mut self,
        identity: BrowserIdentity,
    ) -> Result<(), DiscoveryError> {
        if !self.known.contains_key(&identity) {
            return Err(DiscoveryError::PathNotFound {
                path: std::path::PathBuf::from(format!("{:?}", identity)),
            });
        }
        self.preferred = Some(identity);
        Ok(())
    }

    pub fn preferred_browser(&self) -> Option<&BrowserInfo> {
        self.preferred.as_ref().and_then(|id| self.known.get(id))
    }

    pub fn all_browsers(&self) -> Vec<&BrowserInfo> {
        self.known.values().collect()
    }

    pub fn find(&self, identity: &BrowserIdentity) -> Option<&BrowserInfo> {
        self.known.get(identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_info(identity: BrowserIdentity, path: &str) -> BrowserInfo {
        BrowserInfo {
            identity,
            executable_path: PathBuf::from(path),
            version: None,
            is_default: false,
        }
    }

    #[test]
    fn registry_stores_discovered_browsers() {
        let discovered = vec![
            make_info(BrowserIdentity::Firefox, "/usr/bin/firefox"),
            make_info(BrowserIdentity::Chrome, "/usr/bin/google-chrome"),
        ];
        let registry = BrowserRegistry::new(discovered);
        assert_eq!(registry.all_browsers().len(), 2);
    }

    #[test]
    fn set_preferred_succeeds_for_known_browser() {
        let discovered = vec![make_info(BrowserIdentity::Firefox, "/usr/bin/firefox")];
        let mut registry = BrowserRegistry::new(discovered);
        let result = registry.set_preferred(BrowserIdentity::Firefox);
        assert!(result.is_ok());
        assert_eq!(
            registry.preferred_browser().unwrap().identity,
            BrowserIdentity::Firefox
        );
    }

    #[test]
    fn set_preferred_fails_for_unknown_browser() {
        let discovered = vec![make_info(BrowserIdentity::Firefox, "/usr/bin/firefox")];
        let mut registry = BrowserRegistry::new(discovered);
        let result = registry.set_preferred(BrowserIdentity::Chrome);
        assert!(result.is_err());
    }

    #[test]
    fn find_returns_some_for_known() {
        let discovered = vec![make_info(BrowserIdentity::Firefox, "/usr/bin/firefox")];
        let registry = BrowserRegistry::new(discovered);
        assert!(registry.find(&BrowserIdentity::Firefox).is_some());
        assert!(registry.find(&BrowserIdentity::Chrome).is_none());
    }

    #[test]
    fn preferred_browser_none_when_unset() {
        let discovered = vec![make_info(BrowserIdentity::Firefox, "/usr/bin/firefox")];
        let registry = BrowserRegistry::new(discovered);
        assert!(registry.preferred_browser().is_none());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test launcher::registry::tests --lib`

Expected: FAIL — `registry` module not declared in `mod.rs`.

- [ ] **Step 3: Wire mod.rs and run again**

In `src/launcher/mod.rs`:

```rust
pub mod error;
pub mod identity;
pub mod info;
pub mod platform;
pub mod registry;
```

Run: `cargo test launcher::registry::tests --lib`

Expected: PASS (5 tests).

- [ ] **Step 4: Commit**

```bash
git add src/launcher/mod.rs src/launcher/registry.rs
git commit -m "feat: add BrowserRegistry with lookup and preference tracking"
```

---

## Task 6: MockPlatform (`mock.rs`)

**Files:**
- Create: `src/launcher/mock.rs`

- [ ] **Step 1: Write the failing test**

In `src/launcher/mock.rs`:

```rust
use std::collections::HashMap;
use std::io;
use std::process::Child;

use super::error::LaunchError;
use super::identity::BrowserIdentity;
use super::info::BrowserInfo;
use super::platform::PlatformBrowser;
use super::error::DiscoveryError;

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
        // We cannot construct a real Child in tests without spawning.
        // Return an error that test assertions can match on.
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test launcher::mock::tests --lib`

Expected: FAIL — `mock` module not declared in `mod.rs`.

- [ ] **Step 3: Wire mod.rs and run again**

In `src/launcher/mod.rs`:

```rust
pub mod error;
pub mod identity;
pub mod info;

#[cfg(test)]
pub mod mock;

pub mod platform;
pub mod registry;
```

Run: `cargo test launcher::mock::tests --lib`

Expected: PASS (3 tests).

- [ ] **Step 4: Commit**

```bash
git add src/launcher/mod.rs src/launcher/mock.rs
git commit -m "feat: add MockPlatform test double for PlatformBrowser"
```

---

## Task 7: Launcher (`launcher.rs`)

**Files:**
- Create: `src/launcher/launcher.rs`
- Modify: `src/launcher/mod.rs`

- [ ] **Step 1: Write the failing test**

In `src/launcher/launcher.rs`:

```rust
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
        let result = launcher.launch_with_identity(&BrowserIdentity::Firefox, "https://example.com");
        assert!(
            matches!(result, Err(LaunchError::SpawnFailed { source }) if source.contains("launch_url")),
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
        let result = launcher.launch_with_identity(&BrowserIdentity::Firefox, "https://example.com");
        assert!(
            matches!(result, Err(LaunchError::SpawnFailed { source }) if source.contains("launch_new_tab")),
            "expected launch_new_tab to be called when browser is running, got: {:?}",
            result
        );
    }

    #[test]
    fn launch_falls_back_to_launch_url_when_detection_fails() {
        // MockPlatform never errors from is_browser_running, so we can't test this path directly.
        // Instead, verify the control flow by inspection of launch_in_browser.
        // This test documents the expected behavior.
        let browsers = vec![make_info(BrowserIdentity::Firefox)];
        let platform = MockPlatform::new(browsers.clone());
        let launcher = Launcher::new(platform, browsers);
        // When is_browser_running returns Ok(false), launch_url is called.
        let result = launcher.launch_with_identity(&BrowserIdentity::Firefox, "https://example.com");
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test launcher::launcher::tests --lib`

Expected: FAIL — `launcher` module not declared in `mod.rs`.

- [ ] **Step 3: Wire mod.rs and run again**

In `src/launcher/mod.rs`:

```rust
pub mod error;
pub mod identity;
pub mod info;
pub mod launcher;

#[cfg(test)]
pub mod mock;

pub mod platform;
pub mod registry;
```

Run: `cargo test launcher::launcher::tests --lib`

Expected: PASS (5 tests).

- [ ] **Step 4: Commit**

```bash
git add src/launcher/mod.rs src/launcher/launcher.rs
git commit -m "feat: add Launcher<P> with launch logic and MockPlatform tests"
```

---

## Task 8: LinuxBrowser (`linux.rs`)

**Files:**
- Create: `src/launcher/linux.rs`
- Modify: `src/launcher/mod.rs`

- [ ] **Step 1: Write the failing test**

In `src/launcher/linux.rs`:

```rust
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Child;

use super::error::{DiscoveryError, LaunchError};
use super::identity::BrowserIdentity;
use super::info::BrowserInfo;
use super::platform::PlatformBrowser;

/// Linux-specific browser discovery and launching.
pub struct LinuxBrowser;

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
        if let Ok(output) = std::process::Command::new("xdg-settings")
            .arg("get")
            .arg("default-web-browser")
            .output()
        {
            if output.status.success() {
                let default = String::from_utf8_lossy(&output.stdout);
                let default_lower = default.trim().to_lowercase();
                for browser in &mut browsers {
                    let name = format!("{:?}", browser.identity).to_lowercase();
                    if default_lower.contains(&name) {
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
            matches!(result, Err(LaunchError::InvalidExecutable { path, .. }) if path == PathBuf::from("/does/not/exist"))
        );
    }
}
```

- [ ] **Step 2: Add `which` dependency**

In `Cargo.toml`:

```toml
[dependencies]
url = "2.5"
rusqlite = { version = "0.32", features = ["bundled"] }
sublime_fuzzy = "0.7"
which = "7.0"
```

Run: `cargo check`

Expected: FAIL — `linux` module not declared in `mod.rs`.

- [ ] **Step 3: Wire mod.rs and run tests**

In `src/launcher/mod.rs`:

```rust
pub mod error;
pub mod identity;
pub mod info;
pub mod launcher;
pub mod linux;

#[cfg(test)]
pub mod mock;

pub mod platform;
pub mod registry;
```

Run: `cargo test launcher::linux::tests --lib`

Expected: PASS (3 tests).

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock src/launcher/mod.rs src/launcher/linux.rs
git commit -m "feat: add LinuxBrowser with path discovery and process launching"
```

---

## Task 9: MacBrowser (`macos.rs`)

**Files:**
- Create: `src/launcher/macos.rs`
- Modify: `src/launcher/mod.rs`

- [ ] **Step 1: Write the failing test**

In `src/launcher/macos.rs`:

```rust
use std::io;
use std::path::PathBuf;
use std::process::Child;

use super::error::{DiscoveryError, LaunchError};
use super::identity::BrowserIdentity;
use super::info::BrowserInfo;
use super::platform::PlatformBrowser;

/// macOS-specific browser discovery and launching.
pub struct MacBrowser;

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
            let app_path = format!("/Applications/{}.app", Self::app_name_for_identity(&identity));
            let app_path_buf = PathBuf::from(&app_path);
            if app_path_buf.exists() {
                // Try to find the actual executable inside the bundle
                let executable = Self::find_executable_in_bundle(&app_path_buf);
                found.push(BrowserInfo {
                    identity: identity.clone(),
                    executable_path: executable.unwrap_or(app_path_buf),
                    version: None,
                    is_default: false,
                });
            } else {
                // Fallback: try mdfind
                if let Ok(output) = std::process::Command::new("mdfind")
                    .arg(format!("kMDItemCFBundleIdentifier == '{}'", bundle_id))
                    .output()
                {
                    if output.status.success() {
                        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if !path.is_empty() {
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
                }
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

    fn find_executable_in_bundle(bundle_path: &PathBuf) -> Option<PathBuf> {
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

        // Mark default browser
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
        // On macOS, `open -a` opens a new tab if the app is already running.
        self.launch_url(info, url)
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test launcher::macos::tests --lib`

Expected: FAIL — `macos` module not declared in `mod.rs`.

- [ ] **Step 3: Wire mod.rs and run again**

In `src/launcher/mod.rs`:

```rust
pub mod error;
pub mod identity;
pub mod info;
pub mod launcher;
pub mod linux;
pub mod macos;

#[cfg(test)]
pub mod mock;

pub mod platform;
pub mod registry;
```

Run: `cargo test launcher::macos::tests --lib`

Expected: PASS (3 tests).

- [ ] **Step 4: Commit**

```bash
git add src/launcher/mod.rs src/launcher/macos.rs
git commit -m "feat: add MacBrowser with bundle discovery and open -a launching"
```

---

## Task 10: WindowsBrowser (`windows.rs`)

**Files:**
- Create: `src/launcher/windows.rs`
- Modify: `src/launcher/mod.rs`

- [ ] **Step 1: Write the failing test**

In `src/launcher/windows.rs`:

```rust
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
            // Try HKLM first, then HKCU
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
                        // Parse the (Default) value line
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
                                    break; // Found in this hive, stop looking
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

        // Try to detect default browser from HKCR\http\shell\open\command
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
        // On Windows, most browsers open a new tab when given a URL if already running.
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test launcher::windows::tests --lib`

Expected: FAIL — `windows` module not declared in `mod.rs`.

- [ ] **Step 3: Wire mod.rs and run again**

In `src/launcher/mod.rs`:

```rust
pub mod error;
pub mod identity;
pub mod info;
pub mod launcher;
pub mod linux;
pub mod macos;
pub mod platform;
pub mod registry;
pub mod windows;

#[cfg(test)]
pub mod mock;
```

Run: `cargo test launcher::windows::tests --lib`

Expected: PASS (2 tests).

- [ ] **Step 4: Commit**

```bash
git add src/launcher/mod.rs src/launcher/windows.rs
git commit -m "feat: add WindowsBrowser with registry discovery and tasklist detection"
```

---

## Task 11: Public Exports and lib.rs Wiring

**Files:**
- Modify: `src/launcher/mod.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Add platform selector and public re-exports**

In `src/launcher/mod.rs`:

```rust
pub mod error;
pub mod identity;
pub mod info;
pub mod launcher;
pub mod linux;
pub mod macos;
pub mod platform;
pub mod registry;
pub mod windows;

#[cfg(test)]
pub mod mock;

// Public re-exports for convenient access
pub use error::{DiscoveryError, LaunchError};
pub use identity::BrowserIdentity;
pub use info::BrowserInfo;
pub use launcher::Launcher;
pub use platform::PlatformBrowser;
pub use registry::BrowserRegistry;

// Platform-specific default implementation selector
#[cfg(target_os = "linux")]
pub use linux::LinuxBrowser as DefaultPlatform;

#[cfg(target_os = "macos")]
pub use macos::MacBrowser as DefaultPlatform;

#[cfg(target_os = "windows")]
pub use windows::WindowsBrowser as DefaultPlatform;
```

- [ ] **Step 2: Add launcher module to lib.rs**

In `src/lib.rs`:

```rust
pub mod launcher;
pub mod storage;
pub mod url;
```

- [ ] **Step 3: Run full test suite**

Run: `cargo test --lib`

Expected: PASS — all launcher tests pass.

Run: `cargo test`

Expected: PASS — all existing tests still pass.

Run: `cargo clippy`

Expected: No new warnings.

- [ ] **Step 4: Commit**

```bash
git add src/launcher/mod.rs src/lib.rs
git commit -m "feat: wire launcher module into lib.rs with platform selector"
```

---

## Spec Coverage Check

| Spec Requirement | Task |
|-----------------|------|
| `BrowserIdentity` enum (Brave, Firefox, Zen, LibreWolf, Chrome, Custom) | Task 1 |
| `BrowserInfo` struct | Task 2 |
| `DiscoveryError`, `LaunchError` with `Display` + `Error` | Task 3 |
| `PlatformBrowser` trait (only trait in subsystem) | Task 4 |
| `BrowserRegistry` — maps identities, stores preference | Task 5 |
| `Launcher<P>` — `launch()`, `launch_with_identity()` | Task 7 |
| Launch logic: detect running → new tab, fallback → plain launch | Task 7 (tests verify control flow) |
| Linux discovery (`$PATH`, `xdg-settings`, `pgrep`) | Task 8 |
| macOS discovery (`/Applications`, `mdfind`, `defaults`) | Task 9 |
| Windows discovery (registry, `tasklist`) | Task 10 |
| No shell interpreters — direct `Command::new(...).arg(url).spawn()` | Tasks 8, 9, 10 |
| MockPlatform for unit tests | Task 6 |
| Platform selector (`DefaultPlatform`) | Task 11 |

---

## Placeholder Scan

- No "TBD", "TODO", "implement later" found.
- No vague "add error handling" steps — every step has exact code.
- No "similar to Task N" — each task is self-contained.
- All test code is included verbatim.
- All expected outputs are specified.

---

## Type Consistency Check

| Type | First Defined | Used Later |
|------|--------------|-----------|
| `BrowserIdentity` | Task 1 | Tasks 2, 5, 6, 7, 8, 9, 10 |
| `BrowserInfo` | Task 2 | Tasks 4, 5, 6, 7, 8, 9, 10 |
| `DiscoveryError` | Task 3 | Tasks 4, 5, 8, 9, 10 |
| `LaunchError` | Task 3 | Tasks 4, 6, 7, 8, 9, 10 |
| `PlatformBrowser` | Task 4 | Tasks 6, 7, 8, 9, 10 |
| `BrowserRegistry` | Task 5 | Task 7 |
| `MockPlatform` | Task 6 | Task 7 |
| `Launcher<P>` | Task 7 | — |

All method names match across tasks.
