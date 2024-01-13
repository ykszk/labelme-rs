use anyhow::{bail, Context, Result};
use labelme_rs::serde_json;

use lmrs::cli::InitCmdArgs as CmdArgs;

pub fn cmd(args: CmdArgs) -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    if args.input.is_dir() {
        let entries = glob::glob(
            args.input
                .join(format!("*.{}", args.extension))
                .to_str()
                .context("Failed to obtain glob string")?,
        )
        .expect("Failed to read glob pattern");
        for entry in entries {
            let input = entry?;
            let mut filename = input.clone();
            filename.set_extension("json");
            let mut json_data = labelme_rs::LabelMeDataLine {
                filename: filename
                    .file_name()
                    .unwrap()
                    .to_os_string()
                    .into_string()
                    .unwrap(),
                ..Default::default()
            };
            json_data.content.imagePath = input
                .file_name()
                .unwrap()
                .to_os_string()
                .into_string()
                .unwrap();
            let line = serde_json::to_string(&json_data)?;
            println!("{line}");
        }
    } else {
        bail!("Single file input is not implemented")
    }
    Ok(())
}
