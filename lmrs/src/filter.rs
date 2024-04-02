use anyhow::{Context, Result};
use labelme_rs::serde_json;
use std::fs::File;
use std::io::{BufRead, BufReader};

use lmrs::cli::FilterCmdArgs as CmdArgs;

pub fn cmd(args: CmdArgs) -> Result<()> {
    let mut rules: Vec<String> = Vec::new();
    for filename in args.rules {
        let ar = lmrs::load_rules(&filename)
            .with_context(|| format!("Reading rule file {filename:?}"))?;
        rules.extend(ar);
    }
    assert!(!rules.is_empty(), "No rule is found.");
    let asts = lmrs::parse_rules(&rules)?;
    let reader: Box<dyn BufRead> = if args.input.as_os_str() == "-" {
        Box::new(BufReader::new(std::io::stdin()))
    } else {
        Box::new(BufReader::new(File::open(&args.input)?))
    };
    for line in reader.lines() {
        let line = line?;
        let json_data: labelme_rs::LabelMeDataLine =
            serde_json::from_str(&line).with_context(|| format!("Processing line:{line}"))?;
        let errors = lmrs::evaluate_rules(&rules, &asts, json_data.content.shapes);
        if errors.is_empty() ^ args.invert {
            println!("{}", line);
        }
    }
    Ok(())
}
