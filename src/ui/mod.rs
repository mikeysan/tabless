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
