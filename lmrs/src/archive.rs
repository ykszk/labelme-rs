use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use labelme_rs::{serde_json, LabelMeData, LabelMeDataLine};
use lmrs::cli::ArchiveCmdArgs as CmdArgs;
use tar::{Builder, Header};

fn add_image<W: std::io::Write>(data: &LabelMeData, ar: &mut Builder<W>) -> Result<()> {
    let image_path: PathBuf = data.imagePath.clone().into();
    let mut image_file = File::open(&image_path)
        .with_context(|| format!("Failed to open image file: {:?}", image_path))?;
    let image_name = image_path.file_name().unwrap().to_str().unwrap();
    ar.append_file(image_name, &mut image_file)?;
    Ok(())
}

fn add_data<W: std::io::Write, P: AsRef<Path>>(
    path: P,
    data: &mut LabelMeData,
    ar: &mut Builder<W>,
) -> Result<()> {
    data.imagePath = Path::new(&data.imagePath)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let json = serde_json::to_string(data)?;
    let mut header = Header::new_gnu();
    header.set_size(json.len() as u64);
    header.set_mode(0o644);
    ar.append_data(&mut header, path, json.as_bytes())?;
    Ok(())
}

fn archive<W: std::io::Write>(args: CmdArgs, ar: Builder<W>) -> Result<()> {
    let mut ar = ar;
    if args.input.is_file() || args.input.as_os_str() == "-" {
        // process ndjson file
        let (reader, json_dir): (Box<dyn BufRead>, _) = if args.input.as_os_str() == "-" {
            let reader = Box::new(BufReader::new(std::io::stdin()));
            let json_dir = std::env::current_dir()?.canonicalize()?;
            (reader, json_dir)
        } else {
            let reader = Box::new(BufReader::new(File::open(&args.input)?));
            let json_dir = args.input.parent().unwrap().canonicalize()?;
            (reader, json_dir)
        };

        for line in reader.lines() {
            let line = line?;
            let data_line: LabelMeDataLine = serde_json::from_str(&line)?;

            let mut data = data_line.content.to_absolute_path(&json_dir);
            let path = Path::new(&data_line.filename).file_name().unwrap();
            add_image(&data, &mut ar)?;
            add_data(path, &mut data, &mut ar)?;
        }
    } else {
        let entries = glob::glob(
            args.input
                .join("*.json")
                .to_str()
                .context("Failed to obtain glob string")?,
        )
        .expect("Failed to read glob pattern");
        let json_dir = args.input.canonicalize()?;

        for entry in entries {
            let input = entry?;
            let mut data = LabelMeData::try_from(input.as_path())?.to_absolute_path(&json_dir);
            add_image(&data, &mut ar)?;
            add_data(input.file_name().unwrap(), &mut data, &mut ar)?;
        }
    }
    ar.finish()?;
    Ok(())
}

pub fn cmd(args: CmdArgs) -> Result<()> {
    if args.output.as_os_str() == "-" {
        archive(args, Builder::new(std::io::stdout()))
    } else {
        let output_file = std::fs::File::create(&args.output)
            .with_context(|| format!("Failed to create file: {:?}", args.output))?;
        archive(args, Builder::new(output_file))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::remove_file, io::Read};

    #[test]
    fn test_archive() -> Result<()> {
        let data_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/data");
        let output = tempfile::NamedTempFile::with_prefix(".tar")?;

        let args = CmdArgs {
            input: data_dir.clone(),
            output: output.path().into(),
        };
        cmd(args)?;
        let file = File::open(output.path())?;
        let mut a = tar::Archive::new(file);

        for file in a.entries()? {
            let mut file = file?;

            println!("{:?}", file.header().path()?);
            println!("{}", file.header().size()?);

            let mut unarchived = Vec::new();
            let _ = file.read_to_end(&mut unarchived)?;

            let mut original = Vec::new();
            let _ = File::open(data_dir.join(file.header().path().unwrap()).as_path())
                .unwrap()
                .read_to_end(&mut original)?;
            if file.path().unwrap().to_str().unwrap().ends_with(".json") {
                assert_eq!(
                    serde_json::from_slice::<LabelMeData>(&unarchived).unwrap(),
                    serde_json::from_slice::<LabelMeData>(&original).unwrap()
                );
            } else {
                assert_eq!(unarchived, original)
            }
        }

        remove_file(output.path())?;
        Ok(())
    }
}
