use labelme_rs::indexmap::IndexSet;
use std::fs::File;
use std::io::{BufRead, BufReader};

use lmrs::cli::FilterCmdArgs as CmdArgs;

pub fn cmd(args: CmdArgs) -> Result<(), Box<dyn std::error::Error>> {
    let mut rules: Vec<String> = Vec::new();
    for filename in args.rules {
        let ar = lmrs::load_rules(&filename)?;
        rules.extend(ar);
    }
    if rules.is_empty() {
        panic!("No rule is found.");
    }
    let asts = lmrs::parse_rules(&rules)?;
    let flag_set: IndexSet<String> = args.flag.into_iter().collect();
    let ignore_set: IndexSet<String> = args.ignore.into_iter().collect();
    let reader: Box<dyn BufRead> = if args.input.as_os_str() == "-" {
        Box::new(BufReader::new(std::io::stdin()))
    } else {
        Box::new(BufReader::new(File::open(&args.input).unwrap()))
    };
    for line in reader.lines() {
        let line = line?;
        let check_result = lmrs::check_jsons(&rules, &asts, &line, &flag_set, &ignore_set);
        if args.invert {
            if check_result.is_err() {
                println!("{}", line);
            }
        } else if let Ok(ret) = check_result {
            if ret == lmrs::CheckResult::Passed {
                println!("{}", line);
            }
        }
    }
    Ok(())
}
