use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
use clap::Parser;
#[macro_use]
extern crate log;
use anyhow::Result;

mod drop_dups;
mod filter;
mod join;
mod jsonl;
mod lm2svg;
mod lms2html;
mod split_jsonl;
mod swap_prefix;
mod validate;

use lmrs::cli::Cli;
use lmrs::cli::Command;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Html(args) => lms2html::cmd(args),
        Command::Svg(args) => lm2svg::cmd(args),
        Command::Validate(args) => validate::cmd(args),
        Command::Swap(args) => swap_prefix::cmd(args),
        Command::Jsonl(args) => jsonl::cmd(args),
        Command::Split(args) => split_jsonl::cmd(args),
        Command::Filter(args) => filter::cmd(args),
        Command::Drop(args) => drop_dups::cmd(args),
        Command::Join(args) => join::cmd(args),
    }
}
