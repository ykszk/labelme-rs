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
    /// Swap prefix (or suffix) of imagePath
    Swap(SwapCmdArgs),
    /// Create ndjson with `content` and `filename` keys
    #[clap(aliases = &["jsonl"])]
    Ndjson(NdjsonCmdArgs),
    /// Split ndjson into json files. i.e. reverse of `lmrs ndjson`
    Split(SplitCmdArgs),
    /// Filter ndjson based on validation result
    Filter(FilterCmdArgs),
    /// Remove labels from ndjson
    Remove(RemoveCmdArgs),
    /// Drop duplicates except for the first occurrence
    Drop(DropCmdArgs),
    /// Join ndjson files
    Join(JoinCmdArgs),
    /// Scale point coordinates according to the resize parameter
    Resize(ResizeCmdArgs),
    /// Create empty labelme json for the image
    Init(InitCmdArgs),
    /// Check if `imagePath` exists. `imagePath` is resolved relative to the input ndjson file or the current working directory if the input is stdin
    Exist(ExistCmdArgs),
    /// Archive json and associated images
    Archive(ArchiveCmdArgs),
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
    /// Input ndjson filename. Specify '-' to use stdin
    pub input: PathBuf,
    /// Text file(s) containing rules
    #[clap(short, long)]
    pub rules: Vec<PathBuf>,
    /// Invert filtering. i.e. output invalid lines
    #[clap(short = 'v', long)]
    pub invert: bool,
}

#[derive(Args, Debug)]
pub struct RemoveCmdArgs {
    /// Input ndjson filename. Specify '-' to use stdin
    pub input: PathBuf,
    /// Label(s) to remove
    #[clap(short, long, required = true)]
    pub label: Vec<String>,
    /// Invert removal condition.
    #[clap(short = 'v', long)]
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
    /// Input labelme directory or ndjson with `filename` data (e.g. output of `lmrs ndjson`).
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
    /// Input json or jsonl/ndjson filename or json containing directory. Specify `-` for ndjson input with stdin (for piping).
    pub input: PathBuf,
    /// New imagePath prefix (or suffix if `--suffix` is specified)
    pub prefix: String,
    /// Output json filename or output directory. Defaults: <INPUT> for directory or single file input, stdout for jsonl/ndjson input.
    pub output: Option<PathBuf>,
    /// Swap suffix (e.g. ".jpg") with the given suffix instead of swapping the prefix
    #[clap(long)]
    pub suffix: bool,
}

#[derive(Args, Debug)]
pub struct ResizeCmdArgs {
    /// Input jsonl/ndjson. Specify `-` to use stdin
    pub input: PathBuf,
    /// Resize parameter. Specify in imagemagick's `-resize`-like format
    pub param: String,
    /// Output directory for resized images
    #[clap(long)]
    pub image: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct NdjsonCmdArgs {
    /// Directories, json files, or ndjson/jsonl files
    #[clap(required=true, num_args=1..)]
    pub input: Vec<PathBuf>,
    /// Key for filename. Only for ndjson output
    #[clap(long, default_value = "filename", id = "key")]
    pub filename: String,
}

#[derive(Debug, Args)]
pub struct InitCmdArgs {
    /// Input image or image containing directory
    pub input: PathBuf,
    /// Image extension
    #[clap(long, default_value = "jpg")]
    pub extension: String,
    /// Key for filename. Only for ndjson output
    #[clap(long, default_value = "filename", id = "key")]
    pub filename: String,
}

#[derive(Debug, Args)]
pub struct ArchiveCmdArgs {
    /// Input directory
    pub input: PathBuf,
    /// Output archive (.tar)
    pub output: PathBuf,
}

#[derive(Debug, Args)]
pub struct SplitCmdArgs {
    /// Input ndjson filename. Stdin is used if omitted
    pub input: Option<PathBuf>,
    /// Output directory. Working directory is used by default
    #[clap(short, long)]
    pub output: Option<PathBuf>,
    /// Key for filename
    #[clap(long, default_value = "filename")]
    pub filename: String,
    /// Key for content
    #[clap(long, default_value = "content")]
    pub content: String,
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
    /// Missing key handling
    #[clap(long, default_value = "exit")]
    pub missing: MissingHandling,
}

#[derive(ValueEnum, Debug, Copy, Clone)]
pub enum JoinMode {
    /// Inner
    Inner,
    /// Left outer
    Left,
    /// Full outer
    Outer,
}

#[derive(ValueEnum, Debug, Copy, Clone, PartialEq, Eq)]
pub enum MissingHandling {
    /// Exit on missing key
    Exit,
    /// Continue on missing key
    Continue,
}

#[derive(Debug, Args)]
pub struct ExistCmdArgs {
    /// Input ndjson. Specify "-" to use stdin
    pub input: PathBuf,
    /// Invert output
    #[clap(short = 'v', long)]
    pub invert: bool,
}
