use tabless::url::UrlValidationError;
use tabless::url::normalizer::normalize;

#[test]
fn error_variants_exist() {
    let _e = UrlValidationError::EmptyInput;
}

#[test]
fn normalizer_lowercases_hostname() {
    let input = url::Url::parse("https://EXAMPLE.COM/path").unwrap();
    let normalized = normalize(&input);
    assert_eq!(normalized.host_str(), Some("example.com"));
}
