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

        let parsed = Url::parse(input).map_err(|e| UrlValidationError::MalformedUrl {
            reason: e.to_string(),
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
