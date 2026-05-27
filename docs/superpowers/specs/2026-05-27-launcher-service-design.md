# Launcher Service Design

## Overview

The launcher service opens URLs in the user's preferred browser. It discovers installed browsers, tracks the user's preference by `BrowserIdentity` (not raw path), and launches URLs safely without shell interpreters.

This subsystem is the bridge between stored URLs and active browsing.

## Goals

- Discover installed browsers on the current platform.
- Identify browsers by canonical `BrowserIdentity` (Brave, Firefox, Zen, LibreWolf, Chrome, Custom).
- Store and retrieve the user's preferred browser identity.
- Launch a URL in the preferred browser.
- Detect if the preferred browser is already running and open the URL in a new tab when possible.
- Remain cross-platform (Linux, macOS, Windows) with a single platform trait.
- Use no shell interpreters — only direct executable spawning with argument arrays.
- Return fine-grained, typed errors for every failure mode.
- Contain zero panics and zero unwraps in production paths.

## Non-Goals

- Embedded browser rendering.
- Browser-specific automation (e.g., DevTools Protocol, AppleScript beyond `open -a`).
- Remote or cloud-based browser launching.
- Forcing browsers to open incognito/private windows.

## Tech Stack

- **Language:** Rust.
- **Platform Detection:** OS-specific logic behind a single trait (`PlatformBrowser`).
- **Process Spawning:** `std::process::Command` with explicit executable paths and argument arrays.

## Module Structure

```
src/
  launcher/
    mod.rs           — public API exports, platform selector
    error.rs         — DiscoveryError, LaunchError
    identity.rs      — BrowserIdentity enum
    info.rs          — BrowserInfo struct
    registry.rs      — BrowserRegistry: stores known browsers + user preference
    launcher.rs      — Launcher<P>: primary launch API
    platform.rs      — PlatformBrowser trait (OS boundary)
    linux.rs         — LinuxBrowser impl
    macos.rs         — MacBrowser impl
    windows.rs       — WindowsBrowser impl
    mock.rs          — MockPlatform for unit tests (#[cfg(test)])
```

## Types

### BrowserIdentity

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
```

The canonical list of supported browsers. `Custom` covers browsers installed in non-standard locations or not yet explicitly supported.

### BrowserInfo

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserInfo {
    pub identity: BrowserIdentity,
    pub executable_path: PathBuf,
    pub version: Option<String>,
    pub is_default: bool,
}
```

Represents everything the application knows about an installed browser. `is_default` is a hint populated during discovery; it does not guarantee the browser is the OS default at launch time.

### BrowserRegistry

```rust
pub struct BrowserRegistry {
    known: HashMap<BrowserIdentity, BrowserInfo>,
    preferred: Option<BrowserIdentity>,
}

impl BrowserRegistry {
    pub fn new(discovered: Vec<BrowserInfo>) -> Self;
    pub fn set_preferred(&mut self, identity: BrowserIdentity) -> Result<(), DiscoveryError>;
    pub fn preferred_browser(&self) -> Option<&BrowserInfo>;
    pub fn all_browsers(&self) -> &[BrowserInfo];
    pub fn find(&self, identity: &BrowserIdentity) -> Option<&BrowserInfo>;
}
```

Concrete struct — no trait. Owns the mapping from identities to paths and the user's preference. Discovery results are passed in; the registry does not perform OS discovery itself.

## Error Types

### DiscoveryError

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryError {
    PlatformNotSupported,
    PathNotFound { path: PathBuf },
    PermissionDenied { path: PathBuf },
    ReadFailed { source: String },
}
```

Returned by `PlatformBrowser::discover_browsers` when the platform cannot enumerate installed browsers.

### LaunchError

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchError {
    BrowserNotFound { identity: BrowserIdentity },
    InvalidExecutable { path: PathBuf, reason: String },
    SpawnFailed { source: String },
    AlreadyRunningButTabFailed,
}
```

Returned by `Launcher` when a URL cannot be opened. `AlreadyRunningButTabFailed` means the browser was detected as running but the new-tab attempt failed; the caller may retry with `launch_url` or surface the error to the user.

Both enums implement `Display` and `std::error::Error`.

## The OS-Boundary Trait

```rust
pub trait PlatformBrowser: Send + Sync {
    fn discover_browsers(&self) -> Result<Vec<BrowserInfo>, DiscoveryError>;
    fn is_browser_running(&self, info: &BrowserInfo) -> Result<bool, io::Error>;
    fn launch_url(&self, info: &BrowserInfo, url: &str) -> Result<Child, LaunchError>;
    fn launch_new_tab(&self, info: &BrowserInfo, url: &str) -> Result<Child, LaunchError>;
}
```

This is the **only trait** in the launcher subsystem.

### Trait Methods

- **`discover_browsers`** — Platform-specific discovery of installed browsers. Best-effort; non-standard installs may require manual `Custom` configuration.
- **`is_browser_running`** — Check whether the browser process is active. Returns `Result<bool, io::Error>` so callers can fall back on failure.
- **`launch_url`** — Spawn a new browser process with the given URL. Must use executable path + argument array, never a shell.
- **`launch_new_tab`** — Open the URL in an existing browser instance. Only called when `is_browser_running` returns `Ok(true)`.

Each platform implementation (`LinuxBrowser`, `MacBrowser`, `WindowsBrowser`) is a zero-sized, stateless struct.

## Launcher

```rust
pub struct Launcher<P: PlatformBrowser> {
    platform: P,
    registry: BrowserRegistry,
}

impl<P: PlatformBrowser> Launcher<P> {
    pub fn new(platform: P, discovered: Vec<BrowserInfo>) -> Self;

    /// Launch a URL in the user's preferred browser.
    pub fn launch(&self, url: &str) -> Result<Child, LaunchError>;

    /// Launch a URL in a specific browser by identity.
    pub fn launch_with_identity(
        &self,
        identity: &BrowserIdentity,
        url: &str,
    ) -> Result<Child, LaunchError>;
}
```

### Launch Logic

```rust
fn launch_in_browser(&self, browser: &BrowserInfo, url: &str)
    -> Result<Child, LaunchError>
{
    match self.platform.is_browser_running(browser) {
        Ok(true) => self.platform.launch_new_tab(browser, url),
        Ok(false) => self.platform.launch_url(browser, url),
        Err(_) => {
            // Detection failed — don't trust it, fall back to plain launch.
            self.platform.launch_url(browser, url)
        }
    }
}
```

The launcher:
1. Resolves the preferred browser from the registry.
2. Checks whether the browser is running.
3. If running and the platform supports reliable new-tab opening, uses `launch_new_tab`.
4. On any error or ambiguity, falls back to `launch_url`.

URL validation is the caller's responsibility — the launcher receives pre-validated URLs.

## Platform Implementations

### Linux

**Discovery:**
- Search `$PATH` for `brave`, `firefox`, `zen`, `librewolf`, `google-chrome-stable`, `google-chrome`, `chrome`.
- Check common install locations: `/usr/bin`, `/usr/local/bin`, `/opt`, `~/.local/bin`.
- Parse `.desktop` files in `/usr/share/applications` and `~/.local/share/applications` for name, executable path, and `IsDefault` hints.
- Read `xdg-settings get default-web-browser` to detect the OS default.

**Running detection:**
- Scan `/proc` for process names matching the browser executable.

**Launch:**
- `Command::new(&browser.executable_path).arg(url).spawn()`
- No shell expansion.

### macOS

**Discovery:**
- Search `/Applications/*.app` for known bundle identifiers (e.g., `com.brave.Browser`, `org.mozilla.firefox`, `com.google.Chrome`).
- Use `mdfind` or `system_profiler SPApplicationsDataType` as fallback.
- Read `defaults read com.apple.LaunchServices/com.apple.launchservices.secure LSHandlers` for the default browser.

**Running detection:**
- `pgrep -x` or `ps` filtered by process name.

**Launch:**
- `open -a "Browser Name" <url>` — opens new tab if browser is running.
- `open -a "Browser Name" --background <url>` — opens without bringing to foreground.

### Windows

**Discovery:**
- Read registry keys under `SOFTWARE\Clients\StartMenuInternet`.
- Check `HKCR\http\shell\open\command` for the default browser.
- Query `HKEY_CURRENT_USER\Software\Microsoft\Windows\Shell\Associations\UrlAssociations\http\UserChoice`.

**Running detection:**
- `tasklist` or Windows API `EnumProcesses`.

**Launch:**
- `ShellExecuteW` with the browser executable and URL.
- Direct `Command::new(&path).arg(url).spawn()` for non-default browsers.

## Testing Strategy

### Unit Tests (Mocked Platform)

A `MockPlatform` implements `PlatformBrowser` and records all calls:

```rust
struct MockPlatform {
    browsers: Vec<BrowserInfo>,
    running: HashMap<BrowserIdentity, bool>,
    spawn_results: HashMap<(BrowserIdentity, bool), Result<FakeChild, LaunchError>>,
}
```

Covered behaviours:
- `BrowserRegistry` lookup, preference setting, fallback to default.
- `Launcher::launch` selects `launch_new_tab` when running, `launch_url` when not.
- `Launcher` falls back to `launch_url` when `is_browser_running` errors.
- `launch_with_identity` fails with `BrowserNotFound` for unknown identity.
- URL escaping: the launcher never passes a URL through a shell.

`FakeChild` is a test double for `std::process::Child` so `Launcher::launch` can return a mock handle.

### Smoke Tests (Real OS, Opt-In)

```rust
#[cfg(feature = "smoke-tests")]
mod smoke {
    // Uses the real LinuxBrowser / MacBrowser / WindowsBrowser.
    // Requires at least one supported browser to be installed.
    // Marked with #[ignore] so they don't run by default.
}
```

Run with: `cargo test --features smoke-tests -- --ignored`

These tests verify that discovery finds installed browsers and that launching a known-safe URL (e.g., `https://example.com`) spawns a process. They do not verify that a visible tab/window opens — that requires browser automation, which is out of scope.

## Security Considerations

- **No shell interpreters.** `launch_url` and `launch_new_tab` use `std::process::Command` with an explicit executable path and a single URL argument. No `sh -c`, `cmd /c`, or `powershell` invocations.
- **Pre-validated URLs.** The launcher assumes the caller has already validated the URL scheme (only `http`/`https`). The launcher does not re-validate but will reject non-UTF-8 URLs at the `Command` boundary.
- **Path validation.** Before spawning, the launcher checks that `executable_path` exists and is a file. `InvalidExecutable` is returned if not.
- **No panics.** All error paths return typed errors. No `unwrap`, `expect`, or `panic!` in production code.

## Success Criteria

- `Launcher::launch` opens a URL in the user's preferred browser.
- If the preferred browser is running, the URL opens in a new tab.
- If the preferred browser is not found, `LaunchError::BrowserNotFound` is returned.
- Discovery finds at least the common browsers (Brave, Firefox, Chrome) on each supported platform.
- The `BrowserRegistry` correctly maps identities to paths and persists the user's preference.
- All public methods return typed errors; no panics in production paths.
- Unit tests pass with `MockPlatform`.
- Smoke tests pass when run with `--features smoke-tests`.
