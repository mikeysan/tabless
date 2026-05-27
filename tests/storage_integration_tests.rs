use tabless::storage::StorageError;

#[test]
fn error_variants_exist() {
    let _e = StorageError::ConnectionFailed {
        reason: "test".to_string(),
    };
}
