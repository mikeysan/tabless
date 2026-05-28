use super::error::ProtocolError;

pub fn parse_protocol_url(input: &str) -> Result<String, ProtocolError> {
    let parsed = url::Url::parse(input).map_err(|e| ProtocolError::InvalidUrl {
        reason: format!("failed to parse protocol URL: {}", e),
    })?;

    if parsed.scheme() != "tabless" {
        return Err(ProtocolError::InvalidUrl {
            reason: format!("expected scheme 'tabless', found '{}'", parsed.scheme()),
        });
    }

    if parsed.host_str() != Some("open") {
        return Err(ProtocolError::InvalidUrl {
            reason: format!("expected host 'open', found '{:?}'", parsed.host_str()),
        });
    }

    let embedded = parsed
        .query_pairs()
        .find(|(k, _)| k == "url")
        .map(|(_, v)| v.into_owned());

    match embedded {
        Some(url) if !url.is_empty() => Ok(url),
        _ => Err(ProtocolError::InvalidUrl {
            reason: "missing or empty 'url' query parameter".to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_tabless_url() {
        let result = parse_protocol_url("tabless://open?url=https://example.com").unwrap();
        assert_eq!(result, "https://example.com");
    }

    #[test]
    fn parse_url_with_encoded_value() {
        let result =
            parse_protocol_url("tabless://open?url=https%3A%2F%2Fexample.com%2Fpath").unwrap();
        assert_eq!(result, "https://example.com/path");
    }

    #[test]
    fn parse_rejects_wrong_scheme() {
        let result = parse_protocol_url("https://open?url=https://example.com");
        assert!(matches!(result, Err(ProtocolError::InvalidUrl { .. })));
    }

    #[test]
    fn parse_rejects_wrong_path() {
        let result = parse_protocol_url("tabless://other?url=https://example.com");
        assert!(matches!(result, Err(ProtocolError::InvalidUrl { .. })));
    }

    #[test]
    fn parse_rejects_missing_url_param() {
        let result = parse_protocol_url("tabless://open?other=thing");
        assert!(matches!(result, Err(ProtocolError::InvalidUrl { .. })));
    }

    #[test]
    fn parse_rejects_empty_url_param() {
        let result = parse_protocol_url("tabless://open?url=");
        assert!(matches!(result, Err(ProtocolError::InvalidUrl { .. })));
    }
}
