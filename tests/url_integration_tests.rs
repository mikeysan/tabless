use tabless::url::UrlValidationError;

#[test]
fn error_variants_exist() {
    let _e = UrlValidationError::EmptyInput;
}
