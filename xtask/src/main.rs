use std::path::PathBuf;

use clap::{Args, Command, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Generator, Shell};

#[derive(Parser)]
struct Tasks {
    #[clap(subcommand)]
    task: Task,
}

#[derive(Subcommand)]
enum Task {
    /// Save man pages to a directory
    Man(ManArgs),
    /// Print shell completion
    Complete(CompleteArgs),
}

#[derive(Debug, Args)]
struct ManArgs {
    /// Output directory. e.g. `$MANPATH/man1`
    output: PathBuf,
}

#[derive(Debug, Args)]
struct CompleteArgs {
    /// Shell to generate completion for
    generator: Shell,
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut std::io::stdout());
}

fn main() {
    let task = Tasks::parse();

    match task.task {
        Task::Man(args) => {
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
                let subcmd_name =
                    format!("{} {}", cmd_name, subcommand.get_name().replace(' ', "-"));
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
        Task::Complete(args) => {
            let mut cmd = lmrs::cli::Cli::command();
            print_completions(args.generator, &mut cmd);
        }
    }
}
