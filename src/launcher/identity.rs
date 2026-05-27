#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BrowserIdentity {
    Brave,
    Firefox,
    Zen,
    LibreWolf,
    Chrome,
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equality_for_known_browsers() {
        assert_eq!(BrowserIdentity::Brave, BrowserIdentity::Brave);
        assert_ne!(BrowserIdentity::Brave, BrowserIdentity::Firefox);
    }

    #[test]
    fn custom_equality() {
        assert_eq!(
            BrowserIdentity::Custom("vivaldi".to_string()),
            BrowserIdentity::Custom("vivaldi".to_string())
        );
        assert_ne!(
            BrowserIdentity::Custom("vivaldi".to_string()),
            BrowserIdentity::Custom("opera".to_string())
        );
    }

    #[test]
    fn custom_does_not_equal_known() {
        assert_ne!(
            BrowserIdentity::Custom("chrome".to_string()),
            BrowserIdentity::Chrome
        );
    }
}
