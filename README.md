# Tabless

Tabless is a lightweight, cross-platform URL capture and launch utility designed to reduce browser memory usage and eliminate dependency on persistent browser tabs as a long-term memory system.

## Supported Platforms

- Linux
- macOS
- Windows

## Build Instructions

```bash
cargo build --release
```

### System Dependencies

**Linux:**

- `libx11-dev`
- `libwayland-dev`
- `libxkbcommon-dev`
- `libegl1-mesa-dev` (required for egui compilation)

Install on Debian/Ubuntu:

```bash
sudo apt-get install libx11-dev libwayland-dev libxkbcommon-dev libegl1-mesa-dev
```

**macOS:** No additional system dependencies required.

**Windows:** No additional system dependencies required.

## Protocol Registration

To register Tabless as the default handler for `tabless://` URLs:

```bash
cargo run -- register-protocol
```

### Platform-Specific Notes

- **Linux:** Registers a `.desktop` entry using `xdg-mime` so that `tabless://` links are handled by the application.
- **macOS:** Creates a `Tabless.app` wrapper in `~/Applications` with a `CFBundleURLTypes` entry in `Info.plist`; updates LaunchServices via `lsregister`.
- **Windows:** Writes registry entries under `HKEY_CLASSES_ROOT\tabless` to associate the scheme with the binary.

## Development Workflow

- **Branch model:** `main` for stable releases, `develop` for active development.
- **Run tests:** `cargo test`
- **Formatting:** `cargo fmt --check`
- **Linting:** `cargo clippy -- -D warnings`

All changes must pass formatting and linting before merging.

## Testing Guidance

- **Unit tests:** Run with `cargo test`. These cover URL validation, storage operations, protocol parsing, and launcher logic.
- **Integration tests:** Located in the `tests/` directory. These exercise storage persistence, URL integration workflows, and end-to-end protocol handler behavior including single-instance detection and IPC forwarding.
- **Manual runtime validation:** After building, verify the following behaviors:
  - Single-instance enforcement: launching a second instance forwards the URL to the running instance via IPC.
  - Protocol capture: a `tabless://` URL received while the app is running is stored in the inbox.
  - IPC forwarding: subsequent instances correctly pass URLs to the first instance's server loop.
  - Launcher integration: URLs can be launched into the system's default browser.

## Architecture

For a detailed overview of the system design, philosophy, and technology choices, see [ARCHITECTURE.md](ARCHITECTURE.md).
