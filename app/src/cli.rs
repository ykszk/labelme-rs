use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[clap(name=env!("CARGO_CRATE_NAME"), author, version, about, long_about = None)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create HTML catalog from a labelme directory
    #[clap(aliases = &["html"])]
    Catalog(HtmlCmdArgs),
    /// Create SVG image from a labeme annotation (json)
    Svg(SvgCmdArgs),
    /// Validate labelme annotations
    Validate(ValidateCmdArgs),
    /// Swap prefix of imagePath
    Swap(SwapCmdArgs),
    /// Concat json files with `filename` key added into jsonl file
    #[clap(aliases = &["ndjson"])]
    Jsonl(JsonlCmdArgs),
    /// Split jsonl into json files
    Split(SplitCmdArgs),
    /// Filter jsonl based on validation result
    Filter(FilterCmdArgs),
    /// Drop duplicates except for the first occurrence
    Drop(DropCmdArgs),
    /// Join ndjson files
    Join(JoinCmdArgs),
    /// Scale point coordinates according to the resize parameter
    Resize(ResizeCmdArgs),
}

#[derive(Debug, Args)]
pub struct DropCmdArgs {
    /// Input ndjson. Specify "-" to use stdin
    pub input: PathBuf,
    /// Key for duplicate checking
    #[clap(long, default_value = "filename")]
    pub key: String,
}

#[derive(Args, Debug)]
pub struct FilterCmdArgs {
    /// Input jsonl filename. Specify '-' to use stdin
    pub input: PathBuf,
    /// Text file(s) containing rules
    #[clap(short, long)]
    pub rules: Vec<PathBuf>,
    /// Check only json files containing given flag(s). Multiple flags are concatenated by OR.
    #[clap(short, long)]
    pub flag: Vec<String>,
    /// Ignore json files containing given flag(s). Multiple flags are concatenated by OR.
    #[clap(short, long)]
    pub ignore: Vec<String>,
    /// Invert filtering. i.e. output invalid lines
    #[clap(long, action)]
    pub invert: bool,
}

#[derive(Args, Debug)]
pub struct ValidateCmdArgs {
    /// Rules
    pub rules: PathBuf,
    /// Input directory
    pub input: PathBuf,
    /// Check only json files containing given flag(s). Multiple flags are concatenated by OR.
    #[clap(short, long)]
    pub flag: Vec<String>,
    /// Ignore json files containing given flag(s). Multiple flags are concatenated by OR.
    #[clap(short, long)]
    pub ignore: Vec<String>,
    /// Additional rules
    #[clap(short, long)]
    pub additional: Vec<PathBuf>,
    /// Report stats at the end
    #[clap(short, long)]
    pub stats: bool,
    /// Set verbosity
    #[clap(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
    /// Set the number of threads
    #[clap(short, long, default_value_t = 0)]
    pub threads: usize,
}

#[derive(Debug, Args)]
pub struct HtmlCmdArgs {
    /// Input labelme directory or jsonl with `filename` data (e.g. output of `lmrs jsonl`).
    /// Specify "-" to use stdin as input
    pub input: PathBuf,
    /// Output html filename
    pub output: PathBuf,
    /// Config filename. Used for `label_colors`
    #[clap(short, long)]
    pub config: Option<PathBuf>,
    /// Flags filename. Used to sort flags
    #[clap(short, long)]
    pub flags: Option<PathBuf>,
    /// Circle radius
    #[clap(long, default_value = "2")]
    pub radius: usize,
    /// Line width
    #[clap(long, default_value = "2")]
    pub line_width: usize,
    /// Resize image. Specify in imagemagick's `-resize`-like format
    #[clap(long)]
    pub resize: Option<String>,
    /// HTML title
    #[clap(long, default_value = "catalog")]
    pub title: String,
    /// CSS filename
    #[clap(long)]
    pub css: Option<PathBuf>,
    /// Override imagePath's directory
    #[clap(long)]
    pub image_dir: Option<PathBuf>,
    /// The number of jobs. Use all available cores by default.
    #[clap(short, long)]
    pub jobs: Option<usize>,
}

#[derive(Debug, Args)]
pub struct SvgCmdArgs {
    /// Input json filename
    pub input: PathBuf,
    /// Output svg filename
    pub output: PathBuf,
    /// Config filename. Used for `label_colors`
    #[clap(short, long)]
    pub config: Option<PathBuf>,
    /// Circle radius
    #[clap(long, default_value = "2")]
    pub radius: usize,
    /// Line width
    #[clap(long, default_value = "2")]
    pub line_width: usize,
    /// Resize image. Specify in imagemagick's `-resize`-like format
    #[clap(long)]
    pub resize: Option<String>,
}

#[derive(Args, Debug)]
pub struct SwapCmdArgs {
    /// Input json or jsonl/ndjson filename or json containing directory. Specify `-` for jsonl input with stdin (for piping).
    pub input: PathBuf,
    /// New imagePath prefix
    pub prefix: String,
    /// Output json filename or output directory. Defaults: <INPUT> for directory or single file input, stdout for jsonl/ndjson input.
    pub output: Option<PathBuf>,
    /// Swap prefix of the value associated by the given key instead of `imagePath`
    #[clap(long, default_value = "imagePath")]
    pub key: String,
}

#[derive(Args, Debug)]
pub struct ResizeCmdArgs {
    /// Input jsonl/ndjson. Specify `-` to use stdin
    pub input: PathBuf,
    /// Resize parameter. Specify in imagemagick's `-resize`-like format
    pub param: String,
}

#[derive(Debug, Args)]
pub struct JsonlCmdArgs {
    /// Directories, json files, or ndjson/jsonl files
    #[clap(required=true, num_args=1..)]
    pub input: Vec<PathBuf>,
    /// Key for filename
    #[clap(long, default_value = "filename", id = "key")]
    pub filename: String,
}

#[derive(Debug, Args)]
pub struct SplitCmdArgs {
    /// Input jsonl filename. Stdin is used if omitted
    pub input: Option<PathBuf>,
    /// Output directory. Working directory is used by default
    #[clap(short, long)]
    pub output: Option<PathBuf>,
    /// Key for filename
    #[clap(long, default_value = "filename", id = "key")]
    pub filename: String,
    /// Overwrite json files if exist
    #[clap(long, action)]
    pub overwrite: bool,
}

#[derive(Debug, Args)]
pub struct JoinCmdArgs {
    /// Input ndjson. Specify "-" to use stdin
    #[clap(required=true, num_args=2..)]
    pub input: Vec<PathBuf>,
    /// Key to join based on
    #[clap(long, default_value = "filename")]
    pub key: String,
    /// Join mode
    #[clap(long, default_value = "outer")]
    pub mode: JoinMode,
}

#[derive(ValueEnum, Debug, Copy, Clone)]
pub enum JoinMode {
    /// Inner
    Inner,
    /// Left inner
    Left,
    /// Outer
    Outer,
}
