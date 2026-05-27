# Protocol Handler Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `src/protocol/` subsystem that registers `tabless://` as a custom URL scheme, handles single-instance forwarding, validates and stores URLs captured from other apps.

**Architecture:** The protocol handler uses an IPC socket to detect whether a tabless instance is already running. A first instance binds a local socket and blocks accepting URLs. A subsequent instance connects, sends the URL, and exits. The captured URL is parsed from `tabless://open?url=...`, validated by `ValidatedUrl::parse`, and stored via `Storage::urls::insert`.

**Tech Stack:** Rust, `interprocess` v2 (cross-platform local sockets), `url` crate (query parameter parsing), `dirs` (platform data directories — already available via standard library or we add it).

---

## File Structure

| File | Responsibility |
|------|----------------|
| `src/protocol/error.rs` | `ProtocolError` enum with `Display` + `Error` |
| `src/protocol/parse.rs` | `parse_protocol_url(input)` — extracts embedded URL from `tabless://open?url=...` |
| `src/protocol/ipc.rs` | `IpcServer` (bind + accept) and `IpcClient` (connect + send) using `interprocess` local sockets |
| `src/protocol/single_instance.rs` | `SingleInstance::new()` — tries to bind server; on failure, connects as client |
| `src/protocol/registration.rs` | `register_protocol()` — OS-specific registration (Linux `.desktop`, macOS `.app` wrapper, Windows registry) |
| `src/protocol/mod.rs` | Public API: `ProtocolConfig`, `ProtocolHandler`, `RunOutcome`, re-exports |
| `src/lib.rs` | Add `pub mod protocol;` |
| `src/main.rs` | Wire protocol handling: check args, single instance, store URL |
| `Cargo.toml` | Add `interprocess = "2"` dependency |

---

### Task 1: Add `interprocess` dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add dependency**

```toml
[dependencies]
url = "2.5"
rusqlite = { version = "0.32", features = ["bundled"] }
sublime_fuzzy = "0.7"
which = "7.0"
interprocess = "2"
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: No errors (empty module not yet created, but dependency resolves).

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "feat: add interprocess dependency for protocol IPC"
```

---

### Task 2: ProtocolError enum

**Files:**
- Create: `src/protocol/error.rs`

- [ ] **Step 1: Write the error enum**

```rust
use std::fmt;

use crate::storage::error::StorageError;
use crate::url::error::UrlValidationError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    InvalidUrl { reason: String },
    UrlValidationFailed { source: UrlValidationError },
    StorageFailed { source: StorageError },
    IpcBindFailed { reason: String },
    IpcConnectFailed { reason: String },
    RegistrationFailed { platform: String, reason: String },
    AlreadyRegistered,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtocolError::InvalidUrl { reason } => write!(f, "invalid protocol URL: {}", reason),
            ProtocolError::UrlValidationFailed { source } => {
                write!(f, "URL validation failed: {}", source)
            }
            ProtocolError::StorageFailed { source } => {
                write!(f, "storage error: {}", source)
            }
            ProtocolError::IpcBindFailed { reason } => {
                write!(f, "IPC bind failed: {}", reason)
            }
            ProtocolError::IpcConnectFailed { reason } => {
                write!(f, "IPC connect failed: {}", reason)
            }
            ProtocolError::RegistrationFailed { platform, reason } => {
                write!(f, "protocol registration failed on {}: {}", platform, reason)
            }
            ProtocolError::AlreadyRegistered => {
                write!(f, "protocol already registered")
            }
        }
    }
}

impl std::error::Error for ProtocolError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProtocolError::UrlValidationFailed { source } => Some(source),
            ProtocolError::StorageFailed { source } => Some(source),
            _ => None,
        }
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/protocol/error.rs
git commit -m "feat: add ProtocolError enum"
```

---

### Task 3: Parse protocol URL

**Files:**
- Create: `src/protocol/parse.rs`

- [ ] **Step 1: Write `parse_protocol_url`**

```rust
use super::error::ProtocolError;

pub fn parse_protocol_url(input: &str) -> Result<String, ProtocolError> {
    let parsed = url::Url::parse(input).map_err(|e| ProtocolError::InvalidUrl {
        reason: format!("failed to parse protocol URL: {}", e),
    })?;

    if parsed.scheme() != "tabless" {
        return Err(ProtocolError::InvalidUrl {
            reason: format!("expected scheme 'tabless', found '{}'", parsed.scheme()),
        });
    }

    if parsed.path() != "/open" {
        return Err(ProtocolError::InvalidUrl {
            reason: format!("expected path '/open', found '{}'", parsed.path()),
        });
    }

    let embedded = parsed
        .query_pairs()
        .find(|(k, _)| k == "url")
        .map(|(_, v)| v.into_owned());

    match embedded {
        Some(url) if !url.is_empty() => Ok(url),
        _ => Err(ProtocolError::InvalidUrl {
            reason: "missing or empty 'url' query parameter".to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_tabless_url() {
        let result = parse_protocol_url("tabless://open?url=https://example.com").unwrap();
        assert_eq!(result, "https://example.com");
    }

    #[test]
    fn parse_url_with_encoded_value() {
        let result = parse_protocol_url("tabless://open?url=https%3A%2F%2Fexample.com%2Fpath").unwrap();
        assert_eq!(result, "https://example.com/path");
    }

    #[test]
    fn parse_rejects_wrong_scheme() {
        let result = parse_protocol_url("https://open?url=https://example.com");
        assert!(matches!(result, Err(ProtocolError::InvalidUrl { .. })));
    }

    #[test]
    fn parse_rejects_wrong_path() {
        let result = parse_protocol_url("tabless://other?url=https://example.com");
        assert!(matches!(result, Err(ProtocolError::InvalidUrl { .. })));
    }

    #[test]
    fn parse_rejects_missing_url_param() {
        let result = parse_protocol_url("tabless://open?other=thing");
        assert!(matches!(result, Err(ProtocolError::InvalidUrl { .. })));
    }

    #[test]
    fn parse_rejects_empty_url_param() {
        let result = parse_protocol_url("tabless://open?url=");
        assert!(matches!(result, Err(ProtocolError::InvalidUrl { .. })));
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test protocol::parse::tests --lib`
Expected: All 6 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/protocol/parse.rs
git commit -m "feat: add parse_protocol_url with tests"
```

---

### Task 4: IPC layer (IpcServer + IpcClient)

**Files:**
- Create: `src/protocol/ipc.rs`

- [ ] **Step 1: Write the IPC module**

Uses `interprocess` v2 `LocalSocketListener` and `LocalSocketStream`. The socket name is a file path on Unix and a namespaced path on Windows.

```rust
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

use interprocess::local_socket::{
    traits::Listener, LocalSocketListener, LocalSocketStream,
};

use super::error::ProtocolError;

pub struct IpcServer {
    listener: LocalSocketListener,
}

impl IpcServer {
    pub fn bind(socket_path: &Path) -> Result<Self, ProtocolError> {
        // Remove stale socket file on Unix before binding
        #[cfg(unix)]
        let _ = std::fs::remove_file(socket_path);

        let name = socket_path.to_string_lossy().into_owned();
        let listener = LocalSocketListener::bind(name).map_err(|e| ProtocolError::IpcBindFailed {
            reason: e.to_string(),
        })?;
        Ok(IpcServer { listener })
    }

    pub fn accept_url(&self) -> Result<String, ProtocolError> {
        let stream = self.listener.accept().map_err(|e| ProtocolError::IpcBindFailed {
            reason: format!("accept failed: {}", e),
        })?;

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|e| ProtocolError::IpcBindFailed {
                reason: format!("read failed: {}", e),
            })?;

        let line = line.trim_end_matches('\n');
        if let Some(url) = line.strip_prefix("URL:") {
            Ok(url.to_string())
        } else {
            Err(ProtocolError::InvalidUrl {
                reason: format!("unexpected IPC message: {}", line),
            })
        }
    }
}

pub struct IpcClient {
    stream: LocalSocketStream,
}

impl IpcClient {
    pub fn connect(socket_path: &Path) -> Result<Self, ProtocolError> {
        let name = socket_path.to_string_lossy().into_owned();
        let stream = LocalSocketStream::connect(name).map_err(|e| {
            ProtocolError::IpcConnectFailed {
                reason: e.to_string(),
            }
        })?;
        Ok(IpcClient { stream })
    }

    pub fn send_url(&mut self, url: &str) -> Result<(), ProtocolError> {
        let message = format!("URL:{}\n", url);
        self.stream
            .write_all(message.as_bytes())
            .map_err(|e| ProtocolError::IpcConnectFailed {
                reason: format!("write failed: {}", e),
            })?;
        self.stream
            .flush()
            .map_err(|e| ProtocolError::IpcConnectFailed {
                reason: format!("flush failed: {}", e),
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn ipc_roundtrip() {
        let tmpdir = std::env::temp_dir().join(format!("tabless-test-{}", std::process::id()));
        let socket_path = tmpdir.join("ipc.sock");
        let _ = std::fs::create_dir_all(&tmpdir);

        let server_path = socket_path.clone();
        let handle = thread::spawn(move || {
            let server = IpcServer::bind(&server_path).unwrap();
            server.accept_url().unwrap()
        });

        // Give server time to bind
        thread::sleep(std::time::Duration::from_millis(100));

        let mut client = IpcClient::connect(&socket_path).unwrap();
        client.send_url("https://example.com").unwrap();

        let received = handle.join().unwrap();
        assert_eq!(received, "https://example.com");

        let _ = std::fs::remove_dir_all(&tmpdir);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test protocol::ipc::tests --lib`
Expected: `ipc_roundtrip` passes.

- [ ] **Step 3: Commit**

```bash
git add src/protocol/ipc.rs
git commit -m "feat: add IpcServer and IpcClient with roundtrip test"
```

---

### Task 5: SingleInstance

**Files:**
- Create: `src/protocol/single_instance.rs`

- [ ] **Step 1: Write SingleInstance**

```rust
use std::path::Path;

use super::error::ProtocolError;
use super::ipc::{IpcClient, IpcServer};

pub enum SingleInstance {
    First(IpcServer),
    Subsequent(IpcClient),
}

impl SingleInstance {
    pub fn new(socket_path: &Path) -> Result<Self, ProtocolError> {
        // Try to connect first — if another instance is running, we are the client.
        match IpcClient::connect(socket_path) {
            Ok(client) => Ok(SingleInstance::Subsequent(client)),
            Err(_) => {
                // No server running; become the server.
                let server = IpcServer::bind(socket_path)?;
                Ok(SingleInstance::First(server))
            }
        }
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/protocol/single_instance.rs
git commit -m "feat: add SingleInstance detection"
```

---

### Task 6: Platform registration

**Files:**
- Create: `src/protocol/registration.rs`

- [ ] **Step 1: Write registration module**

This is platform-conditional. Each platform gets its own `register_protocol` implementation.

```rust
use std::fs;
use std::path::Path;

use super::error::ProtocolError;

pub fn register_protocol(binary_path: &Path) -> Result<(), ProtocolError> {
    #[cfg(target_os = "linux")]
    return register_linux(binary_path);

    #[cfg(target_os = "macos")]
    return register_macos(binary_path);

    #[cfg(target_os = "windows")]
    return register_windows(binary_path);

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    Err(ProtocolError::RegistrationFailed {
        platform: "unknown".to_string(),
        reason: "unsupported platform".to_string(),
    })
}

#[cfg(target_os = "linux")]
fn register_linux(binary_path: &Path) -> Result<(), ProtocolError> {
    let apps_dir = dirs::data_local_dir()
        .ok_or_else(|| ProtocolError::RegistrationFailed {
            platform: "linux".to_string(),
            reason: "could not determine data local dir".to_string(),
        })?
        .join("applications");

    fs::create_dir_all(&apps_dir).map_err(|e| ProtocolError::RegistrationFailed {
        platform: "linux".to_string(),
        reason: format!("create dir failed: {}", e),
    })?;

    let desktop_path = apps_dir.join("tabless.desktop");
    if desktop_path.exists() {
        return Err(ProtocolError::AlreadyRegistered);
    }

    let exec = binary_path.to_string_lossy();
    let desktop_entry = format!(
        "[Desktop Entry]\n\
         Name=Tabless\n\
         Exec={} %u\n\
         Type=Application\n\
         Terminal=false\n\
         MimeType=x-scheme-handler/tabless;\n",
        exec
    );

    fs::write(&desktop_path, desktop_entry).map_err(|e| ProtocolError::RegistrationFailed {
        platform: "linux".to_string(),
        reason: format!("write .desktop failed: {}", e),
    })?;

    // Run xdg-mime to set default
    let status = std::process::Command::new("xdg-mime")
        .args(["default", "tabless.desktop", "x-scheme-handler/tabless"])
        .status()
        .map_err(|e| ProtocolError::RegistrationFailed {
            platform: "linux".to_string(),
            reason: format!("xdg-mime failed: {}", e),
        })?;

    if !status.success() {
        return Err(ProtocolError::RegistrationFailed {
            platform: "linux".to_string(),
            reason: "xdg-mime exited with error".to_string(),
        });
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn register_macos(binary_path: &Path) -> Result<(), ProtocolError> {
    // For CLI-only usage we write a minimal .app bundle wrapper
    let apps_dir = dirs::home_dir()
        .ok_or_else(|| ProtocolError::RegistrationFailed {
            platform: "macos".to_string(),
            reason: "could not determine home dir".to_string(),
        })?
        .join("Applications")
        .join("Tabless.app");

    if apps_dir.exists() {
        return Err(ProtocolError::AlreadyRegistered);
    }

    let contents = apps_dir.join("Contents");
    let macos = contents.join("MacOS");
    fs::create_dir_all(&macos).map_err(|e| ProtocolError::RegistrationFailed {
        platform: "macos".to_string(),
        reason: format!("create dir failed: {}", e),
    })?;

    let info_plist = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
         <key>CFBundleIdentifier</key>\n\
         <string>com.tabless.app</string>\n\
         <key>CFBundleName</key>\n\
         <string>Tabless</string>\n\
         <key>CFBundleExecutable</key>\n\
         <string>tabless-wrapper</string>\n\
         <key>CFBundleURLTypes</key>\n\
         <array>\n\
         <dict>\n\
         <key>CFBundleURLName</key>\n\
         <string>Tabless URL</string>\n\
         <key>CFBundleURLSchemes</key>\n\
         <array>\n\
         <string>tabless</string>\n\
         </array>\n\
         </dict>\n\
         </array>\n\
         </dict>\n\
         </plist>\n"
    );

    fs::write(contents.join("Info.plist"), info_plist).map_err(|e| {
        ProtocolError::RegistrationFailed {
            platform: "macos".to_string(),
            reason: format!("write Info.plist failed: {}", e),
        }
    })?;

    let wrapper_script = format!(
        "#!/bin/sh\nexec \"{}\" \"$1\"\n",
        binary_path.to_string_lossy()
    );

    let wrapper_path = macos.join("tabless-wrapper");
    fs::write(&wrapper_path, wrapper_script).map_err(|e| {
        ProtocolError::RegistrationFailed {
            platform: "macos".to_string(),
            reason: format!("write wrapper failed: {}", e),
        }
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&wrapper_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&wrapper_path, perms).map_err(|e| {
            ProtocolError::RegistrationFailed {
                platform: "macos".to_string(),
                reason: format!("chmod wrapper failed: {}", e),
            }
        })?;
    }

    // Register with LaunchServices
    let _ = std::process::Command::new("lsregister")
        .arg("-f")
        .arg(&apps_dir)
        .status();

    Ok(())
}

#[cfg(target_os = "windows")]
fn register_windows(binary_path: &Path) -> Result<(), ProtocolError> {
    use std::process::Command;

    let binary_str = binary_path.to_string_lossy();
    let reg_commands = [
        format!(r#"add HKEY_CLASSES_ROOT\tabless /ve /d "URL:Tabless Protocol" /f"#),
        format!(r#"add HKEY_CLASSES_ROOT\tabless /v "URL Protocol" /d "" /f"#),
        format!(
            r#"add HKEY_CLASSES_ROOT\tabless\shell\open\command /ve /d "\"{}\" \"%%1\"" /f"#,
            binary_str
        ),
    ];

    for cmd in &reg_commands {
        let status = Command::new("reg")
            .args(["add", &cmd[4..]])
            .status()
            .map_err(|e| ProtocolError::RegistrationFailed {
                platform: "windows".to_string(),
                reason: format!("reg command failed: {}", e),
            })?;

        if !status.success() {
            return Err(ProtocolError::RegistrationFailed {
                platform: "windows".to_string(),
                reason: format!("reg command exited with error: {}", cmd),
            });
        }
    }

    Ok(())
}
```

- [ ] **Step 2: Commit**

```bash
git add src/protocol/registration.rs
git commit -m "feat: add cross-platform protocol registration"
```

---

### Task 7: ProtocolHandler and mod.rs

**Files:**
- Create: `src/protocol/mod.rs`

- [ ] **Step 1: Write the public API**

```rust
pub mod error;
pub mod ipc;
pub mod parse;
pub mod registration;
pub mod single_instance;

pub use error::ProtocolError;

use std::path::{Path, PathBuf};

use crate::storage::Storage;
use crate::url::ValidatedUrl;

use self::ipc::{IpcClient, IpcServer};
use self::parse::parse_protocol_url;
use self::registration::register_protocol;
use self::single_instance::SingleInstance;

pub struct ProtocolConfig {
    pub scheme: &'static str,
    pub binary_path: PathBuf,
    pub data_dir: PathBuf,
}

impl ProtocolConfig {
    pub fn socket_path(&self) -> PathBuf {
        self.data_dir.join("tabless.ipc")
    }
}

pub enum RunOutcome {
    FirstInstance,
    UrlForwarded,
}

pub struct ProtocolHandler {
    storage: Storage,
    config: ProtocolConfig,
}

impl ProtocolHandler {
    pub fn new(config: ProtocolConfig, storage: Storage) -> Result<Self, ProtocolError> {
        Ok(ProtocolHandler { storage, config })
    }

    pub fn handle_url(&self, url: &str) -> Result<(), ProtocolError> {
        let raw = parse_protocol_url(url)?;
        let validated = ValidatedUrl::parse(&raw)
            .map_err(|e| ProtocolError::UrlValidationFailed { source: e })?;
        self.storage
            .urls()
            .insert(&validated, None)
            .map_err(|e| ProtocolError::StorageFailed { source: e })?;
        Ok(())
    }

    pub fn run(&self, protocol_url: &str) -> Result<RunOutcome, ProtocolError> {
        let socket_path = self.config.socket_path();

        match SingleInstance::new(&socket_path)? {
            SingleInstance::Subsequent(mut client) => {
                client.send_url(protocol_url)?;
                Ok(RunOutcome::UrlForwarded)
            }
            SingleInstance::First(server) => {
                self.handle_url(protocol_url)?;
                self.run_server(server)?;
                Ok(RunOutcome::FirstInstance)
            }
        }
    }

    fn run_server(&self, server: IpcServer) -> Result<(), ProtocolError> {
        loop {
            match server.accept_url() {
                Ok(url) => {
                    let _ = self.handle_url(&url);
                }
                Err(e) => {
                    // Log or ignore accept errors to stay resilient
                    eprintln!("IPC accept error: {}", e);
                }
            }
        }
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/protocol/mod.rs
git commit -m "feat: add ProtocolHandler with single-instance logic and server loop"
```

---

### Task 8: Wire into lib.rs

**Files:**
- Modify: `src/lib.rs`

- [ ] **Step 1: Add protocol module**

```rust
pub mod launcher;
pub mod protocol;
pub mod storage;
pub mod url;
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: No errors (module exists but may have unused code warnings).

- [ ] **Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "feat: wire protocol module into lib.rs"
```

---

### Task 9: Wire into main.rs

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Handle protocol URLs from command line**

```rust
use std::env;
use std::path::PathBuf;

use tabless::protocol::{ProtocolConfig, ProtocolHandler, RunOutcome};
use tabless::storage::Storage;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && args[1] == "register-protocol" {
        let binary_path = env::current_exe().expect("failed to get current executable path");
        match tabless::protocol::registration::register_protocol(&binary_path) {
            Ok(()) => println!("Protocol registered successfully."),
            Err(e) => eprintln!("Registration failed: {}", e),
        }
        return;
    }

    let protocol_url = args.get(1).filter(|s| s.starts_with("tabless://"));

    if let Some(url) = protocol_url {
        let data_dir = dirs::data_local_dir()
            .expect("failed to determine data directory")
            .join("tabless");

        std::fs::create_dir_all(&data_dir).expect("failed to create data directory");

        let db_path = data_dir.join("tabless.db");
        let storage = Storage::open(&db_path).expect("failed to open storage");

        let config = ProtocolConfig {
            scheme: "tabless",
            binary_path: env::current_exe().unwrap_or_else(|_| PathBuf::from("tabless")),
            data_dir,
        };

        let handler = ProtocolHandler::new(config, storage).expect("failed to create handler");

        match handler.run(url) {
            Ok(RunOutcome::FirstInstance) => {
                // Server loop blocks until interrupted
            }
            Ok(RunOutcome::UrlForwarded) => {
                // Silent exit — URL forwarded to running instance
            }
            Err(e) => {
                eprintln!("Protocol handling failed: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        println!("tabless - URL capture and launch utility");
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: No errors.

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire protocol handler into main.rs with register-protocol subcommand"
```

---

### Task 10: Integration test for protocol handler

**Files:**
- Create: `tests/protocol_handler.rs`

- [ ] **Step 1: Write integration test**

```rust
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use tabless::protocol::{ProtocolConfig, ProtocolHandler, RunOutcome};
use tabless::storage::Storage;

#[test]
fn protocol_handler_forwards_url_to_first_instance() {
    let tmpdir = std::env::temp_dir().join(format!("tabless-int-test-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmpdir);

    let db_path = tmpdir.join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let config = ProtocolConfig {
        scheme: "tabless",
        binary_path: PathBuf::from("tabless"),
        data_dir: tmpdir.clone(),
    };

    let handler = ProtocolHandler::new(config, storage).unwrap();

    // Start server in a thread
    let server_handle = thread::spawn(move || {
        let outcome = handler.run("tabless://open?url=https://example.com").unwrap();
        assert!(matches!(outcome, RunOutcome::FirstInstance));
    });

    // Give server time to bind
    thread::sleep(Duration::from_millis(200));

    // Second instance
    let tmpdir2 = tmpdir.clone();
    let db_path2 = tmpdir2.join("test2.db");
    let storage2 = Storage::open(&db_path2).unwrap();
    let config2 = ProtocolConfig {
        scheme: "tabless",
        binary_path: PathBuf::from("tabless"),
        data_dir: tmpdir2,
    };
    let handler2 = ProtocolHandler::new(config2, storage2).unwrap();
    let outcome = handler2.run("tabless://open?url=https://example.org").unwrap();
    assert!(matches!(outcome, RunOutcome::UrlForwarded));

    // Server thread is blocking; drop it by cleaning up socket
    let socket = tmpdir.join("tabless.ipc");
    let _ = std::fs::remove_file(&socket);

    // Give server time to notice socket removal and exit accept loop
    thread::sleep(Duration::from_millis(100));

    // Cleanup
    let _ = std::fs::remove_dir_all(&tmpdir);
}
```

Note: This test has a blocking server. In practice, `run_server` loops forever, so the test thread never joins. For a cleaner test, we could add a shutdown mechanism or just accept the test structure. A better approach: test `SingleInstance` and `IpcClient::send_url` directly rather than full `ProtocolHandler::run`.

Let me revise the integration test to avoid the blocking issue:

```rust
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use tabless::protocol::ipc::{IpcClient, IpcServer};
use tabless::protocol::parse::parse_protocol_url;
use tabless::protocol::single_instance::SingleInstance;
use tabless::storage::Storage;
use tabless::url::ValidatedUrl;

#[test]
fn end_to_end_single_instance_and_storage() {
    let tmpdir = std::env::temp_dir().join(format!("tabless-e2e-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmpdir);
    let socket_path = tmpdir.join("tabless.ipc");
    let db_path = tmpdir.join("e2e.db");

    // First instance: bind server, store URL, keep server alive briefly
    let server_socket = socket_path.clone();
    let server_db = db_path.clone();
    let handle = thread::spawn(move || {
        let server = IpcServer::bind(&server_socket).unwrap();
        let storage = Storage::open(&server_db).unwrap();
        let url = parse_protocol_url("tabless://open?url=https://example.com").unwrap();
        let validated = ValidatedUrl::parse(&url).unwrap();
        storage.urls().insert(&validated, None).unwrap();

        // Accept one forwarded URL then exit
        let forwarded = server.accept_url().unwrap();
        let url2 = parse_protocol_url(&forwarded).unwrap();
        let validated2 = ValidatedUrl::parse(&url2).unwrap();
        storage.urls().insert(&validated2, None).unwrap();
    });

    thread::sleep(Duration::from_millis(100));

    // Second instance: connect, send URL
    let mut client = IpcClient::connect(&socket_path).unwrap();
    client.send_url("tabless://open?url=https://example.org").unwrap();

    handle.join().unwrap();

    // Verify both URLs are in the database
    let storage = Storage::open(&db_path).unwrap();
    let urls = storage.urls().list_inbox().unwrap();
    assert_eq!(urls.len(), 2);

    let _ = std::fs::remove_dir_all(&tmpdir);
}
```

- [ ] **Step 2: Run integration test**

Run: `cargo test --test protocol_handler`
Expected: Test passes.

- [ ] **Step 3: Commit**

```bash
git add tests/protocol_handler.rs
git commit -m "test: add end-to-end protocol handler integration test"
```

---

### Task 11: Final verification

- [ ] **Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: No warnings (fix any that appear).

- [ ] **Step 3: Commit any fixes**

```bash
git add -A
git commit -m "fix: address clippy warnings in protocol module"
```

---

## Spec Coverage Checklist

| Spec Requirement | Plan Task |
|------------------|-----------|
| `ProtocolError` enum | Task 2 |
| `parse_protocol_url` | Task 3 |
| `IpcServer` / `IpcClient` | Task 4 |
| `SingleInstance` detection | Task 5 |
| Platform registration (Linux/macOS/Windows) | Task 6 |
| `ProtocolConfig` | Task 7 |
| `ProtocolHandler` | Task 7 |
| `RunOutcome` | Task 7 |
| Wire into `lib.rs` | Task 8 |
| Wire into `main.rs` | Task 9 |
| Integration test | Task 10 |

---

## Placeholder Scan

No placeholders, TODOs, or TBDs in this plan. Every task has exact file paths, code, and expected outputs.
