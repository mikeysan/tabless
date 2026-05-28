use eframe::App;

use crate::storage::{Storage, UrlRecord};
use crate::ui::inbox::{inbox_view, InboxState};
use crate::ui::ViewAction;

pub struct TablessApp {
    storage: Storage,
    inbox_state: InboxState,
    urls: Vec<UrlRecord>,
    error_message: Option<String>,
    launcher: Option<Box<dyn crate::launcher::UrlLauncher>>,
    ipc_rx: Option<std::sync::mpsc::Receiver<()>>,
}

impl TablessApp {
    pub fn new(
        storage: Storage,
        launcher: Option<Box<dyn crate::launcher::UrlLauncher>>,
        ipc_rx: Option<std::sync::mpsc::Receiver<()>>,
    ) -> Self {
        let mut app = TablessApp {
            storage,
            inbox_state: InboxState::new(),
            urls: Vec::new(),
            error_message: None,
            launcher,
            ipc_rx,
        };
        app.refresh_urls();
        app
    }

    pub fn refresh_urls(&mut self) {
        match self.storage.urls().list_inbox() {
            Ok(urls) => {
                self.urls = urls;
            }
            Err(e) => {
                eprintln!("Failed to load inbox: {}", e);
                if self.error_message.is_none() {
                    self.error_message = Some(format!("Failed to load inbox: {}", e));
                }
            }
        }
    }

    pub fn apply_actions(&mut self, actions: Vec<ViewAction>) {
        self.error_message = None;
        let mut mutated = false;
        for action in actions {
            let result = match action {
                ViewAction::Archive(id) => {
                    mutated = true;
                    self.storage.urls().set_archived(id, true)
                }
                ViewAction::Pin(id) => {
                    mutated = true;
                    self.storage.urls().set_pinned(id, true)
                }
                ViewAction::Delete(id) => {
                    mutated = true;
                    self.storage.urls().delete(id)
                }
                ViewAction::Launch(id) => {
                    match self.storage.urls().find_by_id(id) {
                        Ok(Some(record)) => {
                            if let Some(ref launcher) = self.launcher {
                                if let Err(e) = launcher.launch(&record.canonical_url) {
                                    self.error_message = Some(format!("Launch failed: {}", e));
                                }
                            } else {
                                self.error_message = Some("No browser configured".to_string());
                            }
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
        if mutated {
            self.refresh_urls();
        }
    }

    fn drain_ipc(&mut self) -> bool {
        let mut notified = false;
        if let Some(ref rx) = self.ipc_rx {
            while let Ok(()) = rx.try_recv() {
                notified = true;
            }
        }
        notified
    }
}

impl App for TablessApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drain IPC notifications and refresh when new URLs arrive
        if self.drain_ipc() {
            self.refresh_urls();
        }

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
                self.inbox_state.search_focused = false;
            }

            if ctx.input(|i| i.key_pressed(egui::Key::Slash)) {
                self.inbox_state.search_focused = true;
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

        let mut app = TablessApp::new(storage, None, None);
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

        let mut app = TablessApp::new(storage, None, None);
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

        let mut app = TablessApp::new(storage, None, None);
        app.apply_actions(vec![ViewAction::Delete(id)]);

        let record = app.storage.urls().find_by_id(id).unwrap();
        assert!(record.is_none());
    }

    #[test]
    fn launch_action_invokes_launcher() {
        use std::sync::{Arc, Mutex};

        struct MockLauncher {
            launched: Arc<Mutex<Vec<String>>>,
        }

        impl crate::launcher::UrlLauncher for MockLauncher {
            fn launch(&self, url: &str) -> Result<(), crate::launcher::LaunchError> {
                self.launched.lock().unwrap().push(url.to_string());
                Ok(())
            }
        }

        let db_path = temp_db_path();
        let storage = Storage::open(&db_path).unwrap();
        let url = ValidatedUrl::parse("https://example.com").unwrap();
        let id = storage.urls().insert(&url, Some("Example")).unwrap();

        let launched = Arc::new(Mutex::new(Vec::new()));
        let launcher = MockLauncher {
            launched: launched.clone(),
        };

        let mut app = TablessApp::new(storage, Some(Box::new(launcher)), None);
        app.apply_actions(vec![ViewAction::Launch(id)]);

        let urls = launched.lock().unwrap();
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0], "https://example.com/");
    }

    #[test]
    fn launch_action_with_no_launcher_shows_error() {
        let db_path = temp_db_path();
        let storage = Storage::open(&db_path).unwrap();
        let url = ValidatedUrl::parse("https://example.com").unwrap();
        let id = storage.urls().insert(&url, Some("Example")).unwrap();

        let mut app = TablessApp::new(storage, None, None);
        app.apply_actions(vec![ViewAction::Launch(id)]);

        assert_eq!(app.error_message, Some("No browser configured".to_string()));
    }

    #[test]
    fn launch_action_with_failing_launcher_shows_error() {
        use crate::launcher::LaunchError;

        struct FailingLauncher;

        impl crate::launcher::UrlLauncher for FailingLauncher {
            fn launch(&self, _url: &str) -> Result<(), LaunchError> {
                Err(LaunchError::BrowserNotFound {
                    identity: crate::launcher::BrowserIdentity::Firefox,
                })
            }
        }

        let db_path = temp_db_path();
        let storage = Storage::open(&db_path).unwrap();
        let url = ValidatedUrl::parse("https://example.com").unwrap();
        let id = storage.urls().insert(&url, Some("Example")).unwrap();

        let mut app = TablessApp::new(storage, Some(Box::new(FailingLauncher)), None);
        app.apply_actions(vec![ViewAction::Launch(id)]);

        assert!(app.error_message.as_ref().unwrap().contains("Launch failed"));
    }

    #[test]
    fn launch_action_with_missing_id_is_graceful() {
        let db_path = temp_db_path();
        let storage = Storage::open(&db_path).unwrap();

        let mut app = TablessApp::new(storage, None, None);
        app.apply_actions(vec![ViewAction::Launch(9999)]);

        // Should not panic and should not set an error message
        assert!(app.error_message.is_none());
    }

    #[test]
    fn ipc_notification_refreshes_urls() {
        let db_path = temp_db_path();
        let storage = Storage::open(&db_path).unwrap();

        let (tx, rx) = std::sync::mpsc::channel();
        let mut app = TablessApp::new(storage, None, Some(rx));
        assert!(app.urls.is_empty());

        // Simulate another process inserting a URL
        let storage2 = Storage::open(&db_path).unwrap();
        let url = ValidatedUrl::parse("https://example.com").unwrap();
        let _ = storage2.urls().insert(&url, None).unwrap();

        // Simulate IPC notification
        tx.send(()).unwrap();

        // Trigger update (normally called by eframe; we call the logic directly)
        if app.drain_ipc() {
            app.refresh_urls();
        }

        assert_eq!(app.urls.len(), 1);
        assert_eq!(app.urls[0].canonical_url, "https://example.com/");
    }
}
