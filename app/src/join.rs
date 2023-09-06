use clap::{Args, ValueEnum};
use labelme_rs::indexmap::IndexSet;
use labelme_rs::serde_json;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug, Args)]
pub struct CmdArgs {
    /// Input ndjson. Specify "-" to use stdin
    #[clap(required=true, num_args=2..)]
    input: Vec<PathBuf>,
    /// Key to join based on
    #[clap(long, default_value = "filename")]
    key: String,
    /// Join mode
    #[clap(long, default_value = "outer")]
    mode: JoinMode,
}

#[derive(ValueEnum, Debug, Clone)]
enum JoinMode {
    /// Inner
    Inner,
    /// Left inner
    Left,
    /// Right inner
    Right,
    /// Outer
    Outer,
}

type JsonObject = serde_json::Map<String, serde_json::Value>;

fn load_ndjson(input: &Path) -> Result<Vec<JsonObject>, Box<dyn std::error::Error>> {
    let reader: Box<dyn BufRead> = if input.as_os_str() == "-" {
        Box::new(BufReader::new(std::io::stdin()))
    } else {
        Box::new(BufReader::new(File::open(input).unwrap()))
    };
    let ndjson: Result<Vec<JsonObject>, _> = reader
        .lines()
        .map(|line| {
            line.map_err(|e| e.into())
                .and_then(|l| serde_json::from_str(&l).map_err(|e| e.into()))
        })
        .collect();
    ndjson
}

fn join(left: &mut JsonObject, right: JsonObject) {
    for (key, r_value) in right.into_iter() {
        let entry = left.entry(key);
        match entry {
            // maybe later: could not use ok_modify().or_insert()
            serde_json::map::Entry::Vacant(entry) => {
                entry.insert(r_value);
            }
            serde_json::map::Entry::Occupied(mut l_value) => match l_value.get_mut() {
                serde_json::Value::Array(l) => {
                    if let serde_json::Value::Array(r) = r_value {
                        l.extend(r);
                    } else {
                        panic!("Inconsistent types found at ?. Array vs ?");
                    };
                }
                serde_json::Value::Object(l) => {
                    if let serde_json::Value::Object(r) = r_value {
                        join(l, r);
                    } else {
                        panic!("Inconsistent types found at ?. Array vs ?");
                    };
                }
                l => panic!(
                    "Trying to join to invalid type other than array or map: {:?} vs {:?}",
                    l, r_value
                ),
            },
        };
    }
}

pub fn cmd(args: CmdArgs) -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
    let input_set: IndexSet<PathBuf> = IndexSet::from_iter(args.input.into_iter());
    if input_set.len() <= 1 {
        return Err("Need more than one input".into());
    }
    debug!("Read ndjsons");
    let ndjsons: Result<Vec<Vec<JsonObject>>, _> =
        input_set.iter().map(|input| load_ndjson(input)).collect();
    debug!("Create map");
    let mut json_map: HashMap<String, Vec<JsonObject>> =
        HashMap::with_capacity(ndjsons.iter().map(|e| e.len()).min().unwrap());
    for ndjson in ndjsons?.into_iter() {
        for json in ndjson {
            let value = json
                .get(&args.key)
                .unwrap_or_else(|| panic!("Key {} not found", args.key));
            if let serde_json::Value::String(value) = value {
                let v = json_map.entry(value.clone()).or_insert_with(Vec::new);
                v.push(json);
            } else {
                panic!("Key value is not a string: {}", value);
            }
        }
    }
    debug!("Join");
    for (_, jsons) in json_map.into_iter() {
        match args.mode {
            JoinMode::Inner => {
                if jsons.len() != input_set.len() {
                    continue;
                }
            }
            JoinMode::Left => panic!("`--mode left` is not implemented"),
            JoinMode::Right => panic!("`--mode right` is not implemented"),
            JoinMode::Outer => {}
        }
        let joined = jsons.into_iter().reduce(|mut a, mut b| {
            b.remove(&args.key);
            join(&mut a, b);
            a
        });
        let line = serde_json::to_string(&joined)?;
        println!("{line}");
    }
    debug!("Done");
    Ok(())
}
