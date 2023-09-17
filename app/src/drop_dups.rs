use labelme_rs::serde_json;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};

use lmrs::cli::DropCmdArgs as CmdArgs;

pub fn cmd(args: CmdArgs) -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let reader: Box<dyn BufRead> = if args.input.as_os_str() == "-" {
        Box::new(BufReader::new(std::io::stdin()))
    } else {
        Box::new(BufReader::new(File::open(&args.input).unwrap()))
    };
    let mut existing_set: HashSet<String> = HashSet::new();
    for line in reader.lines() {
        let line = line?;
        let json_data: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&line).unwrap();
        let value = json_data
            .get(&args.key)
            .ok_or_else(|| format!("Key '{}' not found", args.key))?;
        if let serde_json::Value::String(value) = value {
            if existing_set.insert(value.clone()) {
                // HashSet::insert returns true when the given value is new
                println!("{}", line);
            }
        } else {
            panic!("Value for {} should be string. {} found", args.key, value);
        };
    }
    Ok(())
}
