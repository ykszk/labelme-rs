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

    let mut proc_filter = Command::new(bin)
        .arg("split")
        .arg("--output")
        .arg(&tmp_dir)
        .arg("--overwrite")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let filter_stdin = proc_filter.stdin.as_mut().unwrap();
    filter_stdin.write_all(&output.stdout).unwrap();
    // drop(filter_stdin);

    let output = proc_filter.wait_with_output().unwrap();
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

#[test]
fn test_filter() {
    let bin = env!("CARGO_BIN_EXE_lmrs");
    let json_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let rule_file = json_dir.join("rules.txt");
    let jsonl_output = Command::new(bin)
        .arg("jsonl")
        .arg(&json_dir)
        .output()
        .unwrap();
    assert_eq!(jsonl_output.stderr.len(), 0);

    // test filtering
    let mut proc_filter = Command::new(bin)
        .arg("filter")
        .arg("-")
        .arg("-r")
        .arg(&rule_file)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let filter_stdin = proc_filter.stdin.as_mut().unwrap();
    filter_stdin.write_all(&jsonl_output.stdout).unwrap();

    let filter_output = proc_filter.wait_with_output().unwrap();
    assert_eq!(filter_output.stderr.len(), 0, "Non-empty stderror");
    assert_ne!(filter_output.stdout.len(), 0, "Empty stdout");
    let filter_stdout = std::str::from_utf8(&filter_output.stdout).unwrap();
    assert!(filter_stdout.contains("test.json"), "Filtering error");

    // test inverted filtering
    let mut proc_filter = Command::new(bin)
        .arg("filter")
        .arg("-")
        .arg("--invert")
        .arg("-r")
        .arg(&rule_file)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let filter_stdin = proc_filter.stdin.as_mut().unwrap();
    filter_stdin.write_all(&jsonl_output.stdout).unwrap();

    let filter_output = proc_filter.wait_with_output().unwrap();
    assert_eq!(filter_output.stderr.len(), 0, "Non-empty stderror");
    assert_ne!(filter_output.stdout.len(), 0, "Empty stdout");
    let filter_stdout = std::str::from_utf8(&filter_output.stdout).unwrap();
    assert!(!filter_stdout.contains("test.json"), "Filtering error");
}
