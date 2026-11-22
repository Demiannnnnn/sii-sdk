//! Smoke tests for the `sii` crate.

#[test]
fn smoke_test_version() {
    assert_eq!(sii::VERSION, "0.1.0");
}
