//! Smoke tests for the `sii-cli` crate.

#[test]
fn smoke_test_cli_runs() -> Result<(), Box<dyn std::error::Error>> {
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_sii")).status()?;
    assert!(status.success());
    Ok(())
}
