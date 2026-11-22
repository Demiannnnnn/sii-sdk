//! Smoke tests for the `sii-cli` crate.

#[test]
fn smoke_test_cli_runs() {
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_sii"))
        .status()
        .expect("failed to execute sii binary");
    assert!(status.success());
}
