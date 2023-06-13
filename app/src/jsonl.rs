use clap::Args;
use labelme_rs::serde_json;
use std::path::PathBuf;

#[cfg(not(target_os = "windows"))]
extern crate libc;

#[derive(Debug, Args)]
pub struct CmdArgs {
    /// Input json directory
    input: PathBuf,
    /// Key for filename
    #[clap(long, default_value = "filename", id = "key")]
    filename: String,
}

pub fn cmd_jsonl(args: CmdArgs) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(target_os = "windows"))]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    if args.input.is_file() {
        return Err(format!("Input {:?} is a file not a directory.", args.input).into());
    }
    if !args.input.exists() {
        return Err(format!("Input directory {:?} not found.", args.input).into());
    }
    let entries: Vec<_> = glob::glob(args.input.join("*.json").to_str().unwrap())
        .expect("Failed to read glob pattern")
        .collect();
    if entries.is_empty() {
        return Err("No json file found.".into());
    }
    for entry in entries {
        let input = entry?;
        let json_str = std::fs::read_to_string(&input).unwrap();
        let mut json_data: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&json_str).unwrap();
        let should_be_none = json_data.insert(
            args.filename.clone(),
            input.file_name().unwrap().to_string_lossy().into(),
        );
        if let Some(prev) = should_be_none {
            return Err(format!(
                "\"{}\" key already exists with value \"{}\"",
                args.filename, prev
            )
            .into());
        }
        let line = serde_json::to_string(&json_data)?;
        println!("{}", line);
    }
    Ok(())
}
