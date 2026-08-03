use std::process::Command;

#[test]
fn malformed_yaml_is_reported_without_a_stack_trace() {
    let suite = tempfile::tempdir().unwrap();
    let config = suite.path().join("broken.config.yml");
    std::fs::write(&config, ": bad: yaml: here").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_tempest"))
        .arg("test")
        .arg("--path")
        .arg(suite.path())
        .env("RUST_BACKTRACE", "1")
        .env("RUST_LIB_BACKTRACE", "1")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(stderr.contains("error: invalid YAML"), "stderr: {stderr}");
    assert!(stderr.contains("broken.config.yml"), "stderr: {stderr}");
    assert!(!stderr.contains("Stack backtrace"), "stderr: {stderr}");
}
