use url::Url;

pub fn normalize(url: &Url) -> Url {
    let mut normalized = url.clone();
    if let Some(host) = normalized.host_str() {
        let lower = host.to_lowercase();
        if host != lower {
            normalized
                .set_host(Some(&lower))
                .expect("lowercasing an existing valid host should never fail");
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_already_lowercase_noop() {
        let url = Url::parse("https://example.com").unwrap();
        let normalized = normalize(&url);
        assert_eq!(normalized.host_str(), Some("example.com"));
    }

    #[test]
    fn normalize_uppercase_host() {
        let url = Url::parse("https://EXAMPLE.COM").unwrap();
        let normalized = normalize(&url);
        assert_eq!(normalized.host_str(), Some("example.com"));
    }

    #[test]
    fn normalize_mixed_case_host() {
        let url = Url::parse("https://ExAmPlE.CoM").unwrap();
        let normalized = normalize(&url);
        assert_eq!(normalized.host_str(), Some("example.com"));
    }

    #[test]
    fn normalize_preserves_path_and_query() {
        let url = Url::parse("https://EXAMPLE.COM/PATH?Q=1").unwrap();
        let normalized = normalize(&url);
        assert_eq!(normalized.as_str(), "https://example.com/PATH?Q=1");
    }
}
