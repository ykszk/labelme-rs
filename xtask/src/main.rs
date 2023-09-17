use std::env;
use std::path::PathBuf;

use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate_to, shells};

#[derive(Parser)]
struct Tasks {
    #[clap(subcommand)]
    task: Task,
}

#[derive(Subcommand)]
enum Task {
    /// Print man
    Man {},
    /// Generate completion files. Dry run by default. Add `--install` option to perform actual installation
    Complete(CompleteArgs),
}

#[derive(Debug, Args)]
struct CompleteArgs {
    /// Shell
    shell: Shell,
    /// Override installation installation directory
    output: Option<PathBuf>,
    /// Binary name
    #[clap(default_value = "lmrs")]
    name: String,
    /// Execute installation
    #[clap(long, action)]
    install: bool,
}

#[derive(ValueEnum, Debug, Copy, Clone)]
enum Shell {
    Bash,
    Fish,
    Zsh,
}

fn main() {
    let task = Tasks::parse();
    let mut cmd = lmrs::cli::Cli::command();

    match task.task {
        Task::Man {} => {
            let man = clap_mangen::Man::new(cmd);
            man.render(&mut std::io::stdout()).unwrap();
        }
        Task::Complete(args) => match args.shell {
            Shell::Bash => {
                panic!("Not implemented for bash.");
                // generate_to(Bash, &mut cmd, &args.name, &args.output).unwrap();
            }
            Shell::Fish => {
                if args.output.is_some() {
                    panic!("<OUTPUT> argument is invalid for fish shell.");
                }
                let config_dir = env::var("XDG_CONFIG_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| {
                        home::home_dir()
                            .map(|d| d.join(".config"))
                            .expect("Failed to get home directory.")
                    });
                let completion_dir = config_dir.join("fish/completions");
                if args.install {
                    println!("Installing in {:?}", completion_dir);
                    generate_to(shells::Fish, &mut cmd, &args.name, completion_dir).unwrap();
                } else {
                    println!("Dryrun: install in {:?}", completion_dir);
                }
            }
            Shell::Zsh => {
                panic!("Not implemented for zsh.");
                // generate_to(Zsh, &mut cmd, &args.name, &args.output).unwrap();
            }
        },
    }
}
