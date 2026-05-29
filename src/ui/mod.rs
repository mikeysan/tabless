use crate::launcher::BrowserIdentity;

pub mod app;
pub mod main_list;
pub mod url_row;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewAction {
    Archive(i64),
    Restore(i64),
    Favorite(i64),
    Unfavorite(i64),
    Launch(i64),
    Copy(i64),
    MoveFavoriteUp(i64),
    MoveFavoriteDown(i64),
    LaunchWithBrowser { id: i64, identity: BrowserIdentity },
}
