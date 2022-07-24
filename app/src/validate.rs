use clap::Parser;
use dsl::Parser as DslParser;
use glob::glob;
use labelme_rs::indexmap::{IndexMap, IndexSet};
use labelme_rs::serde_json;
use labelme_rs::{FlagSet, LabelMeData, Point};
use std::error;
use std::fmt;
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::{
    fs::File,
    io::{self, BufReader},
    path::Path,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum CheckError {
    FileNotFound,
    InvalidJson,
    CountMismatchError(String, String, usize, usize),
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckError::CountMismatchError(msg1, msg2, c1, c2) => write!(
                f,
                "Count mismatch between {} and {}: {} vs. {}",
                msg1, msg2, c1, c2
            ),
            _ => write!(f, "{:?}", self),
        }
    }
}

impl error::Error for CheckError {}

fn check_json(
    asts: &Vec<dsl::Expr>,
    json_filename: &Path,
    flags: &FlagSet,
) -> Result<bool, CheckError> {
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
        return Ok(true);
    }
    let mut point_map: IndexMap<String, Vec<Point>> = IndexMap::new();
    for shape in json_data.shapes.into_iter() {
        let vec: &mut Vec<Point> = point_map.entry(shape.label).or_insert_with(Vec::new);
        vec.push(shape.points[0]);
    }
    for required_label in ["TL", "TR", "BL", "BR"] {
        point_map
            .entry(required_label.to_string())
            .or_insert_with(Vec::new);
    }
    let vars: Vec<_> = point_map
        .iter()
        .map(|(k, v)| (k, v.len() as isize))
        .collect();
    for ast in asts {
        dsl::eval(ast, &vars).unwrap();
    }
    Ok(false)
}

#[test]
fn test_check_json() {
    let ast = dsl::parser().parse("TL > 0").unwrap();
    let asts = vec!(ast);
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.push("tests/img1.json");
    let err = check_json(&asts, &filename, &FlagSet::new()).unwrap();
    // assert_eq!(err, test_pair.1);

    // assert!(check_json(Path::new("tests/good.json"), &FlagSet::new()).is_ok());
    // let error_test_pairs = [
    //     (
    //         "tests/count_error_tl_tr.json",
    //         CheckError::CountMismatchError("TL".into(), "TR".into(), 17, 18),
    //     ),
    //     (
    //         "tests/count_error_bl_br.json",
    //         CheckError::CountMismatchError("BL".into(), "BR".into(), 17, 16),
    //     ),
    //     (
    //         "tests/count_error_tl_bl.json",
    //         CheckError::CountMismatchError("TL".into(), "(BL+1)".into(), 18, 17),
    //     ),
    //     ("tests/foo.json", CheckError::FileNotFound),
    //     ("tests/no_shapes.json", CheckError::InvalidJson),
    // ];
    // for test_pair in error_test_pairs {
    //     let err = check_json(Path::new(test_pair.0), &FlagSet::new()).unwrap_err();
    //     assert_eq!(err, test_pair.1);
    // }
    // let mut flags = FlagSet::new();
    // flags.insert("flag1".into());
    // assert!(check_json(Path::new("tests/count_error_tl_tr.json"), &flags).is_ok());
}

/// Validate labelme annotations
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
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
    let rules: Vec<String> = BufReader::new(File::open(args.rules)?)
        .lines()
        .filter_map(|l| l.ok())
        .collect();
    let asts: Vec<_> = rules
        .into_iter()
        .map(|r| dsl::parser().parse(r).unwrap())
        .collect();
    let indir = &args.input;
    if !indir.exists() {
        return Err(io::Error::from(io::ErrorKind::NotFound).into());
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
    {
        let mut handles = vec![];
        let flag_set = IndexSet::from_iter(args.flag.into_iter());
        for thread_i in 0..n_threads {
            let checked_count = Arc::clone(&checked_count);
            let valid_count = Arc::clone(&valid_count);
            let file_list = Arc::clone(&file_list);
            let indir = args.input.clone();
            let flag_set = flag_set.clone();
            let asts = asts.clone();
            let handle = std::thread::spawn(move || {
                for i in (thread_i..file_list.len()).step_by(n_threads) {
                    let entry = &file_list[i];
                    match entry {
                        Ok(path) => {
                            let ret = check_json(&asts, path, &flag_set);
                            let disp_path = path.strip_prefix(&indir).unwrap();
                            match ret {
                                Ok(skipped) => {
                                    if !skipped {
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
    }
    if args.stats {
        println!(
            "{} / {} annotations are valid.",
            valid_count.load(Ordering::SeqCst),
            checked_count.load(Ordering::SeqCst)
        )
    }
    Ok(())
}
