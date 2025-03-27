use anyhow::{Context, Result};
use labelme_rs::serde_json;
use std::fs::File;
use std::io::{BufRead, BufReader};

use lmrs::cli::{ReshapeType, ShapeshiftCmdArgs as CmdArgs};

fn reshape(shape: &mut labelme_rs::Shape, from: &str, to: &str, index: usize) {
    if shape.shape_type == from {
        let point = shape.points[index];
        shape.shape_type = to.to_string();
        shape.points = vec![point];
    }
}

fn change_shape(json_data_line: &mut labelme_rs::LabelMeDataLine, reshape_type: &ReshapeType) {
    json_data_line
        .content
        .shapes
        .iter_mut()
        .for_each(|shape| match reshape_type {
            ReshapeType::CirclePoint(args) => {
                reshape(shape, "circle", "point", args.index);
            }
            ReshapeType::PolyPoint(args) => {
                reshape(shape, "polygon", "point", args.index);
            }
        });
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
        let mut json_data_line: labelme_rs::LabelMeDataLine =
            serde_json::from_str(&line).with_context(|| format!("Processing line:{line}"))?;
        change_shape(&mut json_data_line, &args.reshape);
        serde_json::to_writer(writer.lock(), &json_data_line)?;
        println!();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use labelme_rs::LabelMeDataLine;
    use lmrs::cli::Reshape2Point;

    use super::*;
    fn read_to_line(name: &str) -> Result<String> {
        let json_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/data")
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
    fn test_c2p() -> Result<()> {
        let line = read_to_line("Mandrill.json")?;
        let mut original_data_line = LabelMeDataLine::try_from(line.as_str())?;
        let original_circles = original_data_line
            .content
            .shapes
            .iter()
            .filter(|shape| shape.shape_type == "circle")
            .collect::<Vec<_>>();
        assert!(!original_circles.is_empty());

        change_shape(
            &mut original_data_line,
            &ReshapeType::CirclePoint(Reshape2Point { index: 0 }),
        );
        let reshaped_circles: Vec<_> = original_data_line
            .content
            .shapes
            .iter()
            .filter(|shape| shape.shape_type == "circle")
            .collect::<Vec<_>>();
        assert!(reshaped_circles.is_empty());
        for reshaped_circle in reshaped_circles {
            assert_eq!(reshaped_circle.shape_type, "point");
            assert_eq!(reshaped_circle.points.len(), 1);
        }
        Ok(())
    }
}
