use eframe::App;

use crate::launcher::BrowserIdentity;
use crate::storage::{Storage, UrlRecord};
use crate::ui::ViewAction;
use crate::ui::main_list::{MainListState, main_list_view};
use crate::url::ValidatedUrl;

pub struct TablessApp {
    storage: Storage,
    main_list_state: MainListState,
    favorites: Vec<UrlRecord>,
    main_list: Vec<UrlRecord>,
    archived_urls: Vec<UrlRecord>,
    archive_view: bool,
    manual_entry: String,
    manual_entry_error: Option<String>,
    error_message: Option<String>,
    launcher: Option<Box<dyn crate::launcher::UrlLauncher>>,
    browser_identities: Vec<BrowserIdentity>,
    show_browser_picker: bool,
    browser_picker_id: Option<i64>,
    ipc_rx: Option<std::sync::mpsc::Receiver<()>>,
}

impl TablessApp {
    pub fn new(
        storage: Storage,
        launcher: Option<Box<dyn crate::launcher::UrlLauncher>>,
        browser_identities: Vec<BrowserIdentity>,
        ipc_rx: Option<std::sync::mpsc::Receiver<()>>,
    ) -> Self {
        let mut app = TablessApp {
            storage,
            main_list_state: MainListState::new(),
            favorites: Vec::new(),
            main_list: Vec::new(),
            archived_urls: Vec::new(),
            archive_view: false,
            manual_entry: String::new(),
            manual_entry_error: None,
            error_message: None,
            launcher,
            browser_identities,
            show_browser_picker: false,
            browser_picker_id: None,
            ipc_rx,
        };
        app.refresh_urls();
        app
    }

    pub fn refresh_urls(&mut self) {
        self.error_message = None;
        match self.storage.urls().list_favorites() {
            Ok(urls) => self.favorites = urls,
            Err(e) => {
                log::error!("Failed to load favorites: {}", e);
                if self.error_message.is_none() {
                    self.error_message = Some(format!("Failed to load favorites: {}", e));
                }
            }
        }
        match self.storage.urls().list_main() {
            Ok(urls) => self.main_list = urls,
            Err(e) => {
                log::error!("Failed to load main list: {}", e);
                if self.error_message.is_none() {
                    self.error_message = Some(format!("Failed to load main list: {}", e));
                }
            }
        }
        match self.storage.urls().list_archived() {
            Ok(urls) => self.archived_urls = urls,
            Err(e) => {
                log::error!("Failed to load archive: {}", e);
                if self.error_message.is_none() {
                    self.error_message = Some(format!("Failed to load archive: {}", e));
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
                ViewAction::Restore(id) => {
                    mutated = true;
                    self.storage.urls().set_archived(id, false)
                }
                ViewAction::Favorite(id) => {
                    mutated = true;
                    self.storage.urls().set_favorite(id, true)
                }
                ViewAction::Unfavorite(id) => {
                    mutated = true;
                    self.storage.urls().set_favorite(id, false)
                }
                ViewAction::Launch(id) => match self.storage.urls().find_by_id(id) {
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
                        log::warn!("URL not found for launch: id={}", id);
                        Ok(())
                    }
                    Err(e) => Err(e),
                },
                ViewAction::Copy(id) => match self.storage.urls().find_by_id(id) {
                    Ok(Some(record)) => {
                        if let Ok(mut clipboard) = arboard::Clipboard::new()
                            && let Err(e) = clipboard.set_text(&record.canonical_url)
                        {
                            self.error_message = Some(format!("Copy failed: {}", e));
                        }
                        Ok(())
                    }
                    Ok(None) => {
                        log::warn!("URL not found for copy: id={}", id);
                        Ok(())
                    }
                    Err(e) => Err(e),
                },
                ViewAction::MoveFavoriteUp(id) => {
                    mutated = true;
                    if let Some(pos) = self.favorites.iter().position(|r| r.id == id) {
                        if pos > 0 {
                            let prev_id = self.favorites[pos - 1].id;
                            self.storage.urls().swap_favorite_order(id, prev_id)
                        } else {
                            Ok(())
                        }
                    } else {
                        Ok(())
                    }
                }
                ViewAction::MoveFavoriteDown(id) => {
                    mutated = true;
                    if let Some(pos) = self.favorites.iter().position(|r| r.id == id) {
                        if pos + 1 < self.favorites.len() {
                            let next_id = self.favorites[pos + 1].id;
                            self.storage.urls().swap_favorite_order(id, next_id)
                        } else {
                            Ok(())
                        }
                    } else {
                        Ok(())
                    }
                }
                ViewAction::LaunchWithBrowser { id, identity } => {
                    match self.storage.urls().find_by_id(id) {
                        Ok(Some(record)) => {
                            if let Some(ref launcher) = self.launcher {
                                if let Err(e) =
                                    launcher.launch_with_identity(&record.canonical_url, identity)
                                {
                                    self.error_message = Some(format!("Launch failed: {}", e));
                                }
                            } else {
                                self.error_message = Some("No browser configured".to_string());
                            }
                            Ok(())
                        }
                        Ok(None) => {
                            log::warn!("URL not found for launch: id={}", id);
                            Ok(())
                        }
                        Err(e) => Err(e),
                    }
                }
            };
            if let Err(e) = result {
                log::error!("Action failed: {}", e);
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
        // Global paste handler: valid URLs anywhere in the focused app should add them
        let paste_texts: Vec<String> = ctx.input_mut(|i| {
            let mut texts = Vec::new();
            i.events.retain(|event| {
                if let egui::Event::Paste(text) = event {
                    texts.push(text.clone());
                    false
                } else {
                    true
                }
            });
            texts
        });
        let mut pasted_valid = false;
        for text in paste_texts {
            if let Ok(url) = ValidatedUrl::parse(&text) {
                if let Err(e) = self.storage.urls().insert(&url, None) {
                    log::error!("Paste insert failed: {}", e);
                } else {
                    pasted_valid = true;
                }
            }
        }
        if pasted_valid || self.drain_ipc() {
            self.refresh_urls();
        }

        let all_urls: Vec<UrlRecord> = if self.archive_view {
            self.archived_urls.clone()
        } else {
            self.favorites
                .iter()
                .cloned()
                .chain(self.main_list.iter().cloned())
                .collect()
        };

        let mut hovered_id = None;

        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.archive_view {
                ui.horizontal(|ui| {
                    ui.label("Add URL:");
                    let response = ui.text_edit_singleline(&mut self.manual_entry);
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) && response.has_focus() {
                        self.manual_entry_error = None;
                        if let Ok(url) = ValidatedUrl::parse(&self.manual_entry) {
                            if let Err(e) = self.storage.urls().insert(&url, None) {
                                self.manual_entry_error = Some(format!("Insert failed: {}", e));
                            } else {
                                self.manual_entry.clear();
                                self.refresh_urls();
                            }
                        } else {
                            self.manual_entry_error = Some("Invalid URL".to_string());
                        }
                    }
                });
                if let Some(ref err) = self.manual_entry_error {
                    ui.colored_label(egui::Color32::RED, err);
                }
            }

            if let Some(ref msg) = self.error_message {
                ui.colored_label(egui::Color32::RED, msg);
            }

            let (view_actions, maybe_hovered) =
                main_list_view(ui, &mut self.main_list_state, &all_urls, self.archive_view);
            hovered_id = maybe_hovered;
            if !view_actions.is_empty() {
                self.apply_actions(view_actions);
            }
        });

        let filtered = self.main_list_state.filtered_items(&all_urls);

        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            self.main_list_state.navigate_up(filtered.len());
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            self.main_list_state.navigate_down(filtered.len());
        }

        let mut actions = Vec::new();
        let target_record = hovered_id
            .and_then(|id| filtered.iter().find(|r| r.id == id).copied())
            .or_else(|| filtered.get(self.main_list_state.selected_index).copied());

        if !ctx.wants_keyboard_input() {
            if let Some(record) = target_record {
                if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                    actions.push(ViewAction::Launch(record.id));
                }
                if ctx.input(|i| i.key_pressed(egui::Key::L)) {
                    actions.push(ViewAction::Launch(record.id));
                }
                if ctx.input(|i| i.key_pressed(egui::Key::C)) {
                    actions.push(ViewAction::Copy(record.id));
                }
                if self.archive_view {
                    if ctx.input(|i| i.key_pressed(egui::Key::R)) {
                        actions.push(ViewAction::Restore(record.id));
                    }
                } else {
                    if ctx.input(|i| i.key_pressed(egui::Key::A)) {
                        actions.push(ViewAction::Archive(record.id));
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::F)) {
                        if record.favorite {
                            actions.push(ViewAction::Unfavorite(record.id));
                        } else {
                            actions.push(ViewAction::Favorite(record.id));
                        }
                    }
                    if record.favorite {
                        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp) && i.modifiers.shift) {
                            actions.push(ViewAction::MoveFavoriteUp(record.id));
                        }
                        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown) && i.modifiers.shift) {
                            actions.push(ViewAction::MoveFavoriteDown(record.id));
                        }
                    }
                }
                if ctx.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.shift) {
                    self.show_browser_picker = true;
                    self.browser_picker_id = Some(record.id);
                }
            }
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Tab)) {
            self.archive_view = !self.archive_view;
            self.main_list_state.selected_index = 0;
            self.main_list_state.search_query.clear();
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.main_list_state.search_query.clear();
            self.main_list_state.selected_index = 0;
            self.main_list_state.search_focused = false;
            self.show_browser_picker = false;
            self.browser_picker_id = None;
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Slash)) {
            self.main_list_state.search_focused = true;
        }

        if !actions.is_empty() {
            self.apply_actions(actions);
        }

        // Browser picker modal
        let mut chosen_identity: Option<BrowserIdentity> = None;
        let mut close_picker = false;
        if self.show_browser_picker {
            egui::Window::new("Open in Alternate Browser")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    if self.browser_identities.is_empty() {
                        ui.label("No alternate browsers discovered.");
                    } else {
                        for identity in &self.browser_identities {
                            let label = format!("{:?}", identity);
                            if ui.button(&label).clicked() {
                                chosen_identity = Some(identity.clone());
                                close_picker = true;
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        close_picker = true;
                    }
                });
        }
        if close_picker {
            if let (Some(id), Some(identity)) = (self.browser_picker_id, chosen_identity) {
                self.apply_actions(vec![ViewAction::LaunchWithBrowser { id, identity }]);
            }
            self.show_browser_picker = false;
            self.browser_picker_id = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::TablessApp;
    use crate::launcher::BrowserIdentity;
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

        let mut app = TablessApp::new(storage, None, Vec::new(), None);
        app.apply_actions(vec![ViewAction::Archive(id)]);

        let record = app.storage.urls().find_by_id(id).unwrap().unwrap();
        assert!(record.archived);
    }

    #[test]
    fn favorite_action_updates_storage() {
        let db_path = temp_db_path();
        let storage = Storage::open(&db_path).unwrap();
        let url = ValidatedUrl::parse("https://example.com").unwrap();
        let id = storage.urls().insert(&url, Some("Example Site")).unwrap();

        let mut app = TablessApp::new(storage, None, Vec::new(), None);
        app.apply_actions(vec![ViewAction::Favorite(id)]);

        let record = app.storage.urls().find_by_id(id).unwrap().unwrap();
        assert!(record.favorite);
    }

    #[test]
    fn restore_action_updates_storage() {
        let db_path = temp_db_path();
        let storage = Storage::open(&db_path).unwrap();
        let url = ValidatedUrl::parse("https://example.com").unwrap();
        let id = storage.urls().insert(&url, Some("Example Site")).unwrap();
        storage.urls().set_archived(id, true).unwrap();

        let mut app = TablessApp::new(storage, None, Vec::new(), None);
        app.apply_actions(vec![ViewAction::Restore(id)]);

        let record = app.storage.urls().find_by_id(id).unwrap().unwrap();
        assert!(!record.archived);
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
            fn launch_with_identity(
                &self,
                _url: &str,
                _identity: BrowserIdentity,
            ) -> Result<(), crate::launcher::LaunchError> {
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

        let mut app = TablessApp::new(storage, Some(Box::new(launcher)), Vec::new(), None);
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

        let mut app = TablessApp::new(storage, None, Vec::new(), None);
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
                    identity: BrowserIdentity::Firefox,
                })
            }
            fn launch_with_identity(
                &self,
                _url: &str,
                _identity: BrowserIdentity,
            ) -> Result<(), LaunchError> {
                Err(LaunchError::BrowserNotFound {
                    identity: BrowserIdentity::Firefox,
                })
            }
        }

        let db_path = temp_db_path();
        let storage = Storage::open(&db_path).unwrap();
        let url = ValidatedUrl::parse("https://example.com").unwrap();
        let id = storage.urls().insert(&url, Some("Example")).unwrap();

        let mut app = TablessApp::new(storage, Some(Box::new(FailingLauncher)), Vec::new(), None);
        app.apply_actions(vec![ViewAction::Launch(id)]);

        assert!(
            app.error_message
                .as_ref()
                .unwrap()
                .contains("Launch failed")
        );
    }

    #[test]
    fn launch_action_with_missing_id_is_graceful() {
        let db_path = temp_db_path();
        let storage = Storage::open(&db_path).unwrap();

        let mut app = TablessApp::new(storage, None, Vec::new(), None);
        app.apply_actions(vec![ViewAction::Launch(9999)]);

        // Should not panic and should not set an error message
        assert!(app.error_message.is_none());
    }

    #[test]
    fn ipc_notification_refreshes_urls() {
        let db_path = temp_db_path();
        let storage = Storage::open(&db_path).unwrap();

        let (tx, rx) = std::sync::mpsc::channel();
        let mut app = TablessApp::new(storage, None, Vec::new(), Some(rx));
        assert!(app.main_list.is_empty());

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

        assert_eq!(app.main_list.len(), 1);
        assert_eq!(app.main_list[0].canonical_url, "https://example.com/");
    }
}
