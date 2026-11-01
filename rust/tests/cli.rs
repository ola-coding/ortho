use std::process::Command;

#[test]
fn says_its_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_ortho"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        env!("CARGO_PKG_VERSION")
    );
}
