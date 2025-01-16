use anyhow::{bail, Result};
use labelme_rs::indexmap::{IndexMap, IndexSet};
use labelme_rs::{serde_json, LabelMeDataLine};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use lmrs::cli::{MergeCmdArgs as CmdArgs, MissingHandling};

fn load_ndjson(input: &Path) -> Result<IndexMap<String, LabelMeDataLine>> {
    let reader: Box<dyn BufRead> = if input.as_os_str() == "-" {
        Box::new(BufReader::new(std::io::stdin()))
    } else {
        Box::new(BufReader::new(File::open(input)?))
    };
    let ndjson: Result<IndexMap<String, LabelMeDataLine>> = reader
        .lines()
        .map(|line| {
            let line = line?;
            let obj = LabelMeDataLine::try_from(line.as_str())?;
            Ok((obj.filename.clone(), obj))
        })
        .collect();
    ndjson
}

fn merge(
    left: IndexMap<String, LabelMeDataLine>,
    right: IndexMap<String, LabelMeDataLine>,
    missing_handling: MissingHandling,
) -> Result<IndexMap<String, LabelMeDataLine>> {
    let mut left = left;
    for (key, right_obj) in right {
        match left.entry(key) {
            labelme_rs::indexmap::map::Entry::Occupied(mut left_obj) => {
                let left = left_obj.get_mut();
                left.content
                    .shapes
                    .extend_from_slice(right_obj.content.shapes.as_slice());
            }
            labelme_rs::indexmap::map::Entry::Vacant(entry) => {
                if missing_handling == MissingHandling::Exit {
                    bail!("Key {} not found in left object", entry.key());
                } else {
                    debug!("Key {} not found in left object", entry.key());
                }
            }
        }
    }
    Ok(left)
}

pub fn cmd(args: CmdArgs) -> Result<()> {
    let input_set: IndexSet<PathBuf> = IndexSet::from_iter(args.input);
    anyhow::ensure!(input_set.len() > 1, "Need more than one input");
    debug!("Read and join ndjsons");
    let joined: Result<IndexMap<String, LabelMeDataLine>, _> = input_set
        .iter()
        .map(|input| load_ndjson(input))
        .reduce(|l, r| l.and_then(|l| r.map(|r| merge(l, r, args.missing)))?)
        .unwrap();
    debug!("Print result");
    for (_key, obj) in joined? {
        println!("{}", serde_json::to_string(&obj)?);
    }
    debug!("Done");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lmrs::LabelMeData;
    use std::path::PathBuf;

    #[test]
    fn test_merge() -> Result<()> {
        let data: LabelMeData = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/data")
            .join("Mandrill.json")
            .as_path()
            .try_into()?;
        let data_line = LabelMeDataLine {
            filename: "Mandrill".to_string(),
            content: data,
        };

        // test normal merge
        let left = IndexMap::from_iter(vec![("Mandrill".to_string(), data_line.clone())]);
        let right = IndexMap::from_iter(vec![("Mandrill".to_string(), data_line.clone())]);
        let merged = merge(left, right, MissingHandling::Exit)?;
        assert_eq!(merged.len(), 1);
        let merged_data = merged.get("Mandrill").unwrap();
        assert_eq!(
            merged_data.content.shapes.len(),
            data_line.content.shapes.len() * 2
        );

        // test missing_handling
        let empty = IndexMap::new();
        let right = IndexMap::from_iter(vec![("Mandrill".to_string(), data_line.clone())]);
        let result = merge(empty, right, MissingHandling::Exit);
        assert!(result.is_err());

        let empty = IndexMap::new();
        let right = IndexMap::from_iter(vec![("Mandrill".to_string(), data_line.clone())]);
        let result = merge(empty, right, MissingHandling::Continue);
        assert!(result.is_ok());

        Ok(())
    }
}
