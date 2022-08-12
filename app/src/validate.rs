use clap::Parser;
use glob::glob;
use labelme_rs::indexmap::{IndexMap, IndexSet};
use labelme_rs::serde_json;
use labelme_rs::{FlagSet, LabelMeData, Point};
use std::error;
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum CheckError {
    FileNotFound,
    InvalidJson,
    EvaluatedFalse(String, isize, isize),
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckError::EvaluatedFalse(cond, c1, c2) => {
                write!(f, "Unsatisfied rule \"{}\": {} vs. {}", cond, c1, c2)
            }
            _ => write!(f, "{:?}", self),
        }
    }
}

impl error::Error for CheckError {}

#[derive(PartialEq, Debug)]
enum CheckResult {
    Skipped,
    Passed,
}

fn check_json(
    rules: &[String],
    asts: &[dsl::Expr],
    json_filename: &Path,
    flags: &FlagSet,
) -> Result<CheckResult, CheckError> {
    let json_data: LabelMeData = serde_json::from_reader(BufReader::new(
        File::open(json_filename).or(Err(CheckError::FileNotFound))?,
    ))
    .map_err(|_| CheckError::InvalidJson)?;
    let json_flags =
        FlagSet::from_iter(json_data.flags.into_iter().filter_map(
            |(k, v)| {
                if v {
                    Some(k)
                } else {
                    None
                }
            },
        ));
    if !flags.is_empty() && json_flags.intersection(flags).count() == 0 {
        return Ok(CheckResult::Skipped);
    }
    let mut point_map: IndexMap<String, Vec<Point>> = IndexMap::new();
    for shape in json_data.shapes.into_iter() {
        let vec: &mut Vec<Point> = point_map.entry(shape.label).or_insert_with(Vec::new);
        vec.push(shape.points[0]);
    }
    let vars: Vec<_> = point_map
        .iter()
        .map(|(k, v)| (k, v.len() as isize))
        .collect();
    for (i, ast) in asts.iter().enumerate() {
        dsl::eval(ast, &vars)
            .map_err(|(a, b)| CheckError::EvaluatedFalse(rules[i].clone(), a, b))?;
    }
    Ok(CheckResult::Passed)
}

#[test]
fn test_check_json() {
    let rule = "TL > 0".to_string();
    let rules = vec![rule.clone()];
    let asts = dsl::parse_rules(&rules).unwrap();
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.push("tests/img1.json");
    assert_eq!(
        check_json(&rules, &asts, &filename, &FlagSet::new()).unwrap(),
        CheckResult::Passed,
        "Valid rule"
    );

    let rule = "X == 0".to_string();
    let rules = vec![rule.clone()];
    let asts = dsl::parse_rules(&rules).unwrap();
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.push("tests/img1.json");
    assert_eq!(
        check_json(&rules, &asts, &filename, &FlagSet::new()).unwrap(),
        CheckResult::Passed,
        "Non-existent variable"
    );

    let rule = "TL == 0".to_string();
    let rules = vec![rule.clone()];
    let asts = dsl::parse_rules(&rules).unwrap();
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.push("tests/img1.json");
    assert_eq!(
        check_json(&rules, &asts, &filename, &FlagSet::new()).unwrap_err(),
        CheckError::EvaluatedFalse(rule, 1, 0),
        "False rule"
    );

    let rule = "TL == TR".to_string();
    let rules = vec![rule.clone()];
    let asts = dsl::parse_rules(&rules).unwrap();
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.push("tests/test.json");
    assert_eq!(
        check_json(&rules, &asts, &filename, &FlagSet::new()).unwrap(),
        CheckResult::Passed,
        "Valid rule"
    );
    assert_eq!(
        check_json(
            &rules,
            &asts,
            &filename,
            &FlagSet::from_iter(vec!["f1".into()])
        )
        .unwrap(),
        CheckResult::Passed,
        "Test for a true flag"
    );
    assert_eq!(
        check_json(
            &rules,
            &asts,
            &filename,
            &FlagSet::from_iter(vec!["f2".into()])
        )
        .unwrap(),
        CheckResult::Skipped,
        "Test for a false flag"
    );
    assert_eq!(
        check_json(
            &rules,
            &asts,
            &filename,
            &FlagSet::from_iter(vec!["fx".into()])
        )
        .unwrap(),
        CheckResult::Skipped,
        "Test for a non-existent flag"
    );

    let rule = "TL == BL + 1".to_string();
    let rules = vec![rule.clone()];
    let asts = dsl::parse_rules(&rules).unwrap();
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.push("tests/test.json");
    assert_eq!(
        check_json(&rules, &asts, &filename, &FlagSet::new()).unwrap_err(),
        CheckError::EvaluatedFalse(rule, 1, 2),
        "False rule"
    );
}

/// Validate labelme annotations
#[derive(Parser, Debug)]
#[clap(name=env!("CARGO_BIN_NAME"), author, version, about, long_about = None)]
struct Args {
    /// Rules
    rules: PathBuf,
    /// Input directory
    input: PathBuf,
    /// Check only json files containing given flag(s)
    #[clap(short, long)]
    flag: Vec<String>,
    /// Report stats at the end
    #[clap(short, long)]
    stats: bool,
    /// Set verbosity
    #[clap(short, long, parse(from_occurrences))]
    verbose: usize,
    /// Set the number of threads
    #[clap(short, long, default_value_t = 0)]
    threads: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let verbosity = args.verbose;
    let rules = dsl::load_rules(&args.rules)?;
    let asts = dsl::parse_rules(&rules)?;
    let indir = &args.input;
    if !indir.exists() {
        return Err(std::io::Error::from(std::io::ErrorKind::NotFound).into());
    }
    let mut n_threads = args.threads;
    if n_threads == 0 {
        n_threads = num_cpus::get_physical();
    }
    let checked_count = Arc::new(AtomicUsize::new(0));
    let valid_count = Arc::new(AtomicUsize::new(0));
    let file_list: Vec<_> = glob(indir.join("**/*.json").to_str().unwrap())
        .expect("Failed to read glob pattern")
        .collect();
    let file_list = Arc::new(file_list);
    let flag_set = IndexSet::from_iter(args.flag.into_iter());
    std::thread::scope(|scope| {
        let mut handles = vec![];
        for thread_i in 0..n_threads {
            let checked_count = Arc::clone(&checked_count);
            let valid_count = Arc::clone(&valid_count);
            let file_list = &file_list;
            let indir = &args.input;
            let flag_set = &flag_set;
            let rules = &rules;
            let asts = &asts;
            let handle = scope.spawn(move || {
                for i in (thread_i..file_list.len()).step_by(n_threads) {
                    let entry = &file_list[i];
                    match entry {
                        Ok(path) => {
                            let check_result = check_json(rules, asts, path, flag_set);
                            let disp_path = path.strip_prefix(&indir).unwrap_or(path.as_path());
                            match check_result {
                                Ok(ret) => {
                                    if ret == CheckResult::Passed {
                                        checked_count.fetch_add(1, Ordering::SeqCst);
                                        valid_count.fetch_add(1, Ordering::SeqCst);
                                    }
                                    if verbosity > 0 {
                                        println!("{},", disp_path.to_str().unwrap());
                                    }
                                }
                                Err(err) => {
                                    checked_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                    println!("{},{}", disp_path.to_str().unwrap(), err);
                                }
                            };
                        }
                        Err(e) => println!("{:?}", e),
                    }
                }
            });
            handles.push(handle);
        }
        for handle in handles {
            handle.join().unwrap();
        }
    });
    if args.stats {
        println!(
            "{} / {} annotations are valid.",
            valid_count.load(Ordering::SeqCst),
            checked_count.load(Ordering::SeqCst)
        )
    }
    Ok(())
}
