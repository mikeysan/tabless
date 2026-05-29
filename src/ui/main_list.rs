use crate::storage::UrlRecord;
use crate::ui::ViewAction;
use crate::ui::url_row::url_row;

pub struct MainListState {
    pub selected_index: usize,
    pub search_query: String,
    pub search_focused: bool,
}

impl Default for MainListState {
    fn default() -> Self {
        Self::new()
    }
}

impl MainListState {
    pub fn new() -> Self {
        MainListState {
            selected_index: 0,
            search_query: String::new(),
            search_focused: false,
        }
    }

    pub fn navigate_up(&mut self, item_count: usize) {
        if item_count == 0 {
            self.selected_index = 0;
        } else {
            self.selected_index = (self.selected_index + item_count - 1) % item_count;
        }
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

pub fn main_list_view(
    ui: &mut egui::Ui,
    state: &mut MainListState,
    items: &[UrlRecord],
) -> Vec<ViewAction> {
    let mut actions = Vec::new();

    // Search bar
    ui.horizontal(|ui| {
        ui.label("Search:");
        let response = ui.text_edit_singleline(&mut state.search_query);
        if state.search_focused {
            response.request_focus();
            state.search_focused = false;
        }
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
            if items.is_empty() {
                ui.label("No URLs captured yet. Open a link to get started.");
            } else {
                ui.label("No URLs match your search.");
            }
        });
        return actions;
    }

    // Clamp selected_index to filtered list bounds
    if state.selected_index >= filtered.len() {
        state.selected_index = filtered.len().saturating_sub(1);
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        let mut favorites_heading_shown = false;
        let mut main_heading_shown = false;
        for (idx, record) in filtered.iter().enumerate() {
            if record.pinned && !favorites_heading_shown {
                ui.heading("Favorites");
                ui.separator();
                favorites_heading_shown = true;
            }
            if !record.pinned && !main_heading_shown {
                ui.heading("Saved URLs");
                ui.separator();
                main_heading_shown = true;
            }

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
    use super::MainListState;
    use crate::storage::UrlRecord;

    #[test]
    fn navigate_down_increments_index() {
        let mut state = MainListState::new();
        state.navigate_down(5);
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn navigate_up_decrements_index() {
        let mut state = MainListState::new();
        state.selected_index = 2;
        state.navigate_up(5);
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn navigate_down_wraps_to_zero() {
        let mut state = MainListState::new();
        state.selected_index = 4;
        state.navigate_down(5);
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn navigate_up_wraps_to_end() {
        let mut state = MainListState::new();
        state.navigate_up(5);
        assert_eq!(state.selected_index, 4);
    }

    #[test]
    fn navigate_down_with_zero_items() {
        let mut state = MainListState::new();
        state.navigate_down(0);
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
        let mut state = MainListState::new();
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
        let mut state = MainListState::new();
        state.search_query = "rust".to_string();
        let filtered = state.filtered_items(&items);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, 2);
    }

    #[test]
    fn empty_search_returns_all() {
        let items = vec![UrlRecord {
            id: 1,
            canonical_url: "https://example.com".to_string(),
            original_url: "https://example.com".to_string(),
            title: None,
            favicon_path: None,
            created_at: 0,
            updated_at: 0,
            archived: false,
            pinned: false,
        }];
        let state = MainListState::new();
        let filtered = state.filtered_items(&items);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn search_case_insensitive() {
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
        let mut state = MainListState::new();
        state.search_query = "RUST".to_string();
        let filtered = state.filtered_items(&items);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, 2);
    }

    #[test]
    fn search_matches_both_title_and_url() {
        let items = vec![
            UrlRecord {
                id: 1,
                canonical_url: "https://rust-lang.org".to_string(),
                original_url: "https://rust-lang.org".to_string(),
                title: Some("Rust Lang".to_string()),
                favicon_path: None,
                created_at: 0,
                updated_at: 0,
                archived: false,
                pinned: false,
            },
            UrlRecord {
                id: 2,
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
                id: 3,
                canonical_url: "https://rusty-nail.com".to_string(),
                original_url: "https://rusty-nail.com".to_string(),
                title: Some("Another Site".to_string()),
                favicon_path: None,
                created_at: 0,
                updated_at: 0,
                archived: false,
                pinned: false,
            },
        ];
        let mut state = MainListState::new();
        state.search_query = "rust".to_string();
        let filtered = state.filtered_items(&items);
        assert_eq!(filtered.len(), 2);
        let ids: Vec<i64> = filtered.iter().map(|r| r.id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&3));
    }
}
