use anyhow::{Context, Result};
use labelme_rs::indexmap::IndexMap;
use labelme_rs::serde_json;
use serde::Serialize;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

use lmrs::cli::StatsCmdArgs as CmdArgs;

type StatsType = IndexMap<String, IndexMap<String, usize>>;

fn count(shapes: Vec<labelme_rs::Shape>) -> StatsType {
    let mut count: StatsType = IndexMap::new();
    for shape in shapes {
        let label = shape.label.clone();
        let shape_type = shape.shape_type.clone();
        let entry = count.entry(shape_type).or_default();
        let entry = entry.entry(label).or_insert(0);
        *entry += 1;
    }
    count
}

fn process_json(args: CmdArgs, mut out: impl Write) -> Result<()> {
    let json_str = std::fs::read_to_string(&args.input)?;
    let json_data: labelme_rs::LabelMeData = serde_json::from_str(&json_str)?;
    let stats = count(json_data.shapes);
    writeln!(out, "{}", serde_json::to_string(&stats)?)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct StatsLine {
    content: StatsType,
    filename: String,
}

pub fn cmd(args: CmdArgs) -> Result<()> {
    if args.input.extension().unwrap_or_default() == "json" {
        return process_json(args, std::io::stdout());
    }
    let reader: Box<dyn BufRead> = if args.input.as_os_str() == "-" {
        Box::new(BufReader::new(std::io::stdin()))
    } else {
        Box::new(BufReader::new(File::open(&args.input)?))
    };
    for line in reader.lines() {
        let line = line?;
        let json_data: labelme_rs::LabelMeDataLine =
            serde_json::from_str(&line).with_context(|| format!("Processing line:{line}"))?;
        let stats = count(json_data.content.shapes);
        let stats_line = StatsLine {
            content: stats,
            filename: json_data.filename,
        };
        println!("{}", serde_json::to_string(&stats_line)?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_stats() -> Result<()> {
        let args = CmdArgs {
            input: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../tests/data")
                .join("Mandrill.json"),
        };
        let buffer: Vec<u8> = Vec::new();
        let mut writer = std::io::Cursor::new(buffer);
        process_json(args, &mut writer)?;
        let output = String::from_utf8(writer.into_inner())?;
        insta::assert_snapshot!("Mandrill", output);
        Ok(())
    }
}
