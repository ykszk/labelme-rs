use anyhow::{Context, Result};
use labelme_rs::serde_json;
use std::fs::File;
use std::io::{BufRead, BufReader};

use lmrs::cli::{PruneCmdArgs as CmdArgs, PruneMode};

fn prune_points(mut data: labelme_rs::LabelMeData, mode: PruneMode) -> labelme_rs::LabelMeData {
    let width = data.imageWidth as f64;
    let height = data.imageHeight as f64;
    
    data.shapes.retain(|shape| {
        let outside_count = shape.points.iter().filter(|point| {
            point.0 < 0.0 || point.0 >= width || point.1 < 0.0 || point.1 >= height
        }).count();
        !match mode {
            PruneMode::Any => outside_count > 0,
            PruneMode::Majority => outside_count * 2 > shape.points.len(),
            PruneMode::All => outside_count == shape.points.len(),
        }
    });
    
    data
}

fn process_json(args: CmdArgs) -> Result<()> {
    let json_str = std::fs::read_to_string(&args.input)?;
    let json_data: labelme_rs::LabelMeData = serde_json::from_str(&json_str)?;
    let pruned_data = prune_points(json_data, args.mode);
    println!("{}", serde_json::to_string(&pruned_data)?);
    Ok(())
}

pub fn cmd(args: CmdArgs) -> Result<()> {
    if args.input.extension().unwrap_or_default() == "json" {
        return process_json(args);
    }
    
    let reader: Box<dyn BufRead> = if args.input.as_os_str() == "-" {
        Box::new(BufReader::new(std::io::stdin()))
    } else {
        Box::new(BufReader::new(File::open(&args.input)?))
    };
    
    for line in reader.lines() {
        let line = line?;
        let mut json_data: labelme_rs::LabelMeDataLine =
            serde_json::from_str(&line).with_context(|| format!("Processing line: {line}"))?;
        
        json_data.content = prune_points(json_data.content, args.mode);
        println!("{}", serde_json::to_string(&json_data)?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use labelme_rs::{LabelMeData, Shape};

    #[test]
    fn test_prune_points() {
        let data = LabelMeData {
            version: "4.5.7".into(),
            flags: labelme_rs::Flags::new(),
            shapes: vec![
                Shape {
                    label: "inside".into(),
                    points: vec![(10.0, 10.0), (50.0, 50.0)], // Both inside
                    group_id: None,
                    shape_type: "point".into(),
                    flags: labelme_rs::Flags::new(),
                },
                Shape {
                    label: "mixed".into(),
                    points: vec![(10.0, 10.0), (50.0, 50.0), (150.0, 150.0)], // Two inside, one outside
                    group_id: None,
                    shape_type: "polygon".into(),
                    flags: labelme_rs::Flags::new(),
                },
                Shape {
                    label: "outside".into(),
                    points: vec![(-10.0, -10.0), (150.0, 150.0)], // Both outside
                    group_id: None,
                    shape_type: "point".into(),
                    flags: labelme_rs::Flags::new(),
                },
            ],
            imagePath: "test.jpg".into(),
            imageData: None,
            imageHeight: 100,
            imageWidth: 100,
        };

        let pruned = prune_points(data.clone(), PruneMode::Any);
        
        // Should have 1 shape remaining (inside)
        assert_eq!(pruned.shapes.len(), 1);

        let pruned = prune_points(data.clone(), PruneMode::All);

        // 2 shape remaining (inside)
        assert_eq!(pruned.shapes.len(), 2);

        let pruned = prune_points(data, PruneMode::Majority);
        
        // 2 shapes remaining (inside and mixed)
        assert_eq!(pruned.shapes.len(), 2);        
    }

    #[test]
    fn test_boundary_points() {
        let shapes = [(0.0, 0.0), (99.0, 99.0), (100.0, 100.0)]
            .iter()
            .map(|&(x, y)| Shape {
                label: "boundary".into(),
                points: vec![(x, y)],
                group_id: None,
                shape_type: "point".into(),
                flags: labelme_rs::Flags::new(),
            })
            .collect::<Vec<_>>();
        let data = LabelMeData {
            version: "4.5.7".into(),
            flags: labelme_rs::Flags::new(),
            shapes,
            imagePath: "test.jpg".into(),
            imageData: None,
            imageHeight: 100,
            imageWidth: 100,
        };

        let pruned = prune_points(data, PruneMode::Any);
        
        // Should keep points at (0,0) and (99,99) but not (100,100)
        assert_eq!(pruned.shapes.len(), 2);
    }
}
