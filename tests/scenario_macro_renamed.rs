use std::path::PathBuf;
use std::process::Command;

#[test]
fn scenario_macro_supports_a_real_renamed_dependency() {
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/scenario-macro-renamed");
    let output = Command::new(env!("CARGO"))
        .args(["check", "--offline", "--manifest-path"])
        .arg(fixture.join("Cargo.toml"))
        .args(["--target-dir"])
        .arg(fixture.join("target"))
        .output()
        .expect("run cargo check for renamed-dependency fixture");

    assert!(
        output.status.success(),
        "renamed-dependency fixture failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
