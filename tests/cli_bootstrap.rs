use std::process::Command;

#[test]
fn binary_help_lists_create_and_update_commands() {
    let output = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .arg("--help")
        .output()
        .unwrap();

    assert!(output.status.success());
    let standard_output = String::from_utf8(output.stdout).unwrap();
    assert!(standard_output.contains("create"));
    assert!(standard_output.contains("update"));
}
