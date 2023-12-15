use anyhow::Result;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[test]
fn test_jsonl_split() -> Result<()> {
    let bin = env!("CARGO_BIN_EXE_lmrs");
    let tmp_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let json_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let output = Command::new(bin).arg("jsonl").arg(&json_dir).output()?;
    assert_eq!(output.stderr.len(), 0);

    let mut proc = Command::new(bin)
        .arg("split")
        .arg("--output")
        .arg(&tmp_dir)
        .arg("--overwrite")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let filter_stdin = proc.stdin.as_mut().unwrap();
    filter_stdin.write_all(&output.stdout)?;
    // drop(filter_stdin);

    let output = proc.wait_with_output()?;
    assert_eq!(output.stdout.len(), 0, "Non-empty stdout");
    assert_eq!(
        output.stderr.len(),
        0,
        "Non-empty stderror: {}",
        String::from_utf8_lossy(output.stderr.as_slice())
    );
    for entry in glob::glob(tmp_dir.join("*.json").to_str().unwrap())? {
        let path = entry?;
        println!("path:{path:?}");
        let ext = path.extension().unwrap().to_str().unwrap();
        assert_eq!(ext, "json");
        let orig_path = json_dir.join(path.file_name().unwrap());
        assert!(orig_path.exists());
        let orig_str = std::fs::read_to_string(orig_path)?.replace([' ', '\n', '\r'], "");
        let new_str = std::fs::read_to_string(path)?.replace([' ', '\n', '\r'], "");
        assert_eq!(orig_str, new_str);
    }
    Ok(())
}

#[test]
fn test_filter() -> Result<()> {
    let bin = env!("CARGO_BIN_EXE_lmrs");
    let json_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let rule_file = json_dir.join("rules.txt");
    let jsonl_output = Command::new(bin).arg("jsonl").arg(&json_dir).output()?;
    assert_eq!(jsonl_output.stderr.len(), 0);

    // test filtering
    let mut proc = Command::new(bin)
        .arg("filter")
        .arg("-")
        .arg("-r")
        .arg(&rule_file)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let filter_stdin = proc.stdin.as_mut().unwrap();
    filter_stdin.write_all(&jsonl_output.stdout)?;

    let filter_output = proc.wait_with_output()?;
    assert_eq!(filter_output.stderr.len(), 0, "Non-empty stderror");
    assert_ne!(filter_output.stdout.len(), 0, "Empty stdout");
    let filter_stdout = std::str::from_utf8(&filter_output.stdout)?;
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
        .spawn()?;

    let filter_stdin = proc_filter.stdin.as_mut().unwrap();
    filter_stdin.write_all(&jsonl_output.stdout)?;

    let filter_output = proc_filter.wait_with_output()?;
    assert_eq!(filter_output.stderr.len(), 0, "Non-empty stderror");
    assert_ne!(filter_output.stdout.len(), 0, "Empty stdout");
    let filter_stdout = std::str::from_utf8(&filter_output.stdout)?;
    assert!(!filter_stdout.contains("test.json"), "Filtering error");
    Ok(())
}
