use anyhow::{ensure, Context, Result};
use labelme_rs::serde_json;
use std::fs::File;
use std::io::{BufRead, BufReader};

use lmrs::cli::SplitCmdArgs as CmdArgs;

pub fn cmd(args: CmdArgs) -> Result<()> {
    let reader: Box<dyn BufRead> = match args.input {
        None => Box::new(BufReader::new(std::io::stdin())),
        Some(filename) => Box::new(BufReader::new(
            File::open(&filename).with_context(|| format!("Opening {:?}", filename))?,
        )),
    };
    let outdir = args.output.unwrap_or_default();
    for line in reader.lines() {
        let json_data: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&line?)?;
        let v_filename = json_data
            .get(&args.filename)
            .with_context(|| format!("Key {} not found", &args.filename))?;
        let serde_json::Value::String(filename) = v_filename else {
            panic!("expected String")
        };
        let output_filename = outdir.join(filename);
        if !args.overwrite {
            ensure!(!output_filename.exists(),
            "Output file {output_filename:?} already exists. Add \"--overwrite\" option to force overwriting.");
        }
        let writer = std::io::BufWriter::new(
            std::fs::File::create(&output_filename)
                .with_context(|| format!("Writing to {:?}", output_filename))?,
        );
        serde_json::to_writer_pretty(writer, &json_data.get(&args.content))?;
    }
    Ok(())
}
