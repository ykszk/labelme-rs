use anyhow::{Context, Result};
use labelme_rs::indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use labelme_rs::{serde_json, LabelMeData, LabelMeDataLine};
use std::fs::File;
use std::io::{BufRead, BufReader};

use lmrs::cli::CountCmdArgs as CmdArgs;

#[derive(Serialize, Deserialize, Debug)]
struct Counts {
    flags: IndexMap<String, usize>,
}

impl Counts {
    pub fn new() -> Self {
        Self {
            flags: IndexMap::new(),
        }
    }

    pub fn count(&mut self, data: LabelMeData) {
        for (name, state) in data.flags {
            if state {
                *self.flags.entry(name).or_insert(0) += 1;
            }
        }
    }
}

pub fn cmd(args: CmdArgs) -> Result<()> {
    let mut counts = Counts::new();
    if args.input.is_dir() {
        let entries: Vec<_> = glob::glob(
            args.input
                .join("*.json")
                .to_str()
                .context("Failed to get glob")?,
        )
        .expect("Failed to read glob pattern")
        .collect();
        for entry in entries {
            let entry = entry?;
            let reader = BufReader::new(File::open(&entry)?);
            let data: LabelMeData = serde_json::from_reader(reader)?;
            counts.count(data);
        }
    } else {
        debug!("File or stdin input");
        if args.input.extension().is_some_and(|ext| ext == "json") {
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
            for line in reader.lines() {
                let line = line?;
                let lm_data_line = LabelMeDataLine::try_from(line.as_str())?;
                counts.count(lm_data_line.content);
            }
        } else {
            panic!("Unknown input type: {:?}", args.input);
        }
    }
    println!("{}", serde_json::to_string_pretty(&counts)?);
    Ok(())
}
