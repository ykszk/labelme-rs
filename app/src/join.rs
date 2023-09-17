use labelme_rs::indexmap::{IndexMap, IndexSet};
use labelme_rs::serde_json;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

type JzonObject = jzon::JsonValue;
use lmrs::cli::JoinCmdArgs as CmdArgs;
use lmrs::cli::JoinMode;

trait ParseStr: Sized {
    fn parse_str(s: &str) -> Result<Self, Box<dyn std::error::Error>>;
    fn to_line(self) -> Result<String, Box<dyn std::error::Error>>;
}

type SerdeJzonObject = serde_json::Map<String, serde_json::Value>;
impl ParseStr for SerdeJzonObject {
    fn parse_str(s: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let o = serde_json::from_str(s);
        o.map_err(|e| e.into())
    }
    fn to_line(self) -> Result<String, Box<dyn std::error::Error>> {
        serde_json::to_string(&self).map_err(|e| e.into())
    }
}

impl ParseStr for JzonObject {
    fn parse_str(s: &str) -> Result<Self, Box<dyn std::error::Error>> {
        jzon::parse(s).map_err(|e| e.into())
    }
    fn to_line(self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(self.to_string())
    }
}

fn load_ndjson(
    input: &Path,
    key: &str,
) -> Result<IndexMap<String, JzonObject>, Box<dyn std::error::Error>> {
    let reader: Box<dyn BufRead> = if input.as_os_str() == "-" {
        Box::new(BufReader::new(std::io::stdin()))
    } else {
        Box::new(BufReader::new(File::open(input).unwrap()))
    };
    let ndjson: Result<IndexMap<String, JzonObject>, Box<dyn std::error::Error>> = reader
        .lines()
        .map(|line| {
            line.map_err(|e| e.into())
                .and_then(|l| JzonObject::parse_str(&l))
                .and_then(|obj| match obj.get(key) {
                    Some(value) => {
                        if let Some(s) = value.as_str() {
                            Ok((s.to_string(), obj))
                        } else {
                            panic!("Value for the key is not a string");
                        }
                    }
                    None => {
                        panic!("Key {} not found", key)
                    }
                })
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
) -> IndexMap<String, JzonObject> {
    let mut right = right;
    left.into_iter()
        .filter_map(|(key, mut left_obj)| {
            right.remove(&key).map(|right_obj| {
                lmrs::merge(&mut left_obj, right_obj);
                (key, left_obj)
            })
        })
        .collect()
}

fn join_left(
    left: IndexMap<String, JzonObject>,
    right: IndexMap<String, JzonObject>,
) -> IndexMap<String, JzonObject> {
    let mut left = left;
    for (key, right_obj) in right.into_iter() {
        left.entry(key).and_modify(|left_obj| {
            lmrs::merge(left_obj, right_obj);
        });
    }
    left
}

fn join_outer(
    left: IndexMap<String, JzonObject>,
    right: IndexMap<String, JzonObject>,
) -> IndexMap<String, JzonObject> {
    let mut left = left;
    for (key, right_obj) in right.into_iter() {
        let entry = left.entry(key);
        match entry {
            labelme_rs::indexmap::map::Entry::Occupied(mut left_obj) => {
                lmrs::merge(left_obj.get_mut(), right_obj)
            }
            labelme_rs::indexmap::map::Entry::Vacant(entry) => {
                entry.insert(right_obj);
            }
        }
    }
    left
}

pub fn cmd(args: CmdArgs) -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
    let input_set: IndexSet<PathBuf> = IndexSet::from_iter(args.input.into_iter());
    if input_set.len() <= 1 {
        return Err("Need more than one input".into());
    }
    debug!("Read and join ndjsons");
    let joined: Result<IndexMap<String, JzonObject>, _> = input_set
        .iter()
        .map(|input| load_ndjson(input, &args.key))
        .reduce(|l, r| {
            l.and_then(|l| {
                r.map(|r| match args.mode {
                    JoinMode::Inner => join_inner(l, r),
                    JoinMode::Left => join_left(l, r),
                    JoinMode::Outer => join_outer(l, r),
                })
            })
        })
        .unwrap();
    debug!("Print result");
    for (key, mut obj) in joined? {
        obj.insert(&args.key, key).unwrap();
        let line = obj.to_string();
        println!("{}", line);
    }
    debug!("Done");
    Ok(())
}

#[test]
fn test_join() {
    let l: IndexMap<String, JzonObject> =
        IndexMap::from([("k1".into(), jzon::parse("{}").unwrap())]);
    let r: IndexMap<String, JzonObject> =
        IndexMap::from([("k2".into(), jzon::parse("{}").unwrap())]);

    let joined = join_inner(l.clone(), r.clone());
    assert!(!joined.contains_key("k1"));
    assert!(!joined.contains_key("k2"));

    let joined = join_left(l.clone(), r.clone());
    assert!(joined.contains_key("k1"));
    assert!(!joined.contains_key("k2"));

    let joined = join_outer(l, r);
    assert!(joined.contains_key("k1"));
    assert!(joined.contains_key("k2"));
}
