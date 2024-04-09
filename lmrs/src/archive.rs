use std::{fs::File, path::PathBuf};

use anyhow::{bail, Context, Result};

use lmrs::cli::ArchiveCmdArgs as CmdArgs;
use tar::Builder;

pub fn cmd(args: CmdArgs) -> Result<()> {
    if args.input.is_file() {
        bail!("File input is not implemented")
    }
    let entries = glob::glob(
        args.input
            .join("*.json")
            .to_str()
            .context("Failed to obtain glob string")?,
    )
    .expect("Failed to read glob pattern");
    let json_dir = args.input.canonicalize()?;
    let output_file = std::fs::File::create(&args.output)
        .with_context(|| format!("Failed to create file: {:?}", args.output))?;

    let mut ar = Builder::new(output_file);

    for entry in entries {
        let input = entry?;
        ar.append_file(
            input.file_name().unwrap().to_str().unwrap(),
            &mut File::open(&input)?,
        )?;
        let data = labelme_rs::LabelMeData::try_from(input.as_path())?.to_absolute_path(&json_dir);
        let image_path: PathBuf = data.imagePath.into();
        let mut image_file = File::open(&image_path)
            .with_context(|| format!("Failed to open image file: {:?}", image_path))?;
        let image_name = image_path.file_name().unwrap().to_str().unwrap();
        ar.append_file(image_name, &mut image_file)?;
    }
    Ok(())
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
        let result = cmd(args);
        assert!(result.is_ok());
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
            assert_eq!(unarchived, original)
        }

        remove_file(output.path())?;
        Ok(())
    }
}
