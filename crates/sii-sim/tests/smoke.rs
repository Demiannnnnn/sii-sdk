//! Smoke tests for the `sii-sim` crate.

#[test]
fn smoke_test_sim_version() {
    assert_eq!(sii_sim::VERSION, "0.1.0");
}

#[test]
fn smoke_test_sim_bin_runs() {
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_sii-sim"))
        .status()
        .expect("failed to execute sii-sim binary");
    assert!(status.success());
}
