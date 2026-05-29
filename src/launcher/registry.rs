use std::collections::HashMap;

use super::error::DiscoveryError;
use super::identity::BrowserIdentity;
use super::info::BrowserInfo;

pub struct BrowserRegistry {
    known: HashMap<BrowserIdentity, BrowserInfo>,
    preferred: Option<BrowserIdentity>,
}

impl BrowserRegistry {
    pub fn new(discovered: Vec<BrowserInfo>) -> Self {
        let preferred = discovered
            .iter()
            .find(|info| info.is_default)
            .or_else(|| discovered.first())
            .map(|info| info.identity.clone());

        let known = discovered
            .into_iter()
            .map(|info| (info.identity.clone(), info))
            .collect();

        BrowserRegistry { known, preferred }
    }

    pub fn set_preferred(&mut self, identity: BrowserIdentity) -> Result<(), DiscoveryError> {
        if !self.known.contains_key(&identity) {
            return Err(DiscoveryError::PathNotFound {
                path: std::path::PathBuf::from(format!("{:?}", identity)),
            });
        }
        self.preferred = Some(identity);
        Ok(())
    }

    pub fn preferred_browser(&self) -> Option<&BrowserInfo> {
        self.preferred.as_ref().and_then(|id| self.known.get(id))
    }

    pub fn all_browsers(&self) -> Vec<&BrowserInfo> {
        self.known.values().collect()
    }

    pub fn find(&self, identity: &BrowserIdentity) -> Option<&BrowserInfo> {
        self.known.get(identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_info(identity: BrowserIdentity, path: &str) -> BrowserInfo {
        BrowserInfo {
            identity,
            executable_path: PathBuf::from(path),
            version: None,
            is_default: false,
        }
    }

    #[test]
    fn registry_stores_discovered_browsers() {
        let discovered = vec![
            make_info(BrowserIdentity::Firefox, "/usr/bin/firefox"),
            make_info(BrowserIdentity::Chrome, "/usr/bin/google-chrome"),
        ];
        let registry = BrowserRegistry::new(discovered);
        assert_eq!(registry.all_browsers().len(), 2);
    }

    #[test]
    fn set_preferred_succeeds_for_known_browser() {
        let discovered = vec![make_info(BrowserIdentity::Firefox, "/usr/bin/firefox")];
        let mut registry = BrowserRegistry::new(discovered);
        let result = registry.set_preferred(BrowserIdentity::Firefox);
        assert!(result.is_ok());
        assert_eq!(
            registry.preferred_browser().unwrap().identity,
            BrowserIdentity::Firefox
        );
    }

    #[test]
    fn set_preferred_fails_for_unknown_browser() {
        let discovered = vec![make_info(BrowserIdentity::Firefox, "/usr/bin/firefox")];
        let mut registry = BrowserRegistry::new(discovered);
        let result = registry.set_preferred(BrowserIdentity::Chrome);
        assert!(result.is_err());
    }

    #[test]
    fn find_returns_some_for_known() {
        let discovered = vec![make_info(BrowserIdentity::Firefox, "/usr/bin/firefox")];
        let registry = BrowserRegistry::new(discovered);
        assert!(registry.find(&BrowserIdentity::Firefox).is_some());
        assert!(registry.find(&BrowserIdentity::Chrome).is_none());
    }

    #[test]
    fn preferred_browser_none_when_unset() {
        let discovered = vec![make_info(BrowserIdentity::Firefox, "/usr/bin/firefox")];
        let registry = BrowserRegistry::new(discovered);
        assert!(registry.preferred_browser().is_none());
    }
}
