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

type JzonObject = jzon::JsonValue;

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

fn load_ndjson<T: ParseStr>(input: &Path) -> Result<Vec<T>, Box<dyn std::error::Error>> {
    let reader: Box<dyn BufRead> = if input.as_os_str() == "-" {
        Box::new(BufReader::new(std::io::stdin()))
    } else {
        Box::new(BufReader::new(File::open(input).unwrap()))
    };
    let ndjson: Result<Vec<T>, _> = reader
        .lines()
        .map(|line| line.map_err(|e| e.into()).and_then(|l| T::parse_str(&l)))
        .collect();
    ndjson
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
    let ndjsons: Result<Vec<Vec<JzonObject>>, _> =
        input_set.iter().map(|input| load_ndjson(input)).collect();
    debug!("Create map");
    let mut json_map: HashMap<String, Vec<JzonObject>> =
        HashMap::with_capacity(ndjsons.iter().map(|e| e.len()).min().unwrap());
    for ndjson in ndjsons?.into_iter() {
        for json in ndjson {
            let value = json
                .get(&args.key)
                .unwrap_or_else(|| panic!("Key {} not found", args.key));
            match value {
                jzon::JsonValue::Short(s) => {
                    let v = json_map.entry(s.to_string()).or_insert_with(Vec::new);
                    v.push(json);
                }
                jzon::JsonValue::String(s) => {
                    let v = json_map.entry(s.to_string()).or_insert_with(Vec::new);
                    v.push(json);
                }
                _ => panic!("Key value is not a string: {}", value),
            };
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
        let joined: Option<jzon::JsonValue> = jsons.into_iter().reduce(|mut a, mut b| {
            b.remove(&args.key);
            dsl::merge(&mut a, b);
            a
        });
        let line = joined.unwrap().to_string();
        println!("{line}");
    }
    debug!("Done");
    Ok(())
}
