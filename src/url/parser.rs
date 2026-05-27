use url::Url;

use super::error::UrlValidationError;
use super::normalizer::normalize;

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedUrl {
    original: String,
    canonical: Url,
}

impl ValidatedUrl {
    pub fn parse(input: &str) -> Result<Self, UrlValidationError> {
        if input.is_empty() {
            return Err(UrlValidationError::EmptyInput);
        }

        let parsed = Url::parse(input).map_err(|e| match e {
            url::ParseError::EmptyHost => UrlValidationError::EmptyHost,
            _ => UrlValidationError::MalformedUrl {
                reason: e.to_string(),
            },
        })?;

        let scheme = parsed.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(UrlValidationError::InvalidScheme {
                found: scheme.to_string(),
            });
        }

        if parsed.host_str().is_none() {
            return Err(UrlValidationError::EmptyHost);
        }

        let canonical = normalize(&parsed);

        Ok(ValidatedUrl {
            original: input.to_string(),
            canonical,
        })
    }

    pub fn original(&self) -> &str {
        &self.original
    }

    pub fn canonical(&self) -> &str {
        self.canonical.as_str()
    }

    pub fn scheme(&self) -> &str {
        self.canonical.scheme()
    }

    pub fn host(&self) -> &str {
        self.canonical.host_str().unwrap_or("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::url::UrlValidationError;

    #[test]
    fn parse_https_with_path_and_query() {
        let v = ValidatedUrl::parse("https://example.com/path?foo=bar").unwrap();
        assert_eq!(v.canonical(), "https://example.com/path?foo=bar");
    }

    #[test]
    fn parse_rejects_data_scheme() {
        let result = ValidatedUrl::parse("data:text/html,hello");
        assert!(matches!(
            result,
            Err(UrlValidationError::InvalidScheme { found })
            if found == "data"
        ));
    }

    #[test]
    fn parse_rejects_about_scheme() {
        let result = ValidatedUrl::parse("about:blank");
        assert!(matches!(
            result,
            Err(UrlValidationError::InvalidScheme { found })
            if found == "about"
        ));
    }

    #[test]
    fn parse_accepts_punycode_hostname() {
        let v = ValidatedUrl::parse("https://xn--bcher-kva.com").unwrap();
        assert_eq!(v.host(), "xn--bcher-kva.com");
    }

    #[test]
    fn parse_preserves_percent_encoding() {
        let v = ValidatedUrl::parse("https://example.com/hello%20world").unwrap();
        assert_eq!(v.canonical(), "https://example.com/hello%20world");
    }

    #[test]
    fn parse_rejects_missing_host() {
        let result = ValidatedUrl::parse("http:///");
        assert!(matches!(result, Err(UrlValidationError::EmptyHost)));
    }

    #[test]
    fn parse_accepts_custom_port() {
        let v = ValidatedUrl::parse("https://example.com:8080").unwrap();
        assert_eq!(v.canonical(), "https://example.com:8080/");
    }
}
