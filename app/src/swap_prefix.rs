use anyhow::{anyhow, ensure, Context, Result};
use labelme_rs::serde_json;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use lmrs::cli::SwapCmdArgs as CmdArgs;

trait ImagePath {
    fn image_path(&self) -> &str;
}

impl ImagePath for labelme_rs::LabelMeData {
    fn image_path(&self) -> &str {
        &self.imagePath
    }
}

type JsonMap = serde_json::Map<String, serde_json::Value>;
fn swap_prefix(key: &str, prefix: &str, mut json_data: JsonMap) -> Result<JsonMap> {
    let entry = json_data.entry(key);
    match entry {
        serde_json::map::Entry::Vacant(_) => Err(anyhow!("{} not found", key)),
        serde_json::map::Entry::Occupied(mut value) => {
            let value = value.get_mut();
            let path = value
                .as_str()
                .with_context(|| format!("Value {} is not a string", &value))?
                .replace('\\', "/");
            let file_name = Path::new(&path)
                .file_name()
                .with_context(|| format!("Failed to do file_name: {}", path))?
                .to_str()
                .with_context(|| format!("Failed to convert osstr to str: {}", path))?;
            if prefix.is_empty() {
                *value = file_name.into();
            } else {
                *value = format!(
                    "{}{}{}",
                    prefix,
                    if prefix.ends_with('/') { "" } else { "/" },
                    Path::new(&value.as_str().unwrap().replace('\\', "/"))
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                )
                .into();
            }
            Ok(())
        }
    }?;
    Ok(json_data)
}

fn swap_prefix_file(
    input: &Path,
    key: &str,
    prefix: &str,
    output: &Path,
    pretty: bool,
) -> Result<()> {
    let json_str = std::fs::read_to_string(input)?;
    let json_data: JsonMap = serde_json::from_str(&json_str)?;
    let line = if pretty {
        serde_json::to_string_pretty(&swap_prefix(key, prefix, json_data)?)?
    } else {
        serde_json::to_string(&swap_prefix(key, prefix, json_data)?)?
    };
    let mut writer = std::io::BufWriter::new(std::fs::File::create(output)?);
    writeln!(writer, "{}", line)?;
    Ok(())
}

#[test]
fn test_swap_prefix() -> Result<()> {
    use std::path::PathBuf;

    let key = "imagePath";
    let pretty = true;
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.push("tests/img1.json");
    println!("{filename:?}");
    let original_data = labelme_rs::LabelMeData::try_from(filename.as_path()).unwrap();
    let output_filename = PathBuf::from("tests/output/img1_swapped.json");
    assert!(swap_prefix_file(&filename, key, "..", &output_filename, pretty).is_ok());
    let swapped_data = labelme_rs::LabelMeData::try_from(output_filename.as_path()).unwrap();
    assert_eq!(
        format!("../{}", original_data.imagePath),
        swapped_data.imagePath
    );
    assert!(swap_prefix_file(&filename, key, "../", &output_filename, pretty).is_ok());
    let swapped_data = labelme_rs::LabelMeData::try_from(output_filename.as_path()).unwrap();
    assert_eq!(
        format!("../{}", original_data.imagePath),
        swapped_data.imagePath
    );

    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.push("tests/backslash.json");
    println!("{filename:?}");
    let output_filename = PathBuf::from("tests/output/img1_swapped.json");
    assert!(swap_prefix_file(&filename, key, "..", &output_filename, pretty).is_ok());
    let swapped_data = labelme_rs::LabelMeData::try_from(output_filename.as_path()).unwrap();
    assert_eq!("../stem.jpg", swapped_data.imagePath);
    assert!(swap_prefix_file(&filename, key, "", &output_filename, pretty).is_ok());
    let swapped_data = labelme_rs::LabelMeData::try_from(output_filename.as_path()).unwrap();
    assert_eq!("stem.jpg", swapped_data.imagePath);
    Ok(())
}

pub fn cmd(args: CmdArgs) -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    if args.input.is_dir() {
        let output = args.output.unwrap_or_else(|| args.input.clone());
        debug!("Directory input");
        ensure!(
            output.exists(),
            "Output directory \"{}\" does not exist.",
            output.to_string_lossy()
        );
        ensure!(
            output.is_dir(),
            "Existing file \"{}\" found: directory output is required for directory input.",
            output.to_string_lossy()
        );
        let entries: Vec<_> = glob::glob(
            args.input
                .join("*.json")
                .to_str()
                .context("Failed to get glob")?,
        )
        .expect("Failed to read glob pattern")
        .collect();
        let bar = indicatif::ProgressBar::new(entries.len() as _);
        bar.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("[{elapsed}<{eta}] | {wide_bar} | {pos}/{len}")?,
        );
        for entry in entries {
            let input = entry?;
            let output = output
                .clone()
                .join(input.file_name().context("Failed to obtain filename")?);
            swap_prefix_file(&input, &args.key, &args.prefix, &output, true)?;
            bar.inc(1);
        }
        bar.finish();
    } else {
        debug!("File or stdin input");
        if args.input.extension().is_some_and(|ext| ext == "json") {
            // single json
            let output = args.output.unwrap_or_else(|| args.input.clone());
            swap_prefix_file(&args.input, &args.key, &args.prefix, &output, true)?;
        } else if args.input.as_os_str() == "-"
            || args
                .input
                .extension()
                .is_some_and(|ext| ext == "jsonl" || ext == "ndjson")
        {
            // jsonl or ndjson
            let reader: Box<dyn BufRead> = if args.input.as_os_str() == "-" {
                Box::new(BufReader::new(std::io::stdin()))
            } else {
                Box::new(BufReader::new(File::open(&args.input)?))
            };
            let mut writer: Box<dyn Write> = match args.output {
                Some(x) => {
                    if x.as_os_str() == "-" {
                        Box::new(BufWriter::new(std::io::stdout()))
                    } else {
                        Box::new(BufWriter::new(File::create(&x)?))
                    }
                }
                None => Box::new(BufWriter::new(std::io::stdout())),
            };
            for line in reader.lines() {
                let line = line?;
                let json_data: JsonMap = serde_json::from_str(&line)?;
                let json_data = swap_prefix(&args.key, &args.prefix, json_data)?;
                writeln!(writer, "{}", serde_json::to_string(&json_data)?)?;
            }
        } else {
            panic!("Unknown input type: {:?}", args.input);
        }
    }
    Ok(())
}
