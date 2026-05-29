pub mod app;
pub mod main_list;
pub mod url_row;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewAction {
    Archive(i64),
    Restore(i64),
    Pin(i64),
    Unpin(i64),
    Launch(i64),
}
