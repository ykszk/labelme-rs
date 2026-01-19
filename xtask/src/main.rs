use std::path::PathBuf;

use clap::{CommandFactory, Parser};

#[derive(Parser)]
struct ManArgs {
    /// Output directory. e.g. `$MANPATH/man1`
    output: PathBuf,
}

fn main() {
    let args = ManArgs::parse();

    let outdir = args.output;
    if outdir.is_file() {
        panic!("output must be a directory");
    }
    if !outdir.exists() {
        println!("Creating output directory: {:?}", outdir);
        std::fs::create_dir_all(&outdir).unwrap();
    }

    let cmd = lmrs::cli::Cli::command();
    let cmd_name: String = cmd.get_name().into();
    let version: String = cmd.get_version().unwrap().into();
    let ext = ".1";
    cmd.get_subcommands().cloned().for_each(|subcommand| {
        let subcmd_name = format!("{} {}", cmd_name, subcommand.get_name().replace(' ', "-"));
        let named = subcommand.name(&subcmd_name).version(&version);
        let man = clap_mangen::Man::new(named);
        let outname = outdir.join(subcmd_name.replace(' ', "-") + ext);
        let mut file = std::fs::File::create(outname).unwrap();
        man.render(&mut file).unwrap();
    });
    let outname = outdir.join(cmd.get_name().to_owned() + ext);
    let mut file = std::fs::File::create(outname).unwrap();
    clap_mangen::Man::new(cmd).render(&mut file).unwrap();
}
