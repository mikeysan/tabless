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
