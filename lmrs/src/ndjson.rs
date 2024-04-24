use anyhow::{bail, ensure, Context, Result};
use labelme_rs::serde_json;
use lmrs::cli::{NdjsonCmdArgs as CmdArgs, ParentHandling};
use serde_json::{Map, Value};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[cfg(not(target_os = "windows"))]
extern crate libc;

fn print_ndjson(input: PathBuf, key: &str, parent_handling: ParentHandling) -> Result<()> {
    let json_str = std::fs::read_to_string(&input)?;
    let content: Map<String, Value> = serde_json::from_str(&json_str)?;
    let mut json_data: Map<String, Value> = Map::default();
    json_data.insert("content".into(), content.into());
    let filename: String = match parent_handling {
        ParentHandling::Keep => input.to_string_lossy().into(),
        ParentHandling::Absolute => input.canonicalize()?.to_string_lossy().into(),
        ParentHandling::Remove => input
            .file_name()
            .with_context(|| format!("Filename is missing in {:?}", input))?
            .to_string_lossy()
            .into(),
    };
    json_data.insert(key.to_string(), filename.into());
    let line = serde_json::to_string(&json_data)?;
    println!("{line}");
    Ok(())
}

pub fn cmd(args: CmdArgs) -> Result<()> {
    for input in args.input {
        ensure!(input.exists(), "Input {:?} does not exist", input);
        if input.is_dir() {
            let entries = glob::glob(
                input
                    .join("*.json")
                    .to_str()
                    .context("Failed to obtain glob string")?,
            )
            .expect("Failed to read glob pattern");
            for entry in entries {
                let input = entry?;
                print_ndjson(input, &args.filename, args.parent)?;
            }
        } else if input
            .extension()
            .map(|ext| ext == "ndjson" || ext == "jsonl")
            .unwrap_or(false)
        {
            let file = BufReader::new(std::fs::File::open(&input)?);
            for line in file.lines() {
                println!("{}", line?);
            }
        } else if input.extension().map(|ext| ext == "json").unwrap_or(false) {
            print_ndjson(input, &args.filename, args.parent)?;
        } else {
            bail!("{:?} is not a directory, json, or ndjson/jsonl", input);
        }
    }
    Ok(())
}
