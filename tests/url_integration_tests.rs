use tabless::url::{UrlValidationError, ValidatedUrl};

#[test]
fn parse_valid_http_url() {
    let result = ValidatedUrl::parse("http://example.com");
    assert!(result.is_ok());
    let validated = result.unwrap();
    assert_eq!(validated.original(), "http://example.com");
    assert_eq!(validated.canonical(), "http://example.com/");
    assert_eq!(validated.scheme(), "http");
    assert_eq!(validated.host(), "example.com");
}

#[test]
fn parse_valid_https_url() {
    let result = ValidatedUrl::parse("https://example.com");
    assert!(result.is_ok());
}

#[test]
fn parse_rejects_empty_input() {
    let result = ValidatedUrl::parse("");
    assert!(matches!(result, Err(UrlValidationError::EmptyInput)));
}

#[test]
fn parse_rejects_invalid_scheme() {
    let result = ValidatedUrl::parse("javascript:alert(1)");
    assert!(matches!(
        result,
        Err(UrlValidationError::InvalidScheme { found })
        if found == "javascript"
    ));
}

#[test]
fn parse_rejects_file_scheme() {
    let result = ValidatedUrl::parse("file:///etc/passwd");
    assert!(matches!(
        result,
        Err(UrlValidationError::InvalidScheme { found })
        if found == "file"
    ));
}

#[test]
fn parse_rejects_malformed_url() {
    let result = ValidatedUrl::parse("not a url");
    assert!(matches!(
        result,
        Err(UrlValidationError::MalformedUrl { .. })
    ));
}

#[test]
fn parse_lowercases_hostname() {
    let validated = ValidatedUrl::parse("https://EXAMPLE.COM/path").unwrap();
    assert_eq!(validated.host(), "example.com");
    assert_eq!(validated.canonical(), "https://example.com/path");
}

#[test]
fn parse_preserves_original_url() {
    let original = "https://EXAMPLE.COM/path?query=1";
    let validated = ValidatedUrl::parse(original).unwrap();
    assert_eq!(validated.original(), original);
}
