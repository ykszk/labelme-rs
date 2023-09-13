use clap::Args;
use labelme_rs::serde_json;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

#[derive(Args, Debug)]
pub struct CmdArgs {
    /// Input json or jsonl/ndjson filename or json containing directory. Specify `-` for jsonl input with stdin (for piping).
    input: PathBuf,
    /// New imagePath prefix
    prefix: String,
    /// Output json filename or output directory. Defaults: <INPUT> for directory or single file input, stdout for jsonl/ndjson input.
    output: Option<PathBuf>,
    /// Swap prefix of the value associated by the given key instead of `imagePath`
    #[clap(long, default_value = "imagePath")]
    key: String,
}

trait ImagePath {
    fn image_path(&self) -> &str;
}

impl ImagePath for labelme_rs::LabelMeData {
    fn image_path(&self) -> &str {
        &self.imagePath
    }
}

type JsonMap = serde_json::Map<String, serde_json::Value>;
fn swap_prefix(
    key: &str,
    prefix: &str,
    mut json_data: JsonMap,
) -> Result<JsonMap, Box<dyn std::error::Error>> {
    let prefix = prefix.strip_suffix('/').unwrap_or(prefix);
    let entry = json_data.entry(key).and_modify(|value| {
        *value = format!(
            "{}{}{}",
            prefix,
            if prefix.ends_with('/') || prefix.is_empty() {
                ""
            } else {
                "/"
            },
            Path::new(&value.as_str().unwrap().replace('\\', "/"))
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
        )
        .into();
    });
    if let serde_json::map::Entry::Vacant(_) = entry {
        panic!("`{}` key not found.", key);
    }
    Ok(json_data)
}

fn swap_prefix_file(
    input: &Path,
    key: &str,
    prefix: &str,
    output: &Path,
    pretty: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let json_str = std::fs::read_to_string(input)?;
    let json_data: JsonMap = serde_json::from_str(&json_str).unwrap();
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
fn test_swap_prefix() {
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
}

pub fn cmd(args: CmdArgs) -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    if args.input.is_dir() {
        let output = args.output.unwrap_or_else(|| args.input.clone());
        debug!("Directory input");
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
                .template("[{elapsed}<{eta}] | {wide_bar} | {pos}/{len}")?,
        );
        for entry in entries {
            let input = entry?;
            let output = output.clone().join(input.file_name().unwrap());
            swap_prefix_file(&input, &args.key, &args.prefix, &output, true)?;
            bar.inc(1);
        }
        bar.finish();
    } else {
        debug!("File or stdin input");
        if args
            .input
            .extension()
            .map(|ext| ext == "json")
            .unwrap_or(false)
        {
            // single json
            let output = args.output.unwrap_or_else(|| args.input.clone());
            swap_prefix_file(&args.input, &args.key, &args.prefix, &output, false)?;
        } else if args.input.as_os_str() == "-"
            || args
                .input
                .extension()
                .map(|ext| ext == "jsonl" || ext == "ndjson")
                .unwrap_or(false)
        {
            // jsonl or ndjson
            let reader: Box<dyn BufRead> = if args.input.as_os_str() == "-" {
                Box::new(BufReader::new(std::io::stdin()))
            } else {
                Box::new(BufReader::new(File::open(&args.input).unwrap()))
            };
            let mut writer: Box<dyn Write> = match args.output {
                Some(x) => {
                    if x.as_os_str() == "-" {
                        Box::new(BufWriter::new(std::io::stdout()))
                    } else {
                        Box::new(BufWriter::new(File::create(&x).unwrap()))
                    }
                }
                None => Box::new(BufWriter::new(std::io::stdout())),
            };
            for line in reader.lines() {
                let line = line?;
                let json_data: JsonMap = serde_json::from_str(&line).unwrap();
                let json_data = swap_prefix(&args.key, &args.prefix, json_data)?;
                writeln!(writer, "{}", serde_json::to_string(&json_data)?)?;
            }
        } else {
            panic!("Unknown input type: {:?}", args.input);
        }
    }
    Ok(())
}
