use clap::Parser;
use std::path::{Path, PathBuf};
#[macro_use]
extern crate log;

/// Swap prefix of imagePath
#[derive(Parser, Debug)]
#[clap(name=env!("CARGO_BIN_NAME"), author, version, about, long_about = None)]
struct Args {
    /// Input json filename or json containing directory
    input: PathBuf,
    /// New imagePath prefix
    prefix: String,
    /// Output json filename or output directory
    output: Option<PathBuf>,
}

fn swap_prefix(
    input: &Path,
    prefix: &str,
    output: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut json_data = labelme_rs::LabelMeData::load(input)?;
    let prefix = prefix.strip_suffix('/').unwrap_or(prefix);
    let new_image_path = format!(
        "{}/{}",
        prefix,
        Path::new(&json_data.imagePath)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
    );
    json_data.imagePath = new_image_path;
    let output = output.unwrap_or(input);
    json_data.save(output)?;
    Ok(())
}

#[test]
fn test_swap_prefix() {
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.push("tests/img1.json");
    println!("{:?}", filename);
    let original_data = labelme_rs::LabelMeData::load(&filename).unwrap();
    let output_filename = PathBuf::from("tests/output/img1_swapped.json");
    assert!(swap_prefix(&filename, "..", Some(&output_filename),).is_ok());
    let swapped_data = labelme_rs::LabelMeData::load(&output_filename).unwrap();
    assert_eq!(
        format!("../{}", original_data.imagePath),
        swapped_data.imagePath
    );
    assert!(swap_prefix(&filename, "../", Some(&output_filename),).is_ok());
    let swapped_data = labelme_rs::LabelMeData::load(&output_filename).unwrap();
    assert_eq!(
        format!("../{}", original_data.imagePath),
        swapped_data.imagePath
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    if args.input.extension().unwrap_or_default() == "json" {
        if let Some(output) = &args.output {
            assert_eq!(
                output.extension().unwrap_or_default(),
                "json",
                "Output needs to be a json when input is a json"
            )
        };
        info!("Process single file");
        swap_prefix(&args.input, &args.prefix, args.output.as_deref())
    } else {
        if let Some(output) = &args.output {
            assert!(output.exists(), "Output does not exist");
            assert!(
                output.is_dir(),
                "Output needs to be a directory when input is a directory"
            );
        };
        info!("Process a directory");
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
            swap_prefix(&input, &args.prefix, args.output.as_deref())?;
            bar.inc(1);
        }
        bar.finish();
        Ok(())
    }
}
