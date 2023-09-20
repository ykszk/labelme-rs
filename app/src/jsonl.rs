use anyhow::{bail, ensure, Context, Result};
use labelme_rs::serde_json;

use lmrs::cli::JsonlCmdArgs as CmdArgs;

#[cfg(not(target_os = "windows"))]
extern crate libc;

pub fn cmd(args: CmdArgs) -> Result<()> {
    #[cfg(not(target_os = "windows"))]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    for input in args.input {
        ensure!(input.exists(), "Input directory {:?} not found.", input);
        ensure!(
            input.is_dir(),
            "Input {:?} is a file not a directory.",
            input
        );
        let entries: Vec<_> = glob::glob(
            input
                .join("*.json")
                .to_str()
                .context("Failed to obtain glob string")?,
        )
        .expect("Failed to read glob pattern")
        .collect();
        ensure!(
            !entries.is_empty(),
            "No json file found in the input directories."
        );
        for entry in entries {
            let input = entry?;
            let json_str = std::fs::read_to_string(&input)?;
            let mut json_data: serde_json::Map<String, serde_json::Value> =
                serde_json::from_str(&json_str)?;
            let should_be_none = json_data.insert(
                args.filename.clone(),
                input
                    .file_name()
                    .context("filename is missing")?
                    .to_string_lossy()
                    .into(),
            );
            if let Some(prev) = should_be_none {
                bail!(
                    "\"{}\" key already exists with value \"{}\"",
                    args.filename,
                    prev
                );
            }
            let line = serde_json::to_string(&json_data)?;
            println!("{line}");
        }
    }
    Ok(())
}
