use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[test]
fn test_jsonl_split() {
    let bin = env!("CARGO_BIN_EXE_lmrs");
    let tmp_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let json_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let output = Command::new(bin)
        .arg("jsonl")
        .arg(&json_dir)
        .output()
        .unwrap();
    assert_eq!(output.stderr.len(), 0);

    let mut proc_split = Command::new(bin)
        .arg("split")
        .arg("--output")
        .arg(&tmp_dir)
        .arg("--overwrite")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let split_stdin = proc_split.stdin.as_mut().unwrap();
    split_stdin.write_all(&output.stdout).unwrap();
    drop(split_stdin);

    let output = proc_split.wait_with_output().unwrap();
    assert_eq!(output.stdout.len(), 0, "Non-empty stdout");
    assert_eq!(output.stderr.len(), 0, "Non-empty stderror");

    for path in std::fs::read_dir(tmp_dir).unwrap() {
        println!("path:{path:?}");
        let path = path.unwrap().path();
        let ext = path.extension().unwrap().to_str().unwrap();
        assert_eq!(ext, "json");
        let orig_path = json_dir.join(path.file_name().unwrap());
        assert!(orig_path.exists());
        let orig_str = std::fs::read_to_string(orig_path)
            .unwrap()
            .replace([' ', '\n', '\r'], "");
        let new_str = std::fs::read_to_string(path)
            .unwrap()
            .replace([' ', '\n', '\r'], "");
        assert_eq!(orig_str, new_str);
    }
}
