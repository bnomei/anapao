use std::process::Command;

#[test]
fn main_binary_prints_library_first_message_to_stderr() {
    let output =
        Command::new(env!("CARGO_BIN_EXE_anapao")).output().expect("binary should execute");

    assert!(output.status.success());
    assert!(output.stdout.is_empty());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("anapao is library-first"));
}
