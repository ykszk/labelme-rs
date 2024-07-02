use anyhow::{bail, Result};
use labelme_rs::indexmap::{IndexMap, IndexSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

type JzonObject = jzon::JsonValue;
use lmrs::cli::JoinMode;
use lmrs::cli::{JoinCmdArgs as CmdArgs, MissingHandling};

fn load_ndjson(input: &Path, key: &str) -> Result<IndexMap<String, JzonObject>> {
    let reader: Box<dyn BufRead> = if input.as_os_str() == "-" {
        Box::new(BufReader::new(std::io::stdin()))
    } else {
        Box::new(BufReader::new(File::open(input)?))
    };
    let ndjson: Result<IndexMap<String, JzonObject>> = reader
        .lines()
        .map(|line| {
            let line = line?;
            let obj = jzon::parse(&line)?;
            match obj.get(key) {
                Some(value) => {
                    if let Some(s) = value.as_str() {
                        Ok((s.to_string(), obj))
                    } else {
                        bail!("Value for the key {} is not a string", key);
                    }
                }
                None => {
                    bail!("Key {} not found", key)
                }
            }
            .map(|(s, mut obj)| {
                obj.remove(key);
                (s, obj)
            })
        })
        .collect();
    ndjson
}

fn join_inner(
    left: IndexMap<String, JzonObject>,
    right: IndexMap<String, JzonObject>,
    missing_handling: MissingHandling,
) -> Result<IndexMap<String, JzonObject>> {
    let mut right = right;
    let mut joined = IndexMap::new();
    for (key, left_obj) in left {
        match right.swap_remove(&key) {
            Some(right_obj) => {
                let mut obj = left_obj;
                lmrs::merge(&mut obj, right_obj);
                joined.insert(key, obj);
            }
            None => {
                if missing_handling == MissingHandling::Exit {
                    bail!("Key {} not found in right object", key);
                } else {
                    debug!("Key {} not found in left object", key);
                }
            }
        }
    }
    Ok(joined)
}

fn join_left(
    left: IndexMap<String, JzonObject>,
    right: IndexMap<String, JzonObject>,
    missing_handling: MissingHandling,
) -> Result<IndexMap<String, JzonObject>> {
    let mut left = left;
    for (key, right_obj) in right {
        match left.entry(key) {
            labelme_rs::indexmap::map::Entry::Occupied(mut left_obj) => {
                lmrs::merge(left_obj.get_mut(), right_obj);
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

fn join_outer(
    left: IndexMap<String, JzonObject>,
    right: IndexMap<String, JzonObject>,
) -> Result<IndexMap<String, JzonObject>> {
    let mut left = left;
    for (key, right_obj) in right.into_iter() {
        let entry = left.entry(key);
        match entry {
            labelme_rs::indexmap::map::Entry::Occupied(mut left_obj) => {
                lmrs::merge(left_obj.get_mut(), right_obj);
            }
            labelme_rs::indexmap::map::Entry::Vacant(entry) => {
                entry.insert(right_obj);
            }
        }
    }
    Ok(left)
}

pub fn cmd(args: CmdArgs) -> Result<()> {
    let input_set: IndexSet<PathBuf> = IndexSet::from_iter(args.input);
    anyhow::ensure!(input_set.len() > 1, "Need more than one input");
    debug!("Read and join ndjsons");
    let joined: Result<IndexMap<String, JzonObject>, _> = input_set
        .iter()
        .map(|input| load_ndjson(input, &args.key))
        .reduce(|l, r| {
            l.and_then(|l| {
                r.map(|r| match args.mode {
                    JoinMode::Inner => join_inner(l, r, args.missing),
                    JoinMode::Left => join_left(l, r, args.missing),
                    JoinMode::Outer => join_outer(l, r),
                })
            })?
        })
        .unwrap();
    debug!("Print result");
    for (key, mut obj) in joined? {
        obj.insert(&args.key, key)?;
        let line = obj.to_string();
        println!("{}", line);
    }
    debug!("Done");
    Ok(())
}

#[test]
fn test_join() -> anyhow::Result<()> {
    let l: IndexMap<String, JzonObject> = IndexMap::from([("k1".into(), jzon::parse("{}")?)]);
    let r: IndexMap<String, JzonObject> = IndexMap::from([("k2".into(), jzon::parse("{}")?)]);

    // inner
    let joined = join_inner(l.clone(), r.clone(), MissingHandling::Exit);
    assert!(joined.is_err());

    let joined = join_inner(l.clone(), r.clone(), MissingHandling::Continue)?;
    assert!(!joined.contains_key("k1"));
    assert!(!joined.contains_key("k2"));

    // left
    let joined = join_left(l.clone(), r.clone(), MissingHandling::Exit);
    assert!(joined.is_err());

    let joined = join_left(l.clone(), r.clone(), MissingHandling::Continue)?;
    assert!(joined.contains_key("k1"));
    assert!(!joined.contains_key("k2"));

    // outer
    let joined = join_outer(l, r)?;
    assert!(joined.contains_key("k1"));
    assert!(joined.contains_key("k2"));
    Ok(())
}
