use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
use clap::{CommandFactory, Parser};
use clap_complete::{generate, Generator};
#[macro_use]
extern crate log;
use anyhow::Result;

mod archive;
mod browse;
mod count;
mod drop_dups;
mod exist;
mod filter;
mod init;
mod join;
mod lm2svg;
mod lms2html;
mod mat;
mod merge;
mod ndjson;
mod prune;
mod remove;
mod resize;
mod shapeshift;
mod sort;
mod split_ndjson;
mod stats;
mod swap_prefix;
mod validate;

use lmrs::cli::Cli;
use lmrs::cli::Command;

fn print_completions<G: Generator>(gen: G, cmd: &mut clap::Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut std::io::stdout());
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    match cli.command {
        Command::Complete(args) => {
            let mut cmd = Cli::command();
            print_completions(args.shell, &mut cmd);
            Ok(())
        }
        Command::Catalog(args) => lms2html::cmd(args),
        Command::Svg(args) => lm2svg::cmd(args),
        Command::Validate(args) => validate::cmd(args),
        Command::Swap(args) => swap_prefix::cmd(args),
        Command::Ndjson(args) => ndjson::cmd(args),
        Command::Split(args) => split_ndjson::cmd(args),
        Command::Filter(args) => filter::cmd(args),
        Command::Drop(args) => drop_dups::cmd(args),
        Command::Join(args) => join::cmd(args),
        Command::Merge(args) => merge::cmd(args),
        Command::Mat(args) => mat::cmd(args),
        Command::Resize(args) => resize::cmd(args),
        Command::Init(args) => init::cmd(args),
        Command::Exist(args) => exist::cmd(args),
        Command::Remove(args) => remove::cmd(args),
        Command::Shapeshift(args) => shapeshift::cmd(args),
        Command::Archive(args) => archive::cmd(args),
        Command::Count(args) => count::cmd(args),
        Command::Sort(args) => sort::cmd(args),
        Command::Browse(args) => browse::cmd(args),
        Command::Stats(args) => stats::cmd(args),
        Command::Prune(args) => prune::cmd(args),
    }
}
