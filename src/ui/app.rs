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
            Ok(urls) => {
                self.urls = urls;
                self.error_message = None;
            }
            Err(e) => {
                eprintln!("Failed to load inbox: {}", e);
                self.error_message = Some(format!("Failed to load inbox: {}", e));
            }
        }
    }

    pub fn apply_actions(&mut self, actions: Vec<ViewAction>) {
        self.error_message = None;
        for action in actions {
            let result = match action {
                ViewAction::Archive(id) => self.storage.urls().set_archived(id, true),
                ViewAction::Pin(id) => self.storage.urls().set_pinned(id, true),
                ViewAction::Delete(id) => self.storage.urls().delete(id),
                ViewAction::Launch(id) => {
                    match self.storage.urls().find_by_id(id) {
                        Ok(Some(record)) => {
                            println!("Launching: {}", record.canonical_url);
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
        let actions = {
            let filtered = self.inbox_state.filtered_items(&self.urls);

            if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                self.inbox_state.navigate_up(filtered.len());
            }
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                self.inbox_state.navigate_down(filtered.len());
            }

            let mut actions = Vec::new();
            if let Some(record) = filtered.get(self.inbox_state.selected_index) {
                if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                    actions.push(ViewAction::Launch(record.id));
                }
                if ctx.input(|i| i.key_pressed(egui::Key::A)) {
                    actions.push(ViewAction::Archive(record.id));
                }
                if ctx.input(|i| i.key_pressed(egui::Key::P)) {
                    actions.push(ViewAction::Pin(record.id));
                }
                if ctx.input(|i| i.key_pressed(egui::Key::D)) {
                    actions.push(ViewAction::Delete(record.id));
                }
            }

            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.inbox_state.search_query.clear();
                self.inbox_state.selected_index = 0;
            }

            actions
        };

        if !actions.is_empty() {
            self.apply_actions(actions);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Inbox");
            ui.separator();

            if let Some(ref msg) = self.error_message {
                ui.colored_label(egui::Color32::RED, msg);
            }

            let view_actions = inbox_view(ui, &mut self.inbox_state, &self.urls);
            if !view_actions.is_empty() {
                self.apply_actions(view_actions);
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
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("tabless-test-{}-{}", std::process::id(), n));
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
