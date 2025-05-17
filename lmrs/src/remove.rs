use anyhow::{Context, Result};
use labelme_rs::serde_json;
use std::fs::File;
use std::io::{BufRead, BufReader};

use lmrs::cli::RemoveCmdArgs as CmdArgs;

fn remove(
    line: &str,
    shapes: &[String],
    labels: &[String],
    invert: bool,
) -> Result<labelme_rs::LabelMeDataLine> {
    let mut json_data_line: labelme_rs::LabelMeDataLine =
        serde_json::from_str(line).with_context(|| format!("Processing line:{line}"))?;
    json_data_line.content.shapes.retain(|shape| {
        let to_remove = shapes.contains(&shape.shape_type) || labels.contains(&shape.label);
        if invert {
            to_remove
        } else {
            !to_remove
        }
    });
    Ok(json_data_line)
}

pub fn cmd(args: CmdArgs) -> Result<()> {
    let reader: Box<dyn BufRead> = if args.input.as_os_str() == "-" {
        Box::new(BufReader::new(std::io::stdin()))
    } else {
        Box::new(BufReader::new(File::open(&args.input)?))
    };
    let writer = std::io::stdout();
    for line in reader.lines() {
        let line = line?;
        let json_data_line = remove(&line, &args.remove.shape, &args.remove.label, args.invert)?;
        serde_json::to_writer(writer.lock(), &json_data_line)?;
        println!();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn read_to_line(name: &str) -> Result<String> {
        let json_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join(name);
        let labelme_data =
            labelme_rs::LabelMeData::try_from(std::fs::read_to_string(json_path)?.as_str());
        let labelme_data_line = labelme_rs::LabelMeDataLine {
            filename: name.to_string(),
            content: labelme_data?,
        };
        let line = serde_json::to_string(&labelme_data_line)?;
        Ok(line)
    }

    #[test]
    fn test_process_json_line() -> Result<()> {
        let labels = vec!["TL".to_string()];
        let line = read_to_line("img1.json")?;
        let json_data_line = remove(&line, &[], &labels, false)?;
        assert_eq!(json_data_line.content.shapes.len(), 0);
        let json_data_line = remove(&line, &[], &labels, true)?;
        assert_eq!(json_data_line.content.shapes.len(), 1);

        let line = read_to_line("test.json")?;
        let json_data_line = remove(&line, &[], &labels, false)?;
        assert_eq!(json_data_line.content.shapes.len(), 3);
        let json_data_line = remove(&line, &[], &labels, true)?;
        assert_eq!(json_data_line.content.shapes.len(), 1);

        // Test removing shapes
        let shapes = vec!["point".to_string()];
        let json_data_line = remove(&line, &shapes, &[], false)?;
        assert_eq!(json_data_line.content.shapes.len(), 0);
        Ok(())
    }
}
