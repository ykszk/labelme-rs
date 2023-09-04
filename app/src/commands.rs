use clap::{Parser, Subcommand};
#[macro_use]
extern crate log;

#[derive(Parser)]
#[clap(name=env!("CARGO_BIN_NAME"), author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}
mod lm2svg;
mod lms2html;
mod swap_prefix;
mod validate;
mod jsonl;
mod split_jsonl;
mod filter;

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
    }
}
