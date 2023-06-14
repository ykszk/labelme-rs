use clap::Args;
use labelme_rs::serde_json;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct CmdArgs {
    /// Input jsonl filename. Stdin is used if omitted
    input: Option<PathBuf>,
    /// Output directory. Working directory is used by default
    #[clap(short, long)]
    output: Option<PathBuf>,
    /// Key for filename
    #[clap(long, default_value = "filename", id = "key")]
    filename: String,
    /// Overwrite json files if exist
    #[clap(long, action)]
    overwrite: bool,
}

pub fn cmd(args: CmdArgs) -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let reader: Box<dyn BufRead> = match args.input {
        None => Box::new(BufReader::new(std::io::stdin())),
        Some(filename) => Box::new(BufReader::new(File::open(filename).unwrap())),
    };
    let outdir = args.output.unwrap_or_else(PathBuf::new);
    for line in reader.lines() {
        let mut json_data: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&line?).unwrap();
        let v_filename = json_data
            .remove(&args.filename)
            .ok_or_else(|| format!("Key '{}' not found", args.filename))?;
        let filename = match v_filename {
            serde_json::Value::String(s) => s,
            _ => panic!("expected String"),
        };
        let output_filename = outdir.join(filename);
        if output_filename.exists() && !args.overwrite {
            return Err(format!(
                "Output file {:?} already exists. Add \"--overwrite\" option to force overwriting.",
                output_filename
            )
            .into());
        }
        let writer = std::io::BufWriter::new(std::fs::File::create(&output_filename)?);
        serde_json::to_writer_pretty(writer, &json_data)
            .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)?;
    }
    Ok(())
}