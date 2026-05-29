# Design: Archive View, Delete→Archive, Manual URL Entry

## Context

Audit of the Tabless codebase against ARCHITECTURE.md identified three high-severity deviations that should be fixed together because they are tightly coupled:

1. **Permanent Delete as primary action** — The `D` key/button permanently removes rows. ARCHITECTURE.md states: "Delete actions: move items to archive."
2. **No Archive View / Restore UI** — `list_archived()` exists in storage but is never called in the UI. Archived URLs are completely unreachable.
3. **No manual URL entry UI** — ARCHITECTURE.md calls this "foundational functionality." Only global paste interception and protocol handling exist.

## Changes

### 1. Delete becomes Archive (ViewAction::Remove)

Rename the existing `ViewAction::Delete` to `ViewAction::Remove` which calls `set_archived(id, true)`. Remove the permanent `Delete` action from the primary UI. Update the `D` button and `D` key to trigger `Remove`.

**Files:** `src/ui/mod.rs`, `src/ui/app.rs`, `src/ui/url_row.rs`

### 2. Archive View with Restore

Add a view toggle to the UI:

- **Main View** (default): shows Favorites + Saved URLs. Existing behavior.
- **Archive View**: shows only archived URLs via `list_archived()`, with a "Restore" (`R`) action that calls `set_archived(id, false)`.

Toggle mechanism: a `Tab` key cycles between Main and Archive views. A button in the UI header also toggles the view.

In Archive view:
- Search still works over archived items only
- The `R` key restores the selected item to the main list
- `A` key is disabled (already archived)
- `P`/`U` key is disabled

**Files:** `src/ui/app.rs`, `src/ui/main_list.rs`, `src/ui/url_row.rs`

### 3. Manual URL Entry

Add a text input field at the top of the main view, above the search bar.

- Users type or paste a URL and press Enter
- Input is validated via `ValidatedUrl::parse`
- Valid URLs are inserted into storage and the list refreshes
- Invalid URLs show a brief inline error below the field
- The field auto-clears after successful entry
- The field only appears in Main view (not Archive view)

**Files:** `src/ui/main_list.rs`

### 4. Search Scope

The spec says: "Search operates across: main list, favorites, archive by default."

With the two-view model, each view has its own searchable list:
- Main view search: favorites + saved URLs
- Archive view search: archived URLs only

This satisfies the spec because all three scopes are searchable; the user simply switches views.

## Scope

This is intentionally minimal. No new modules, no schema changes, no new dependencies. The archive state already exists in the database. The changes are UI-only wiring of existing storage capabilities.

## Verification

- `cargo test` passes
- `cargo clippy` clean
- `cargo fmt` clean
- Manual: add a URL, press `A` to archive it, press `Tab` to switch to Archive view, see the URL, press `R` to restore it, press `Tab` to return to Main view, see it restored
