use anyhow::{ensure, Context, Result};
use labelme_rs::{serde_json, LabelMeData, LabelMeDataLine};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use lmrs::cli::SwapCmdArgs as CmdArgs;

fn swap_prefix_file(input: &Path, prefix: &str, output: &Path, pretty: bool) -> Result<()> {
    let mut lm_data = LabelMeData::try_from(input)?;
    lm_data.swap_prefix(prefix)?;
    let line = if pretty {
        serde_json::to_string_pretty(&lm_data)?
    } else {
        serde_json::to_string(&lm_data)?
    };
    let mut writer = std::io::BufWriter::new(std::fs::File::create(output)?);
    writeln!(writer, "{}", line)?;
    Ok(())
}

trait Swap {
    fn swap_prefix(&mut self, prefix: &str) -> Result<()>
    where
        Self: Sized;
    fn swap_suffix(&mut self, suffix: &str) -> Result<()>
    where
        Self: Sized;
}

impl Swap for LabelMeData {
    fn swap_prefix(&mut self, prefix: &str) -> Result<()>
    where
        Self: Sized,
    {
        self.imagePath = self.imagePath.replace('\\', "/");
        let file_name = Path::new(&self.imagePath)
            .file_name()
            .with_context(|| format!("Failed to get file_name: {}", self.imagePath))?
            .to_str()
            .unwrap();
        if prefix.is_empty() {
            self.imagePath = file_name.into();
        } else {
            self.imagePath = format!("{}/{}", prefix, file_name);
        }
        Ok(())
    }

    fn swap_suffix(&mut self, suffix: &str) -> Result<()>
    where
        Self: Sized,
    {
        self.imagePath = self.imagePath.replace('\\', "/");
        self.imagePath = Path::new(&self.imagePath)
            .with_extension(suffix)
            .to_str()
            .unwrap()
            .into();
        Ok(())
    }
}

fn swap_suffix_file(input: &Path, suffix: &str, output: &Path, pretty: bool) -> Result<()> {
    let mut lm_data = LabelMeData::try_from(input)?;
    lm_data.swap_suffix(suffix)?;
    let line = if pretty {
        serde_json::to_string_pretty(&lm_data)?
    } else {
        serde_json::to_string(&lm_data)?
    };
    let mut writer = std::io::BufWriter::new(std::fs::File::create(output)?);
    writeln!(writer, "{}", line)?;
    Ok(())
}

#[test]
fn test_swap_prefix() -> Result<()> {
    use std::path::PathBuf;

    let pretty = true;
    let output_filename =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/output/img1_prefix_swapped.json");

    let filename = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/img1.json");
    println!("{filename:?}");
    let original_data = labelme_rs::LabelMeData::try_from(filename.as_path()).unwrap();
    assert!(swap_prefix_file(&filename, "..", &output_filename, pretty).is_ok());
    let swapped_data = labelme_rs::LabelMeData::try_from(output_filename.as_path()).unwrap();
    assert_eq!(
        format!("../{}", original_data.imagePath),
        swapped_data.imagePath
    );

    let filename = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/backslash.json");
    println!("{filename:?}");
    assert!(swap_prefix_file(&filename, "..", &output_filename, pretty).is_ok());
    let swapped_data = labelme_rs::LabelMeData::try_from(output_filename.as_path()).unwrap();
    assert_eq!("../stem.jpg", swapped_data.imagePath);
    assert!(swap_prefix_file(&filename, "", &output_filename, pretty).is_ok());
    let swapped_data = labelme_rs::LabelMeData::try_from(output_filename.as_path()).unwrap();
    assert_eq!("stem.jpg", swapped_data.imagePath);

    Ok(())
}

#[test]
fn test_swap_suffix() -> Result<()> {
    use std::path::PathBuf;
    let pretty = true;
    let output_filename =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/output/img1_suffix_swapped.json");

    let filename = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/img1.json");
    println!("{filename:?}");
    assert!(swap_suffix_file(&filename, "png", &output_filename, pretty).is_ok());
    let swapped_data = labelme_rs::LabelMeData::try_from(output_filename.as_path()).unwrap();
    assert_eq!("img1.png", swapped_data.imagePath);

    let filename = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/backslash.json");
    println!("{filename:?}");
    assert!(swap_suffix_file(&filename, "", &output_filename, pretty).is_ok());
    let swapped_data = labelme_rs::LabelMeData::try_from(output_filename.as_path()).unwrap();
    assert_eq!("parent/stem", swapped_data.imagePath);
    assert!(swap_suffix_file(&filename, "irregular", &output_filename, pretty).is_ok());
    let swapped_data = labelme_rs::LabelMeData::try_from(output_filename.as_path()).unwrap();
    assert_eq!("parent/stem.irregular", swapped_data.imagePath);

    Ok(())
}

pub fn cmd(args: CmdArgs) -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let sanitized_prefix_suffix = if args.suffix {
        args.prefix.trim_start_matches('.')
    } else {
        args.prefix.trim_end_matches('/')
    };

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
            if args.suffix {
                swap_suffix_file(&input, sanitized_prefix_suffix, &output, true)?;
            } else {
                swap_prefix_file(&input, sanitized_prefix_suffix, &output, true)?;
            }
            bar.inc(1);
        }
        bar.finish();
    } else {
        debug!("File or stdin input");
        if args.input.extension().is_some_and(|ext| ext == "json") {
            // single json
            let output = args.output.unwrap_or_else(|| args.input.clone());
            if args.suffix {
                swap_suffix_file(&args.input, sanitized_prefix_suffix, &output, true)?;
            } else {
                swap_prefix_file(&args.input, sanitized_prefix_suffix, &output, true)?;
            }
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
                let mut lm_data_line = LabelMeDataLine::try_from(line.as_str())?;
                if args.suffix {
                    lm_data_line.content.swap_suffix(sanitized_prefix_suffix)?;
                } else {
                    lm_data_line.content.swap_prefix(sanitized_prefix_suffix)?;
                }
                writeln!(writer, "{}", serde_json::to_string(&lm_data_line)?)?;
            }
        } else {
            panic!("Unknown input type: {:?}", args.input);
        }
    }
    Ok(())
}
