# Tabless

Tabless is a lightweight, cross-platform URL capture and launch utility designed to reduce browser memory usage and eliminate dependency on persistent browser tabs as a long-term memory system. It is built as a native desktop GUI application using [egui](https://github.com/emilk/egui).

## Supported Platforms

- Linux
- macOS
- Windows

## Features

- **URL Capture**
  - Paste a valid URL anywhere in the app to instantly save it.
  - Type a URL into the **Add URL** field and press Enter or click **Add**.
  - Receive URLs via the custom `tabless://` protocol handler.
- **URL Management**
  - **Active** and **Archive** tabs keep your inbox clean.
  - **Favorites** are pinned to the top of the active list and can be manually reordered.
  - Real-time search filters by page title or URL.
  - Relative timestamps show when each URL was last updated.
- **Actions**
  - **Launch** a URL in your system's default browser.
  - **Launch in Alternate Browser** (Shift+Enter) when multiple browsers are discovered.
  - **Copy** the canonical URL to the clipboard.
  - **Archive** URLs you want to keep out of the active list.
  - **Restore** archived URLs back to the active list.
- **Single-Instance Enforcement**
  - Launching a second instance forwards the URL to the running instance via IPC and exits silently.
- **Storage**
  - Persistent SQLite database with automatic schema migrations.
  - Tags and collections are tracked in the database schema for future UI support.

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `/` | Focus the search field |
| `Enter` | Launch the selected URL |
| `Shift + Enter` | Open browser picker for the selected URL |
| `L` | Launch the selected URL |
| `C` | Copy the selected URL |
| `A` | Archive the selected URL |
| `F` | Favorite / Unfavorite the selected URL |
| `R` | Restore the selected URL (in Archive view) |
| `Shift + Up` | Move favorite up |
| `Shift + Down` | Move favorite down |
| `Up` / `Down` | Navigate the list |
| `Esc` | Clear search / unfocus input |

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
cargo run --release -- register-protocol
```

### Expected URL Format

The protocol handler expects URLs in this format:

```
tabless://open?url=https://example.com
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
- **Audit:** `cargo audit`
- **License & dependency checks:** `cargo deny check`

All changes must pass formatting, linting, audit, and deny checks before merging.

### Continuous Integration

GitHub Actions run on every PR and push to `main`:

- **check** — formatting and clippy on Ubuntu
- **test** — full test suite on Ubuntu, macOS, and Windows
- **audit** — `cargo audit` for security advisories
- **deny** — `cargo deny` for license and dependency policy compliance

Platform-specific release builds are triggered manually via workflow dispatch.

## Testing Guidance

- **Unit tests:** Run with `cargo test`. These cover URL validation, storage operations, protocol parsing, launcher logic, and UI state management.
- **Integration tests:** Located in the `tests/` directory. These exercise storage persistence, URL integration workflows, and end-to-end protocol handler behavior including single-instance detection and IPC forwarding.
- **Manual runtime validation:** After building, verify the following behaviors:
  - Single-instance enforcement: launching a second instance forwards the URL to the running instance via IPC.
  - Protocol capture: a `tabless://open?url=...` URL received while the app is running is stored in the inbox.
  - IPC forwarding: subsequent instances correctly pass URLs to the first instance's server loop.
  - Launcher integration: URLs can be launched into the system's default browser or an alternate discovered browser.
  - Paste capture: pasting a valid `http` or `https` URL anywhere in the focused app creates a new entry.
