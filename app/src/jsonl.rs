use anyhow::{bail, ensure, Context, Result};
use labelme_rs::serde_json;
use lmrs::cli::JsonlCmdArgs as CmdArgs;
use std::io::{BufRead, BufReader};

#[cfg(not(target_os = "windows"))]
extern crate libc;

fn print_jsonl(input: std::path::PathBuf, key: &str) -> Result<()> {
    let json_str = std::fs::read_to_string(&input)?;
    let mut json_data: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&json_str)?;
    let should_be_none = json_data.insert(
        key.to_string(),
        input
            .file_name()
            .context("filename is missing")?
            .to_string_lossy()
            .into(),
    );
    if let Some(prev) = should_be_none {
        bail!("\"{}\" key already exists with value \"{}\"", key, prev);
    }
    let line = serde_json::to_string(&json_data)?;
    println!("{line}");
    Ok(())
}

pub fn cmd(args: CmdArgs) -> Result<()> {
    #[cfg(not(target_os = "windows"))]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
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
                print_jsonl(input, &args.filename)?;
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
            print_jsonl(input, &args.filename)?;
        } else {
            bail!("{:?} is not a directory, json, or ndjson/jsonl", input);
        }
    }
    Ok(())
}
