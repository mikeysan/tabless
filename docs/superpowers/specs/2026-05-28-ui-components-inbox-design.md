# UI Components — Inbox Vertical Slice Design

## Date
2026-05-28

## Scope
Build a single complete view: the **Inbox**.
This is a vertical slice through the UI layer, wired to the existing storage and launcher services.

## Out of Scope
- Pinned view, Archive view, Search view (future views)
- Tray icon / daemon mode
- Settings / preferences UI
- Context menus
- Favicon rendering
- Drag-and-drop

---

## UI Architecture

```
┌─────────────────────────────────────┐
│  App Shell (egui::App)              │
│  ├── owns Storage instance          │
│  ├── owns Vec<UrlRecord> (data)     │
│  ├── routes to active view          │
│  └── handles all storage mutations  │
│                                     │
│  ┌─────────────────────────────┐   │
│  │  InboxView                  │   │
│  │  ├── selected_index: usize  │   │
│  │  ├── hovered_id: Option<i64>│   │
│  │  └── search_query: String   │   │
│  └─────────────────────────────┘   │
│                                     │
│  ┌─────────────────────────────┐   │
│  │  UrlRow widget (reusable)   │   │
│  └─────────────────────────────┘   │
└─────────────────────────────────────┘
```

**Rule:** Views own ephemeral UI state only. Persistent data and storage live in the App shell.

---

## App Shell Behaviour

### Entry Point
When `main.rs` receives a `tabless://` URL:
1. If no instance is running → start IPC server, open GUI window, add URL to Inbox.
2. If instance is running → forward URL via IPC, silently exit.

When the app is launched with no arguments:
- Open GUI window showing Inbox (if not already open).

### Data Refresh
After every storage mutation (archive, pin, delete, insert), the App shell re-queries the inbox list and re-renders.

---

## Inbox View Design

### Layout
- Full-window scrollable list.
- No sidebar, no tabs for this slice — just the Inbox.

### Row Display (per URL)
- **Title** (or canonical URL if no title) — bold, ellipsized
- **Canonical URL** — muted, smaller, ellipsized
- **Relative timestamp** — "2 min ago", "1 hr ago", etc. — right-aligned, muted

### Row Actions (hover-reveal)
When a row is hovered or keyboard-selected, reveal inline action buttons:
1. **Archive** (prioritised — first button)
2. **Pin**
3. **Launch**
4. **Delete** (last, requires confirmation or is undo-less)

Actions are small icon buttons (or text buttons) that appear inline on the right edge of the row.

### Keyboard Navigation
| Key | Action |
|-----|--------|
| ↑ / ↓ | Navigate rows |
| Enter | Launch selected URL in preferred browser |
| A | Archive selected URL |
| P | Pin selected URL |
| D | Delete selected URL |
| / | Focus search bar (filters inbox live) |
| Esc | Clear search, return focus to list |

### Search
- A search input at the top of the view.
- Typing filters the inbox list in real time (fuzzy or substring match on title + URL).
- Searching does not switch to a separate Search view — it filters Inbox only.

---

## Data Flow

### Render Path
```
App::update()
  → load inbox data from Storage (cached after first load)
  → pass &[UrlRecord] and &mut InboxState to InboxView::ui()
  → InboxView renders rows, handles local input
  → returns Vec<ViewAction> to App
  → App applies actions to Storage, refreshes data
```

### Action Enum
```rust
enum ViewAction {
    Archive(i64),
    Pin(i64),
    Delete(i64),
    Launch(i64),
}
```

Views return actions; the App shell executes them against Storage. Views never touch Storage directly.

---

## Error Handling

- Storage errors during mutation → log to stderr, show a brief toast-style message in the UI.
- Empty inbox → show a friendly empty state: "No URLs captured yet. Open a link to get started."
- No selected row + keyboard action → no-op.

---

## Testing

- **Unit tests:** `InboxState` keyboard navigation logic, search filtering logic.
- **Integration test:** Launch app with test database, simulate keyboard events, assert URL state changes in Storage.
- **UI test (optional):** Use `egui`'s test harness if available, or rely on integration tests for render correctness.

---

## File Structure

```
src/
  ui/
    mod.rs          — Ui module, shared types (ViewAction)
    app.rs          — App shell (egui::App impl)
    inbox.rs        — Inbox view state + render
    url_row.rs      — Reusable URL row widget
  main.rs           — Entry point, protocol + GUI routing
```

Dependencies to add:
- `eframe` (egui app framework)
- `egui` (if not pulled in by eframe)
- no additional deps for timestamps; hand-roll relative formatting (minutes/hours/days)

---

## Evolution Path

This design is intentionally constrained to Inbox but leaves clear extension points:
- Add `PinnedView`, `ArchiveView` as sibling modules.
- Add view-switching tabs in the App shell.
- Tray/daemon mode later: the IPC server stays, GUI window becomes optional.
