use clap::Args;
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
    EvaluatedFalse(String, (isize, isize)),
    EvaluatedMultipleFalses(Vec<(String, (isize, isize))>),
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckError::EvaluatedFalse(cond, (c1, c2)) => {
                write!(f, "Unsatisfied rule; \"{}\": {} vs. {}", cond, c1, c2)
            }
            CheckError::EvaluatedMultipleFalses(errors) => {
                write!(f, "Unsatisfied rules;")?;
                let msg = errors
                    .iter()
                    .map(|(cond, (c1, c2))| format!(" \"{}\": {} vs. {}", cond, c1, c2))
                    .collect::<Vec<_>>()
                    .join(", ");
                f.write_str(&msg)
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
    ignores: &FlagSet,
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
    if (!flags.is_empty() && json_flags.intersection(flags).count() == 0)
        || json_flags.intersection(ignores).count() > 0
    {
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

    let mut errors: Vec<_> = asts
        .iter()
        .zip(rules.iter())
        .filter_map(|(ast, rule)| {
            let result = dsl::eval(ast, &vars);
            match result {
                Ok(_) => None,
                Err(vals) => Some((rule.clone(), vals)),
            }
        })
        .collect();
    if errors.is_empty() {
        Ok(CheckResult::Passed)
    } else if errors.len() == 1 {
        let (rule, vals) = errors.pop().unwrap();
        Err(CheckError::EvaluatedFalse(rule, vals))
    } else {
        Err(CheckError::EvaluatedMultipleFalses(errors))
    }
}

#[test]
fn test_check_json() {
    let rule = "TL > 0".to_string();
    let rules = vec![rule];
    let asts = dsl::parse_rules(&rules).unwrap();
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.push("tests/img1.json");
    assert_eq!(
        check_json(&rules, &asts, &filename, &FlagSet::new(), &FlagSet::new()).unwrap(),
        CheckResult::Passed,
        "Valid rule"
    );

    let rule = "X == 0".to_string();
    let rules = vec![rule];
    let asts = dsl::parse_rules(&rules).unwrap();
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.push("tests/img1.json");
    assert_eq!(
        check_json(&rules, &asts, &filename, &FlagSet::new(), &FlagSet::new()).unwrap(),
        CheckResult::Passed,
        "Non-existent variable"
    );

    let rule = "TL == 0".to_string();
    let rules = vec![rule.clone()];
    let asts = dsl::parse_rules(&rules).unwrap();
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.push("tests/img1.json");
    assert_eq!(
        check_json(&rules, &asts, &filename, &FlagSet::new(), &FlagSet::new()).unwrap_err(),
        CheckError::EvaluatedFalse(rule, (1, 0)),
        "False rule"
    );
    let (rule1, rule2) = ("TL == 0".to_string(), "TR == 1".to_string());
    let rules = vec![rule1.clone(), rule2.clone()];
    let asts = dsl::parse_rules(&rules).unwrap();
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let errors = vec![(rule1, (1, 0)), (rule2, (0, 1))];
    filename.push("tests/img1.json");
    assert_eq!(
        check_json(&rules, &asts, &filename, &FlagSet::new(), &FlagSet::new()).unwrap_err(),
        CheckError::EvaluatedMultipleFalses(errors),
        "False rule"
    );

    let rule = "TL == TR".to_string();
    let rules = vec![rule];
    let asts = dsl::parse_rules(&rules).unwrap();
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.push("tests/test.json");
    assert_eq!(
        check_json(&rules, &asts, &filename, &FlagSet::new(), &FlagSet::new()).unwrap(),
        CheckResult::Passed,
        "Valid rule"
    );
    assert_eq!(
        check_json(
            &rules,
            &asts,
            &filename,
            &FlagSet::from_iter(vec!["f1".into()]),
            &FlagSet::new()
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
            &FlagSet::from_iter(vec!["f2".into()]),
            &FlagSet::new()
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
            &FlagSet::new(),
            &FlagSet::from_iter(vec!["f1".into()])
        )
        .unwrap(),
        CheckResult::Skipped,
        "Test for ignoring flag"
    );
    assert_eq!(
        check_json(
            &rules,
            &asts,
            &filename,
            &FlagSet::from_iter(vec!["fx".into()]),
            &FlagSet::new()
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
        check_json(&rules, &asts, &filename, &FlagSet::new(), &FlagSet::new()).unwrap_err(),
        CheckError::EvaluatedFalse(rule, (1, 2)),
        "False rule"
    );
}

#[derive(Args, Debug)]
pub struct ValidateArgs {
    /// Rules
    rules: PathBuf,
    /// Input directory
    input: PathBuf,
    /// Check only json files containing given flag(s). Multiple flags are concatenated by OR.
    #[clap(short, long)]
    flag: Vec<String>,
    /// Ignore json files containing given flag(s). Multiple flags are concatenated by OR.
    #[clap(short, long)]
    ignore: Vec<String>,
    /// Additional rules
    #[clap(short, long)]
    additional: Vec<PathBuf>,
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

pub fn cmd_validate(args: ValidateArgs) -> Result<(), Box<dyn std::error::Error>> {
    let verbosity = args.verbose;
    let mut rules = dsl::load_rules(&args.rules)?;
    for filename in args.additional {
        let ar = dsl::load_rules(&filename)?;
        rules.extend(ar);
    }
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
    let ignore_set = IndexSet::from_iter(args.ignore.into_iter());
    std::thread::scope(|scope| {
        let mut handles = vec![];
        for thread_i in 0..n_threads {
            let checked_count = Arc::clone(&checked_count);
            let valid_count = Arc::clone(&valid_count);
            let file_list = &file_list;
            let indir = &args.input;
            let flag_set = &flag_set;
            let ignore_set = &ignore_set;
            let rules = &rules;
            let asts = &asts;
            let handle = scope.spawn(move || {
                for i in (thread_i..file_list.len()).step_by(n_threads) {
                    let entry = &file_list[i];
                    match entry {
                        Ok(path) => {
                            let check_result = check_json(rules, asts, path, flag_set, ignore_set);
                            let disp_path = path.strip_prefix(indir).unwrap_or(path.as_path());
                            match check_result {
                                Ok(ret) => {
                                    if ret == CheckResult::Passed {
                                        checked_count.fetch_add(1, Ordering::SeqCst);
                                        valid_count.fetch_add(1, Ordering::SeqCst);
                                    }
                                    if verbosity > 0 && ret != CheckResult::Skipped {
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
