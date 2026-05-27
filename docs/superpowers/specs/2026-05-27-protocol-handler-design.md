# Protocol Handler Design

## Overview

The protocol handler subsystem registers `tabless://` as a custom URL scheme on the host OS. When another application invokes `tabless://open?url=...`, the app starts (or reuses a running instance), extracts the embedded URL, validates it, stores it in the database, and exits silently.

## Goals

- Register `tabless://` as a custom protocol on Linux, macOS, and Windows.
- Ensure single-instance behaviour: if the app is already running, forward the URL to the running instance and exit.
- Validate the embedded URL using the existing `ValidatedUrl` subsystem before storing.
- Persist the captured URL via the existing `Storage` subsystem.
- Return typed errors for every failure mode.
- Zero panics and zero unwraps in production paths.

## Non-Goals

- Launching the captured URL automatically (that is the launcher service's job; the user can open it later).
- Handling arbitrary deep-link paths (only `tabless://open?url=...` is supported in MVP).
- Unregistration / cleanup of protocol registration (manual for now).
- Bundling the app as a `.app` (macOS) or `.exe` with installer (Windows) — registration assumes a known binary path.

## Module Structure

```
src/
  protocol/
    mod.rs              — public API: ProtocolHandler, ProtocolConfig
    error.rs            — ProtocolError enum
    registration.rs     — register_protocol() for Linux, macOS, Windows
    ipc.rs              — IpcServer (first instance) and IpcClient (subsequent)
    parse.rs            — parse_protocol_url() extracts URL from tabless:// query params
    single_instance.rs  — SingleInstance: bind server or connect client
```

## Key Types

### ProtocolConfig

```rust
pub struct ProtocolConfig {
    pub scheme: &'static str,           // "tabless"
    pub binary_path: PathBuf,           // path to the tabless executable
    pub data_dir: PathBuf,              // platform-specific data directory
}
```

### ProtocolHandler

```rust
pub struct ProtocolHandler {
    storage: Storage,
    single_instance: SingleInstance,
}

impl ProtocolHandler {
    pub fn new(config: &ProtocolConfig, storage: Storage) -> Result<Self, ProtocolError>;
    pub fn run(&self) -> Result<RunOutcome, ProtocolError>;
    pub fn handle_url(&self, url: &str) -> Result<(), ProtocolError>;
}
```

### RunOutcome

```rust
pub enum RunOutcome {
    FirstInstance,      // Server bound, waiting for URLs
    UrlForwarded,       // Connected to existing instance, sent URL, exiting
}
```

## Data Flow

```
Another app invokes:
  tabless://open?url=https://example.com

OS launches tabless binary with the full URI as an argument

SingleInstance::new():
  try_bind_server() -> Ok(IpcServer)     → first instance
    OR
  try_connect_client() -> Ok(IpcClient) → send URL, exit

If first instance (no server is running yet):
  parse_protocol_url(arg) → extract "https://example.com"
  ValidatedUrl::parse(raw) → Result<ValidatedUrl, UrlValidationError>
  Storage::urls::insert(&validated, title=None) → Result<i64, StorageError>
  IpcServer begins accepting connections so future invocations can forward URLs.
  The instance remains alive as an IPC server; the caller controls lifetime.

If subsequent instance:
  IpcClient::send_url(url) → IpcServer receives it
  IpcServer stores URL via same path as above
```

## IPC Protocol

A simple line-based protocol over Unix sockets (Linux/macOS) and named pipes (Windows):

```
CLIENT sends: "URL:https://example.com\n"
SERVER receives line, parses URL, stores it
```

One URL per connection, one line per message. No framing protocol needed.

### IpcServer

```rust
pub struct IpcServer {
    listener: UnixListener,  // Linux/macOS
    // listener: NamedPipeServer,  // Windows
}

impl IpcServer {
    pub fn bind(path: &Path) -> Result<Self, ProtocolError>;
    pub fn accept_url(&self) -> Result<String, ProtocolError>;
}
```

### IpcClient

```rust
pub struct IpcClient;

impl IpcClient {
    pub fn connect(path: &Path) -> Result<Self, ProtocolError>;
    pub fn send_url(&self, url: &str) -> Result<(), ProtocolError>;
}
```

## Platform Registration

### Linux

- Create `~/.local/share/applications/tabless.desktop` with `MimeType=x-scheme-handler/tabless;`.
- Run `xdg-mime default tabless.desktop x-scheme-handler/tabless`.

### macOS

- If bundled as `.app`, add `CFBundleURLTypes` to `Info.plist`.
- For CLI-only, the `tabless register-protocol` command writes a minimal `.app` bundle wrapper that points back to the binary.

### Windows

- Write registry keys under `HKEY_CLASSES_ROOT\tabless`:
  - `URL Protocol` = `""`
  - `shell\open\command\(default)` = `"<binary_path>" "%1"`

## Parse Logic

```rust
pub fn parse_protocol_url(input: &str) -> Result<String, ProtocolError>;
```

- Accepts only `tabless://open?url=<url>`.
- Rejects unknown paths, missing `url` query parameter, or empty URL values.
- Returns the raw URL string for downstream `ValidatedUrl::parse`.

## Error Types

```rust
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
```

Implements `Display` and `std::error::Error`.

## Testing Strategy

- **Unit tests:** Mock `Storage` and `IpcServer`/`IpcClient` to test `ProtocolHandler::handle_url` and `parse_protocol_url` without touching the filesystem.
- **Integration tests:** Use temporary directories for IPC socket paths; spawn two threads (server + client) and verify URL forwarding.
- **Platform tests:** Test `parse_protocol_url` with valid and invalid inputs. Test registration helpers with mocked filesystem/registry operations where possible.

## Security Considerations

- Only the `open?url=` path is accepted. Unknown paths return `InvalidUrl`.
- The embedded URL is validated by `ValidatedUrl::parse` before storage, rejecting `javascript:`, `file:`, and other dangerous schemes.
- IPC socket/pipe is created in the user's data directory, not world-writable.
- No shell is invoked at any point in the protocol handling path.

## Success Criteria

- `tabless://open?url=https://example.com` starts the app and stores the URL.
- A second invocation forwards the URL to the already-running instance and exits.
- Invalid URLs (bad scheme, missing query param) return `InvalidUrl`.
- `ProtocolError` is returned for all failure modes; no panics in production paths.
- Registration succeeds on Linux, macOS, and Windows when run with appropriate permissions.
