use anyhow::{Context, Ok, Result};
use labelme_rs::serde_json;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

use lmrs::cli::DropCmdArgs as CmdArgs;

fn drop(json_lines: impl BufRead, key: &str, mut out: impl Write) -> Result<()> {
    let mut existing_set: HashSet<String> = HashSet::new();
    for line in json_lines.lines() {
        let line = line?;
        let json_data: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&line)?;
        let value = json_data
            .get(key)
            .with_context(|| format!("Key '{}' not found", key))?;
        if let serde_json::Value::String(value) = value {
            if existing_set.insert(value.clone()) {
                // HashSet::insert returns true when the given value is new
                writeln!(out, "{}", line)?;
            }
        } else {
            panic!("Value for {} should be string. {} found", key, value);
        };
    }
    Ok(())
}

pub fn cmd(args: CmdArgs) -> Result<()> {
    let reader: Box<dyn BufRead> = if args.input.as_os_str() == "-" {
        Box::new(BufReader::new(std::io::stdin()))
    } else {
        Box::new(BufReader::new(File::open(&args.input)?))
    };
    drop(reader, &args.key, std::io::stdout())?;
    Ok(())
}

#[test]
fn test_drop() -> anyhow::Result<()> {
    use std::io::Cursor;
    let ndjson = r#"{"l":"1","k":"v"}
    {"l":"2","k":"v"}"#;
    let mut buf = Vec::new();
    let cur = Cursor::new(&mut buf);
    drop(BufReader::new(Cursor::new(ndjson)), "k", cur)?;
    let dropped = String::from_utf8(buf)?;
    let expected = r#"{"l":"1","k":"v"}
"#;
    assert_eq!(dropped, expected);
    Ok(())
}
