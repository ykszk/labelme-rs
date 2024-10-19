use anyhow::Result;
use labelme_rs::indexmap::IndexMap;
use labelme_rs::{serde_json, LabelMeData, LabelMeDataLine, Shape};
use std::fs::File;
use std::io::{BufRead, BufReader};

use lmrs::cli::SortCmdArgs as CmdArgs;

/// Collection of shape_type -> shape_label -> shapes
#[derive(Debug)]
struct ShapeMap {
    shapes: IndexMap<String, IndexMap<String, Vec<Shape>>>,
}

impl From<LabelMeData> for ShapeMap {
    fn from(data: LabelMeData) -> Self {
        let mut shapes = IndexMap::new();
        for shape in data.shapes {
            let shape_label = shape.label.clone();
            let shape_type = shape.shape_type.clone();
            shapes
                .entry(shape_type)
                .or_insert_with(IndexMap::new)
                .entry(shape_label)
                .or_insert_with(Vec::new)
                .push(shape);
        }
        Self { shapes }
    }
}

impl ShapeMap {
    /// Sorts the shapes by point coordinates
    pub fn sort(&mut self, by_x: bool, descending: bool) {
        for shapes in self.shapes.values_mut() {
            for shapes in shapes.values_mut() {
                shapes.sort_by(|a, b| {
                    let a0 = a.points.first().unwrap();
                    let b0 = b.points.first().unwrap();
                    if by_x {
                        if descending {
                            b0.0.partial_cmp(&a0.0).unwrap()
                        } else {
                            a0.0.partial_cmp(&b0.0).unwrap()
                        }
                    } else if descending {
                        b0.1.partial_cmp(&a0.1).unwrap()
                    } else {
                        a0.1.partial_cmp(&b0.1).unwrap()
                    }
                });
            }
        }
    }
}

fn process_data(data: LabelMeData, sort_by_x: bool, descending: bool) -> LabelMeData {
    let mut shape_map = ShapeMap::from(data.clone());
    shape_map.sort(sort_by_x, descending);

    LabelMeData {
        shapes: shape_map
            .shapes
            .into_iter()
            .flat_map(|(_, shapes)| shapes.into_iter().flat_map(|(_, shapes)| shapes))
            .collect(),
        ..data
    }
}

pub fn cmd(args: CmdArgs) -> Result<()> {
    if args.input.extension().is_some_and(|ext| ext == "json") {
        let reader = BufReader::new(File::open(&args.input)?);
        let data: LabelMeData = serde_json::from_reader(reader)?;
        let sorted_data = process_data(data, args.by_x, args.descending);
        println!("{}", serde_json::to_string_pretty(&sorted_data)?);
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
            let sorted_data = process_data(lm_data_line.content, args.by_x, args.descending);
            let sorted_data_line = LabelMeDataLine {
                content: sorted_data,
                ..lm_data_line
            };
            println!("{}", serde_json::to_string(&sorted_data_line)?);
        }
    } else {
        panic!("Unknown input type: {:?}", args.input);
    }
    Ok(())
}
