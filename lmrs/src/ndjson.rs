use anyhow::{bail, ensure, Context, Result};
use labelme_rs::serde_json;
use lmrs::cli::{NdjsonCmdArgs as CmdArgs, ParentHandling};
use serde_json::{Map, Value};
use std::io::{BufRead, BufReader};
use std::path::{Component, Path, PathBuf};

#[cfg(not(target_os = "windows"))]
extern crate libc;

/// Parse a jq-like field path such as `.content.imagePath` into keys
fn parse_field(field: &str) -> Result<Vec<String>> {
    let keys: Vec<String> = field
        .strip_prefix('.')
        .unwrap_or(field)
        .split('.')
        .map(String::from)
        .collect();
    ensure!(
        keys.iter().all(|k| !k.is_empty()),
        "Invalid field {:?}. Specify a field like `.content.imagePath`",
        field
    );
    Ok(keys)
}

/// Make `path` absolute and remove `.` and `..` components without touching the filesystem
fn normalize_path(path: &Path) -> Result<PathBuf> {
    let path = std::path::absolute(path).with_context(|| format!("Resolving {:?}", path))?;
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            c => normalized.push(c),
        }
    }
    Ok(normalized)
}

/// Resolve the path at `keys` in `data` relative to `base_dir`
fn normalize_field(data: &mut Map<String, Value>, keys: &[String], base_dir: &Path) -> Result<()> {
    let (last, parents) = keys.split_last().expect("keys are not empty");
    let mut obj = data;
    for key in parents {
        match obj.get_mut(key) {
            Some(Value::Object(o)) => obj = o,
            _ => return Ok(()),
        }
    }
    match obj.get_mut(last) {
        None | Some(Value::Null) => Ok(()),
        Some(Value::String(s)) => {
            #[cfg(not(target_os = "windows"))]
            let path = PathBuf::from(s.replace('\\', "/"));
            #[cfg(target_os = "windows")]
            let path = PathBuf::from(s.as_str());
            *s = normalize_path(&base_dir.join(path))?
                .to_string_lossy()
                .into_owned();
            Ok(())
        }
        Some(v) => bail!("Field {:?} is not a string: {}", keys.join("."), v),
    }
}

fn parent_dir(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn print_ndjson(
    input: PathBuf,
    key: &str,
    parent_handling: ParentHandling,
    fields: &[Vec<String>],
) -> Result<()> {
    let json_str =
        std::fs::read_to_string(&input).with_context(|| format!("Reading {:?}", input))?;
    let content: Map<String, Value> = serde_json::from_str(&json_str)?;
    let mut json_data: Map<String, Value> = Map::default();
    json_data.insert("content".into(), content.into());
    let filename: String = match parent_handling {
        ParentHandling::Keep => input.to_string_lossy().into(),
        ParentHandling::Absolute => input.canonicalize()?.to_string_lossy().into(),
        ParentHandling::Remove => input
            .file_name()
            .with_context(|| format!("Filename is missing in {:?}", input))?
            .to_string_lossy()
            .into(),
    };
    json_data.insert(key.to_string(), filename.into());
    for keys in fields {
        normalize_field(&mut json_data, keys, parent_dir(&input))
            .with_context(|| format!("Normalizing {:?}", input))?;
    }
    let line = serde_json::to_string(&json_data)?;
    println!("{line}");
    Ok(())
}

pub fn cmd(args: CmdArgs) -> Result<()> {
    let fields = args
        .normalize
        .iter()
        .map(|f| parse_field(f))
        .collect::<Result<Vec<_>>>()?;
    for input in args.input {
        ensure!(input.exists(), "Input {:?} does not exist", input);
        let mut options = glob::MatchOptions::new();
        options.require_literal_leading_dot = !args.all;
        if input.is_dir() {
            let entries = glob::glob_with(
                input
                    .join(args.glob.as_str())
                    .to_str()
                    .context("Failed to obtain glob string")?,
                options,
            )
            .expect("Failed to read glob pattern");
            for entry in entries {
                let input = entry?;
                print_ndjson(input, &args.filename, args.parent, &fields)?;
            }
        } else if input
            .extension()
            .map(|ext| ext == "ndjson" || ext == "jsonl")
            .unwrap_or(false)
        {
            let ndjson_dir = parent_dir(&input);
            let file = BufReader::new(std::fs::File::open(&input)?);
            for line in file.lines() {
                let line = line?;
                if fields.is_empty() {
                    println!("{line}");
                    continue;
                }
                let mut data: Map<String, Value> = serde_json::from_str(&line)
                    .with_context(|| format!("Parsing a line in {:?}", input))?;
                let base_dir = match data.get(&args.filename) {
                    Some(Value::String(filename)) => {
                        ndjson_dir.join(parent_dir(Path::new(filename)))
                    }
                    _ => ndjson_dir.to_path_buf(),
                };
                for keys in &fields {
                    normalize_field(&mut data, keys, &base_dir)
                        .with_context(|| format!("Normalizing a line in {:?}", input))?;
                }
                println!("{}", serde_json::to_string(&data)?);
            }
        } else if input.extension().map(|ext| ext == "json").unwrap_or(false) {
            print_ndjson(input, &args.filename, args.parent, &fields)?;
        } else {
            bail!("{:?} is not a directory, json, or ndjson/jsonl", input);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_field() {
        assert_eq!(
            parse_field(".content.imagePath").unwrap(),
            vec!["content", "imagePath"]
        );
        assert_eq!(parse_field("imagePath").unwrap(), vec!["imagePath"]);
        assert!(parse_field(".").is_err());
        assert!(parse_field(".a..b").is_err());
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_normalize_field() {
        let mut data: Map<String, Value> = serde_json::from_str(
            r#"{"content": {"imagePath": "..\\image/./a.jpg", "n": 1}, "abs": "/x/../y.jpg"}"#,
        )
        .unwrap();
        let base = Path::new("/path/json");
        normalize_field(&mut data, &parse_field(".content.imagePath").unwrap(), base).unwrap();
        normalize_field(&mut data, &parse_field(".abs").unwrap(), base).unwrap();
        normalize_field(&mut data, &parse_field(".missing.key").unwrap(), base).unwrap();
        assert_eq!(data["content"]["imagePath"], "/path/image/a.jpg");
        assert_eq!(data["abs"], "/y.jpg");
        assert!(normalize_field(&mut data, &parse_field(".content.n").unwrap(), base).is_err());
    }
}
