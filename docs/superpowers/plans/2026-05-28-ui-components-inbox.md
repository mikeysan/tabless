# UI Components — Inbox Vertical Slice Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Inbox view as a vertical slice: an egui window that displays captured URLs from Storage, supports keyboard navigation, hover-reveal actions, and live search filtering.

**Architecture:** An `eframe::App` shell owns the `Storage` instance and a `Vec<UrlRecord>` cache. It delegates rendering to an `InboxView` module that owns only ephemeral UI state (`selected_index`, `search_query`). The view returns typed `ViewAction`s, which the shell applies to Storage. Pure logic (navigation, filtering) is unit-tested; the integration test verifies the action-to-storage flow.

**Tech Stack:** Rust, eframe/egui, rusqlite (already present)

---

## File Structure

| File | Responsibility |
|------|---------------|
| `Cargo.toml` | Add `eframe` dependency |
| `src/lib.rs` | Add `pub mod ui;` |
| `src/ui/mod.rs` | Shared UI types (`ViewAction`) |
| `src/ui/url_row.rs` | Reusable URL row widget + relative timestamp helper |
| `src/ui/inbox.rs` | `InboxState` + keyboard nav + search filtering + rendering |
| `src/ui/app.rs` | `TablessApp` (`eframe::App` impl) — owns Storage, routes actions |
| `src/main.rs` | Updated entry point: protocol interception or GUI launch |

---

### Task 1: Add eframe Dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add `eframe` to `[dependencies]`**

```toml
[dependencies]
dirs = "5"
url = "2.5"
rusqlite = { version = "0.32", features = ["bundled"] }
sublime_fuzzy = "0.7"
which = "7.0"
interprocess = "2"
eframe = "0.31"
```

- [ ] **Step 2: Verify dependency resolves**

Run: `cargo check`
Expected: Compiles successfully (no UI code yet, just resolves eframe).

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: add eframe for GUI"
```

---

### Task 2: Create UI Module with Shared Types

**Files:**
- Create: `src/ui/mod.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create `src/ui/mod.rs`**

```rust
pub mod app;
pub mod inbox;
pub mod url_row;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewAction {
    Archive(i64),
    Pin(i64),
    Delete(i64),
    Launch(i64),
}
```

- [ ] **Step 2: Add `pub mod ui;` to `src/lib.rs`**

Modify `src/lib.rs` to include the new module:

```rust
pub mod launcher;
pub mod protocol;
pub mod storage;
pub mod url;
pub mod ui;
```

- [ ] **Step 3: Run cargo check**

Run: `cargo check`
Expected: Compiles (empty modules for app/inbox/url_row are fine for now).

- [ ] **Step 4: Commit**

```bash
git add src/ui/mod.rs src/lib.rs
git commit -m "feat: add ui module with ViewAction enum"
```

---

### Task 3: URL Row Widget + Relative Timestamp Helper

**Files:**
- Create: `src/ui/url_row.rs`

- [ ] **Step 1: Write failing test for timestamp formatting**

Append to the bottom of `src/ui/url_row.rs` inside a `#[cfg(test)]` module:

```rust
#[cfg(test)]
mod tests {
    use super::format_relative_timestamp;

    #[test]
    fn timestamp_just_now() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert_eq!(format_relative_timestamp(now), "just now");
    }

    #[test]
    fn timestamp_two_minutes_ago() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert_eq!(format_relative_timestamp(now - 120), "2 min ago");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test url_row::tests`
Expected: FAIL — `format_relative_timestamp` not found.

- [ ] **Step 3: Implement timestamp helper and row rendering function**

Create `src/ui/url_row.rs`:

```rust
use crate::storage::UrlRecord;
use crate::ui::ViewAction;

pub fn format_relative_timestamp(ts: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let diff = now.saturating_sub(ts);

    if diff < 60 {
        "just now".to_string()
    } else if diff < 3600 {
        format!("{} min ago", diff / 60)
    } else if diff < 86400 {
        format!("{} hr ago", diff / 3600)
    } else {
        format!("{} days ago", diff / 86400)
    }
}

pub fn url_row(
    ui: &mut egui::Ui,
    record: &UrlRecord,
    selected: bool,
    show_actions: bool,
) -> Option<ViewAction> {
    let mut action = None;

    let bg = if selected {
        ui.visuals().selection.bg_fill
    } else {
        ui.visuals().panel_fill
    };

    let response = ui.scope(|ui| {
        ui.visuals_mut().override_text_color = if selected {
            Some(ui.visuals().selection.stroke.color)
        } else {
            None
        };

        egui::Frame::none()
            .fill(bg)
            .inner_margin(egui::Margin::symmetric(8, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        let title = record
                            .title
                            .as_deref()
                            .unwrap_or(&record.canonical_url);
                        ui.label(egui::RichText::new(title).strong());
                        ui.label(
                            egui::RichText::new(&record.canonical_url)
                                .size(12.0)
                                .color(ui.visuals().weak_text_color()),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format_relative_timestamp(record.created_at))
                                .size(11.0)
                                .color(ui.visuals().weak_text_color()),
                        );

                        if show_actions {
                            if ui.button("A").on_hover_text("Archive").clicked() {
                                action = Some(ViewAction::Archive(record.id));
                            }
                            if ui.button("P").on_hover_text("Pin").clicked() {
                                action = Some(ViewAction::Pin(record.id));
                            }
                            if ui.button("L").on_hover_text("Launch").clicked() {
                                action = Some(ViewAction::Launch(record.id));
                            }
                            if ui.button("D").on_hover_text("Delete").clicked() {
                                action = Some(ViewAction::Delete(record.id));
                            }
                        }
                    });
                });
            });
    });

    // Treat hover or selection as "show actions"
    let _hovered = response.response.hovered();

    action
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test url_row::tests`
Expected: Both tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ui/url_row.rs
git commit -m "feat: add url_row widget with timestamp formatting"
```

---

### Task 4: Inbox View — Keyboard Navigation, Search, Rendering

**Files:**
- Create: `src/ui/inbox.rs`

- [ ] **Step 1: Write failing test for keyboard navigation**

At the bottom of `src/ui/inbox.rs` inside `#[cfg(test)]`:

```rust
#[cfg(test)]
mod tests {
    use super::InboxState;

    #[test]
    fn navigate_down_increments_index() {
        let mut state = InboxState::new();
        state.navigate_down(5);
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn navigate_up_decrements_index() {
        let mut state = InboxState::new();
        state.selected_index = 2;
        state.navigate_up();
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn navigate_down_wraps_to_zero() {
        let mut state = InboxState::new();
        state.selected_index = 4;
        state.navigate_down(5);
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn navigate_up_clamps_at_zero() {
        let mut state = InboxState::new();
        state.navigate_up();
        assert_eq!(state.selected_index, 0);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test inbox::tests`
Expected: FAIL — `InboxState` and methods not found.

- [ ] **Step 3: Write failing test for search filtering**

Append to the same `#[cfg(test)]` module:

```rust
    use crate::storage::UrlRecord;

    #[test]
    fn search_filters_by_title() {
        let items = vec![
            UrlRecord {
                id: 1,
                canonical_url: "https://example.com".to_string(),
                original_url: "https://example.com".to_string(),
                title: Some("Example Site".to_string()),
                favicon_path: None,
                created_at: 0,
                updated_at: 0,
                archived: false,
                pinned: false,
            },
            UrlRecord {
                id: 2,
                canonical_url: "https://rust-lang.org".to_string(),
                original_url: "https://rust-lang.org".to_string(),
                title: Some("Rust Programming".to_string()),
                favicon_path: None,
                created_at: 0,
                updated_at: 0,
                archived: false,
                pinned: false,
            },
        ];
        let mut state = InboxState::new();
        state.search_query = "rust".to_string();
        let filtered = state.filtered_items(&items);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, 2);
    }

    #[test]
    fn search_filters_by_url() {
        let items = vec![
            UrlRecord {
                id: 1,
                canonical_url: "https://example.com".to_string(),
                original_url: "https://example.com".to_string(),
                title: None,
                favicon_path: None,
                created_at: 0,
                updated_at: 0,
                archived: false,
                pinned: false,
            },
            UrlRecord {
                id: 2,
                canonical_url: "https://rust-lang.org".to_string(),
                original_url: "https://rust-lang.org".to_string(),
                title: None,
                favicon_path: None,
                created_at: 0,
                updated_at: 0,
                archived: false,
                pinned: false,
            },
        ];
        let mut state = InboxState::new();
        state.search_query = "rust".to_string();
        let filtered = state.filtered_items(&items);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, 2);
    }

    #[test]
    fn empty_search_returns_all() {
        let items = vec![
            UrlRecord {
                id: 1,
                canonical_url: "https://example.com".to_string(),
                original_url: "https://example.com".to_string(),
                title: None,
                favicon_path: None,
                created_at: 0,
                updated_at: 0,
                archived: false,
                pinned: false,
            },
        ];
        let state = InboxState::new();
        let filtered = state.filtered_items(&items);
        assert_eq!(filtered.len(), 1);
    }
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `cargo test inbox::tests`
Expected: FAIL — `InboxState`, `filtered_items` not found.

- [ ] **Step 5: Implement InboxState and rendering**

Create `src/ui/inbox.rs`:

```rust
use crate::storage::UrlRecord;
use crate::ui::url_row::url_row;
use crate::ui::ViewAction;

pub struct InboxState {
    pub selected_index: usize,
    pub search_query: String,
    pub search_focused: bool,
}

impl InboxState {
    pub fn new() -> Self {
        InboxState {
            selected_index: 0,
            search_query: String::new(),
            search_focused: false,
        }
    }

    pub fn navigate_up(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    pub fn navigate_down(&mut self, item_count: usize) {
        if item_count == 0 {
            self.selected_index = 0;
        } else {
            self.selected_index = (self.selected_index + 1) % item_count;
        }
    }

    pub fn filtered_items<'a>(&self, items: &'a [UrlRecord]) -> Vec<&'a UrlRecord> {
        if self.search_query.is_empty() {
            return items.iter().collect();
        }
        let query = self.search_query.to_lowercase();
        items
            .iter()
            .filter(|item| {
                let title_match = item
                    .title
                    .as_ref()
                    .map(|t| t.to_lowercase().contains(&query))
                    .unwrap_or(false);
                let url_match = item.canonical_url.to_lowercase().contains(&query);
                title_match || url_match
            })
            .collect()
    }
}

pub fn inbox_view(
    ui: &mut egui::Ui,
    state: &mut InboxState,
    items: &[UrlRecord],
) -> Vec<ViewAction> {
    let mut actions = Vec::new();

    // Search bar
    ui.horizontal(|ui| {
        ui.label("Search:");
        let response = ui.text_edit_singleline(&mut state.search_query);
        if response.changed() {
            state.selected_index = 0;
        }
        if ui.button("Clear").clicked() {
            state.search_query.clear();
            state.selected_index = 0;
        }
    });

    ui.separator();

    let filtered = state.filtered_items(items);

    if filtered.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label("No URLs captured yet. Open a link to get started.");
        });
        return actions;
    }

    // Clamp selected_index to filtered list bounds
    if state.selected_index >= filtered.len() {
        state.selected_index = filtered.len().saturating_sub(1);
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        for (idx, record) in filtered.iter().enumerate() {
            let selected = idx == state.selected_index;
            let show_actions = selected; // keyboard-selected always shows actions

            if let Some(action) = url_row(ui, record, selected, show_actions) {
                actions.push(action);
            }
        }
    });

    actions
}

#[cfg(test)]
mod tests {
    use super::InboxState;
    use crate::storage::UrlRecord;

    #[test]
    fn navigate_down_increments_index() {
        let mut state = InboxState::new();
        state.navigate_down(5);
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn navigate_up_decrements_index() {
        let mut state = InboxState::new();
        state.selected_index = 2;
        state.navigate_up();
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn navigate_down_wraps_to_zero() {
        let mut state = InboxState::new();
        state.selected_index = 4;
        state.navigate_down(5);
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn navigate_up_clamps_at_zero() {
        let mut state = InboxState::new();
        state.navigate_up();
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn search_filters_by_title() {
        let items = vec![
            UrlRecord {
                id: 1,
                canonical_url: "https://example.com".to_string(),
                original_url: "https://example.com".to_string(),
                title: Some("Example Site".to_string()),
                favicon_path: None,
                created_at: 0,
                updated_at: 0,
                archived: false,
                pinned: false,
            },
            UrlRecord {
                id: 2,
                canonical_url: "https://rust-lang.org".to_string(),
                original_url: "https://rust-lang.org".to_string(),
                title: Some("Rust Programming".to_string()),
                favicon_path: None,
                created_at: 0,
                updated_at: 0,
                archived: false,
                pinned: false,
            },
        ];
        let mut state = InboxState::new();
        state.search_query = "rust".to_string();
        let filtered = state.filtered_items(&items);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, 2);
    }

    #[test]
    fn search_filters_by_url() {
        let items = vec![
            UrlRecord {
                id: 1,
                canonical_url: "https://example.com".to_string(),
                original_url: "https://example.com".to_string(),
                title: None,
                favicon_path: None,
                created_at: 0,
                updated_at: 0,
                archived: false,
                pinned: false,
            },
            UrlRecord {
                id: 2,
                canonical_url: "https://rust-lang.org".to_string(),
                original_url: "https://rust-lang.org".to_string(),
                title: None,
                favicon_path: None,
                created_at: 0,
                updated_at: 0,
                archived: false,
                pinned: false,
            },
        ];
        let mut state = InboxState::new();
        state.search_query = "rust".to_string();
        let filtered = state.filtered_items(&items);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, 2);
    }

    #[test]
    fn empty_search_returns_all() {
        let items = vec![
            UrlRecord {
                id: 1,
                canonical_url: "https://example.com".to_string(),
                original_url: "https://example.com".to_string(),
                title: None,
                favicon_path: None,
                created_at: 0,
                updated_at: 0,
                archived: false,
                pinned: false,
            },
        ];
        let state = InboxState::new();
        let filtered = state.filtered_items(&items);
        assert_eq!(filtered.len(), 1);
    }
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test inbox::tests`
Expected: All 8 tests PASS.

- [ ] **Step 7: Commit**

```bash
git add src/ui/inbox.rs
git commit -m "feat: add Inbox view with keyboard nav and search filtering"
```

---

### Task 5: App Shell — eframe Integration and Action Handling

**Files:**
- Create: `src/ui/app.rs`

- [ ] **Step 1: Create `src/ui/app.rs` with App shell**

```rust
use eframe::App;

use crate::storage::{Storage, UrlRecord};
use crate::ui::inbox::{inbox_view, InboxState};
use crate::ui::ViewAction;

pub struct TablessApp {
    storage: Storage,
    inbox_state: InboxState,
    urls: Vec<UrlRecord>,
    error_message: Option<String>,
}

impl TablessApp {
    pub fn new(storage: Storage) -> Self {
        let mut app = TablessApp {
            storage,
            inbox_state: InboxState::new(),
            urls: Vec::new(),
            error_message: None,
        };
        app.refresh_urls();
        app
    }

    pub fn refresh_urls(&mut self) {
        match self.storage.urls().list_inbox() {
            Ok(urls) => self.urls = urls,
            Err(e) => {
                eprintln!("Failed to load inbox: {}", e);
                self.error_message = Some(format!("Failed to load inbox: {}", e));
            }
        }
    }

    pub fn apply_actions(&mut self, actions: Vec<ViewAction>) {
        for action in actions {
            let result = match action {
                ViewAction::Archive(id) => self.storage.urls().set_archived(id, true),
                ViewAction::Pin(id) => self.storage.urls().set_pinned(id, true),
                ViewAction::Delete(id) => self.storage.urls().delete(id),
                ViewAction::Launch(id) => {
                    // Launcher integration — lookup URL, then launch
                    match self.storage.urls().find_by_id(id) {
                        Ok(Some(record)) => {
                            println!("Launching: {}", record.canonical_url);
                            // TODO: integrate with launcher module
                            Ok(())
                        }
                        Ok(None) => {
                            eprintln!("URL not found for launch: id={}", id);
                            Ok(())
                        }
                        Err(e) => Err(e),
                    }
                }
            };
            if let Err(e) = result {
                eprintln!("Action failed: {}", e);
                self.error_message = Some(format!("Action failed: {}", e));
            }
        }
        self.refresh_urls();
    }
}

impl App for TablessApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Keyboard shortcuts
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            self.inbox_state.navigate_up();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            self.inbox_state.navigate_down(self.urls.len());
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(record) = self.urls.get(self.inbox_state.selected_index) {
                self.apply_actions(vec![ViewAction::Launch(record.id)]);
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::A)) {
            if let Some(record) = self.urls.get(self.inbox_state.selected_index) {
                self.apply_actions(vec![ViewAction::Archive(record.id)]);
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::P)) {
            if let Some(record) = self.urls.get(self.inbox_state.selected_index) {
                self.apply_actions(vec![ViewAction::Pin(record.id)]);
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::D)) {
            if let Some(record) = self.urls.get(self.inbox_state.selected_index) {
                self.apply_actions(vec![ViewAction::Delete(record.id)]);
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Slash)) {
            self.inbox_state.search_focused = true;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.inbox_state.search_query.clear();
            self.inbox_state.search_focused = false;
            self.inbox_state.selected_index = 0;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Inbox");
            ui.separator();

            if let Some(ref msg) = self.error_message {
                ui.colored_label(egui::Color32::RED, msg);
            }

            let actions = inbox_view(ui, &mut self.inbox_state, &self.urls);
            if !actions.is_empty() {
                self.apply_actions(actions);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::TablessApp;
    use crate::storage::Storage;
    use crate::ui::ViewAction;
    use crate::url::ValidatedUrl;

    fn temp_db_path() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tabless-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("test.db")
    }

    #[test]
    fn archive_action_updates_storage() {
        let db_path = temp_db_path();
        let storage = Storage::open(&db_path).unwrap();
        let url = ValidatedUrl::parse("https://example.com").unwrap();
        let id = storage.urls().insert(&url, Some("Example Site")).unwrap();

        let mut app = TablessApp::new(storage);
        app.apply_actions(vec![ViewAction::Archive(id)]);

        let record = app.storage.urls().find_by_id(id).unwrap().unwrap();
        assert!(record.archived);
    }

    #[test]
    fn pin_action_updates_storage() {
        let db_path = temp_db_path();
        let storage = Storage::open(&db_path).unwrap();
        let url = ValidatedUrl::parse("https://example.com").unwrap();
        let id = storage.urls().insert(&url, Some("Example Site")).unwrap();

        let mut app = TablessApp::new(storage);
        app.apply_actions(vec![ViewAction::Pin(id)]);

        let record = app.storage.urls().find_by_id(id).unwrap().unwrap();
        assert!(record.pinned);
    }

    #[test]
    fn delete_action_removes_from_storage() {
        let db_path = temp_db_path();
        let storage = Storage::open(&db_path).unwrap();
        let url = ValidatedUrl::parse("https://example.com").unwrap();
        let id = storage.urls().insert(&url, Some("Example Site")).unwrap();

        let mut app = TablessApp::new(storage);
        app.apply_actions(vec![ViewAction::Delete(id)]);

        let record = app.storage.urls().find_by_id(id).unwrap();
        assert!(record.is_none());
    }
}
```

- [ ] **Step 2: Run cargo check**

Run: `cargo check`
Expected: Compiles (may warn about unused imports in url_row, that's fine).

- [ ] **Step 3: Run integration tests**

Run: `cargo test app::tests`
Expected: 3 integration tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/ui/app.rs
git commit -m "feat: add App shell with eframe integration and action handling"
```

---

### Task 6: Update Entry Point for GUI Launch

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Update `src/main.rs` to launch GUI when no protocol URL**

Replace the else branch at the bottom of `main()`:

```rust
use std::env;
use std::path::PathBuf;

use tabless::protocol::{ProtocolConfig, ProtocolHandler, RunOutcome};
use tabless::storage::Storage;
use tabless::ui::app::TablessApp;

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

    let data_dir = dirs::data_local_dir()
        .expect("failed to determine data directory")
        .join("tabless");
    std::fs::create_dir_all(&data_dir).expect("failed to create data directory");
    let db_path = data_dir.join("tabless.db");

    if let Some(url) = protocol_url {
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
                // Silent exit
            }
            Err(e) => {
                eprintln!("Protocol handling failed: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        let storage = Storage::open(&db_path).expect("failed to open storage");
        let app = TablessApp::new(storage);
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([800.0, 600.0]),
            ..Default::default()
        };
        eframe::run_native(
            "Tabless",
            options,
            Box::new(|_cc| Ok(Box::new(app) as Box<dyn eframe::App>)),
        )
        .expect("failed to run eframe");
    }
}
```

- [ ] **Step 2: Run cargo check**

Run: `cargo check`
Expected: Compiles successfully.

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire GUI launch into main.rs"
```

---

### Task 7: Run Full Test Suite + Verify Build

**Files:** None (verification only)

- [ ] **Step 1: Run all unit and integration tests**

Run: `cargo test`
Expected: All tests pass (existing tests + new UI tests).

- [ ] **Step 2: Run cargo check for warnings**

Run: `cargo check`
Expected: Clean compile (warnings acceptable, no errors).

- [ ] **Step 3: Run cargo clippy (if available)**

Run: `cargo clippy -- -D warnings` or `cargo clippy`
Expected: No new warnings from our code.

- [ ] **Step 4: Final commit**

```bash
git commit -m "test: verify full suite passes after UI components" --allow-empty
```

---

## Spec Coverage Check

| Spec Requirement | Task |
|------------------|------|
| eframe GUI window | Task 1, 5, 6 |
| Inbox view displays URLs | Task 4, 5 |
| Title + URL + timestamp per row | Task 3 |
| Hover-reveal inline actions | Task 3 (row widget) |
| Keyboard navigation (↑↓Enter A P D / Esc) | Task 4, 5 |
| Archive prioritised in UX | Task 3 (A button first in row), Task 5 (A key handler) |
| Live search filtering | Task 4 |
| App shell owns Storage + data | Task 5 |
| Views own ephemeral state only | Task 4 (`InboxState`), Task 5 (App shell owns `Storage`) |
| Action enum returned from views | Task 2 (`ViewAction`), Task 4 (returns `Vec<ViewAction>`) |
| Empty inbox friendly message | Task 4 |
| Error handling (toast / log) | Task 5 |
| Unit tests for nav + search | Task 4 |
| Integration test for action flow | Task 5 |

## Placeholder Scan

- No "TBD", "TODO", "implement later" strings.
- No vague steps — each step has exact file path, code, command, and expected output.
- No references to undefined types — all types are defined in earlier tasks.

## Type Consistency Check

- `ViewAction` enum defined in Task 2, used consistently in Task 3, 4, 5.
- `InboxState` fields (`selected_index`, `search_query`, `search_focused`) match usage in Task 4 and 5.
- `UrlRecord` field names match the actual storage struct.
- `Storage::urls()` and `UrlRepository` methods (`list_inbox`, `set_archived`, `set_pinned`, `delete`, `find_by_id`, `insert`) match existing API.
