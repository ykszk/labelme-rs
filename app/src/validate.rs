use clap::Args;
use glob::glob;
use labelme_rs::indexmap::IndexSet;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[derive(Args, Debug)]
pub struct CmdArgs {
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
    #[clap(short, long, action = clap::ArgAction::Count)]
    verbose: usize,
    /// Set the number of threads
    #[clap(short, long, default_value_t = 0)]
    threads: usize,
}

pub fn cmd(args: CmdArgs) -> Result<(), Box<dyn std::error::Error>> {
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
    let flag_set: IndexSet<String> = args.flag.into_iter().collect();
    let ignore_set: IndexSet<String> = args.ignore.into_iter().collect();
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
                            let check_result =
                                dsl::check_json_file(rules, asts, path, flag_set, ignore_set);
                            let disp_path = path.strip_prefix(indir).unwrap_or(path.as_path());
                            match check_result {
                                Ok(ret) => {
                                    if ret == dsl::CheckResult::Passed {
                                        checked_count.fetch_add(1, Ordering::SeqCst);
                                        valid_count.fetch_add(1, Ordering::SeqCst);
                                    }
                                    if verbosity > 0 && ret != dsl::CheckResult::Skipped {
                                        println!("{},", disp_path.to_str().unwrap());
                                    }
                                }
                                Err(err) => {
                                    checked_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                    println!("{},{}", disp_path.to_str().unwrap(), err);
                                }
                            };
                        }
                        Err(e) => println!("{e:?}"),
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
        );
    }
    Ok(())
}
