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

#[derive(Subcommand)]
enum Command {
    /// Create HTML from a labelme directory
    Html(lms2html::HtmlArgs),
    /// Create SVG image from a labeme annotation (json)
    Svg(lm2svg::SvgArgs),
    /// Validate labelme annotations
    Validate(validate::ValidateArgs),
    /// Swap prefix of imagePath
    Swap(swap_prefix::SwapArgs),
    /// Concat json files with `filename` key added into jsonl file
    Jsonl(jsonl::CmdArgs),
    /// Split jsonl into json files
    Split(split_jsonl::CmdArgs),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Html(args) => lms2html::cmd_html(args),
        Command::Svg(args) => lm2svg::cmd_svg(args),
        Command::Validate(args) => validate::cmd_validate(args),
        Command::Swap(args) => swap_prefix::cmd_swap(args),
        Command::Jsonl(args) => jsonl::cmd_jsonl(args),
        Command::Split(args) => split_jsonl::cmd(args),
    }
}
