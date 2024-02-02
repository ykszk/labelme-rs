use anyhow::{Context, Result};
use labelme_rs::serde_json;
use log::debug;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use lmrs::cli::ExistCmdArgs as CmdArgs;

pub fn cmd(args: CmdArgs) -> Result<()> {
    let (reader, json_parent_dir): (Box<dyn BufRead>, PathBuf) = if args.input.as_os_str() == "-" {
        (
            Box::new(BufReader::new(std::io::stdin())),
            PathBuf::from("."),
        )
    } else {
        (
            Box::new(BufReader::new(
                File::open(&args.input)
                    .with_context(|| format!("opening {}", args.input.display()))?,
            )),
            args.input.parent().unwrap().to_path_buf(),
        )
    };
    let json_parent_dir = json_parent_dir.canonicalize()?;
    debug!("json_parent_dir: {:?}", json_parent_dir);

    for line in reader.lines() {
        let line = line.with_context(|| format!("reading line from {}", args.input.display()))?;
        let lmdata_line: labelme_rs::LabelMeDataLine = serde_json::from_str(&line)
            .with_context(|| format!("Failed to parse JSON from the input line: {}", line))?;
        let lmdata = lmdata_line.content.to_absolute_path(&json_parent_dir);
        let image_path = Path::new(&lmdata.imagePath);
        if args.invert ^ image_path.exists() {
            println!("{}", line);
        }
    }
    Ok(())
}
