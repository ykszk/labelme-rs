use clap::Args;
use std::{
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Args, Debug)]
pub struct CmdArgs {
    /// Input json filename or json containing directory
    input: PathBuf,
    /// New imagePath prefix
    prefix: String,
    /// Output json filename or output directory
    output: Option<PathBuf>,
}

fn swap_prefix(
    prefix: &str,
    mut json_data: labelme_rs::LabelMeData,
) -> Result<labelme_rs::LabelMeData, Box<dyn std::error::Error>> {
    let prefix = prefix.strip_suffix('/').unwrap_or(prefix);
    let new_image_path = format!(
        "{}/{}",
        prefix,
        Path::new(&json_data.imagePath.replace('\\', "/"))
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
    );
    json_data.imagePath = new_image_path;
    Ok(json_data)
}

fn swap_prefix_file(
    input: &Path,
    prefix: &str,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut json_data = if input.as_os_str() == std::ffi::OsStr::new("-") {
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        labelme_rs::LabelMeData::try_from(buffer.as_str())?
    } else {
        labelme_rs::LabelMeData::try_from(input)?
    };
    json_data = swap_prefix(prefix, json_data)?;
    if input.as_os_str() == std::ffi::OsStr::new("-") {
        println!("{}", labelme_rs::serde_json::to_string_pretty(&json_data)?);
    } else {
        json_data.save(output)?;
    }
    Ok(())
}

#[test]
fn test_swap_prefix() {
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.push("tests/img1.json");
    println!("{:?}", filename);
    let original_data = labelme_rs::LabelMeData::try_from(filename.as_path()).unwrap();
    let output_filename = PathBuf::from("tests/output/img1_swapped.json");
    assert!(swap_prefix_file(&filename, "..", &output_filename).is_ok());
    let swapped_data = labelme_rs::LabelMeData::try_from(output_filename.as_path()).unwrap();
    assert_eq!(
        format!("../{}", original_data.imagePath),
        swapped_data.imagePath
    );
    assert!(swap_prefix_file(&filename, "../", &output_filename).is_ok());
    let swapped_data = labelme_rs::LabelMeData::try_from(output_filename.as_path()).unwrap();
    assert_eq!(
        format!("../{}", original_data.imagePath),
        swapped_data.imagePath
    );

    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.push("tests/backslash.json");
    println!("{:?}", filename);
    let output_filename = PathBuf::from("tests/output/img1_swapped.json");
    assert!(swap_prefix_file(&filename, "..", &output_filename).is_ok());
    let swapped_data = labelme_rs::LabelMeData::try_from(output_filename.as_path()).unwrap();
    assert_eq!("../stem.jpg", swapped_data.imagePath);
}

pub fn cmd(args: CmdArgs) -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let output = args.output.unwrap_or_else(|| args.input.clone());
    if args.input.is_dir() {
        if !output.exists() {
            return Err(format!(
                "Output directory \"{}\" does not exist.",
                output.to_string_lossy()
            )
            .into());
        };
        if !output.is_dir() {
            return Err(format!(
                "Existing file \"{}\" found: directory output is required for directory input.",
                output.to_string_lossy()
            )
            .into());
        }
        let entries: Vec<_> = glob::glob(args.input.join("*.json").to_str().unwrap())
            .expect("Failed to read glob pattern")
            .collect();
        let bar = indicatif::ProgressBar::new(entries.len() as _);
        bar.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("[{elapsed}<{eta}] | {wide_bar} | {pos}/{len}"),
        );
        for entry in entries {
            let input = entry?;
            let output = output.clone().join(input.file_name().unwrap());
            swap_prefix_file(&input, &args.prefix, &output)?;
            bar.inc(1);
        }
        bar.finish();
        Ok(())
    } else {
        // json file or '-'
        debug!("Process single file");
        swap_prefix_file(&args.input, &args.prefix, &output)
    }
}
