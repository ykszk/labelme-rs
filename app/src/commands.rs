use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
use clap::{Parser, Subcommand};
#[macro_use]
extern crate log;

#[derive(Parser)]
#[clap(name=env!("CARGO_BIN_NAME"), author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

mod drop_dups;
mod filter;
mod join;
mod jsonl;
mod lm2svg;
mod lms2html;
mod split_jsonl;
mod swap_prefix;
mod validate;

#[derive(Subcommand)]
enum Command {
    /// Create HTML from a labelme directory
    Html(lms2html::CmdArgs),
    /// Create SVG image from a labeme annotation (json)
    Svg(lm2svg::CmdArgs),
    /// Validate labelme annotations
    Validate(validate::CmdArgs),
    /// Swap prefix of imagePath
    Swap(swap_prefix::CmdArgs),
    /// Concat json files with `filename` key added into jsonl file
    #[clap(aliases = &["ndjson"])]
    Jsonl(jsonl::CmdArgs),
    /// Split jsonl into json files
    Split(split_jsonl::CmdArgs),
    /// Filter jsonl based on validation result
    Filter(filter::CmdArgs),
    /// Drop duplicates except for the first occurrence
    Drop(drop_dups::CmdArgs),
    /// Join ndjson files
    Join(join::CmdArgs),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
