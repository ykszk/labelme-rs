use anyhow::{Context, Result};
use glob::glob;
use labelme_rs::indexmap::IndexSet;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use lmrs::cli::ValidateCmdArgs as CmdArgs;

struct OrderedWriter {
    pub buf: HashMap<usize, Option<String>>,
    current: usize,
}

impl OrderedWriter {
    fn new() -> Self {
        Self {
            buf: HashMap::new(),
            current: 0,
        }
    }

    fn write(&mut self, id: usize, s: String) {
        self.buf.insert(id, Some(s));
        if id == self.current {
            self.flush();
        }
    }

    fn skip(&mut self, id: usize) {
        self.buf.insert(id, None);
        if id == self.current {
            self.flush();
        }
    }

    fn flush(&mut self) {
        while let Some(s) = self.buf.remove(&self.current) {
            if let Some(s) = s {
                print!("{}", s);
            }
            self.current += 1;
        }
    }

    fn flush_all(&mut self) {
        let ordered_keys: Vec<_> = self.buf.keys().copied().collect();
        for id in ordered_keys {
            if let Some(Some(s)) = self.buf.remove(&id) {
                print!("{}", s);
            }
        }
    }
}

pub fn cmd(args: CmdArgs) -> Result<()> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(args.threads)
        .build_global()?;

    let verbosity = args.verbose;
    let mut rules = lmrs::load_rules(&args.rules)?;
    for filename in args.additional {
        let ar = lmrs::load_rules(&filename)?;
        rules.extend(ar);
    }
    let asts = lmrs::parse_rules(&rules)?;
    let indir = &args.input;
    if !indir.exists() {
        return Err(std::io::Error::from(std::io::ErrorKind::NotFound).into());
    }
    let checked_count = Arc::new(AtomicUsize::new(0));
    let valid_count = Arc::new(AtomicUsize::new(0));
    let file_list: Result<Vec<_>, _> = glob(
        indir
            .join("**/*.json")
            .to_str()
            .context("Failed to get glob string")?,
    )
    .expect("Failed to read glob pattern")
    .collect();
    let mut file_list = file_list?;
    file_list.sort();
    let file_id_list = file_list.into_iter().enumerate().collect::<Vec<_>>();
    let flag_set: IndexSet<String> = args.flag.into_iter().collect();
    let ignore_set: IndexSet<String> = args.ignore.into_iter().collect();
    let writer = Arc::new(Mutex::new(OrderedWriter::new()));
    file_id_list.into_par_iter().for_each(|(id_path, path)| {
        let check_result = lmrs::check_json_file(&rules, &asts, &path, &flag_set, &ignore_set);
        let disp_path = path.strip_prefix(&args.input).unwrap_or(path.as_path());
        match check_result {
            Ok(ret) =>
            {
                #[allow(clippy::collapsible_else_if)]
                if ret == lmrs::CheckResult::Passed {
                    checked_count.fetch_add(1, Ordering::SeqCst);
                    valid_count.fetch_add(1, Ordering::SeqCst);
                    writer.lock().unwrap().skip(id_path);
                } else {
                    if verbosity > 0 {
                        let mut writer = writer.lock().unwrap();
                        writer.write(id_path, format!("{:?}\n", disp_path));
                    } else {
                        writer.lock().unwrap().skip(id_path);
                    }
                }
            }
            Err(err) => {
                checked_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let mut writer = writer.lock().unwrap();
                writer.write(id_path, format!("{:?},{}\n", disp_path, err));
            }
        };
    });
    let mut writer = writer.lock().unwrap();
    writer.flush_all();
    if args.stats {
        println!(
            "{} / {} annotations are valid.",
            valid_count.load(Ordering::SeqCst),
            checked_count.load(Ordering::SeqCst)
        );
    }
    Ok(())
}
