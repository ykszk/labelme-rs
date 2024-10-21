use clap::{Args, Parser, Subcommand, ValueEnum, ValueHint};
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
    /// Change shape type
    Shapeshift(ShapeshiftCmdArgs),
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
    /// Archive json and associated images as a tarball
    Archive(ArchiveCmdArgs),
    /// Count flags
    Count(CountCmdArgs),
    /// Sort shapes by point coordinates
    Sort(SortCmdArgs),
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
pub struct ReshapeCircle2Point {
    /// Point index to use as the point (0 or 1)
    #[clap(short, long, default_value = "0")]
    pub index: usize,
}

#[derive(Debug, Subcommand)]
pub enum ReshapeType {
    /// Circle to point
    C2P(ReshapeCircle2Point),
}

#[derive(Args, Debug)]
pub struct ShapeshiftCmdArgs {
    /// Input ndjson filename. Specify '-' to use stdin
    pub input: PathBuf,
    /// Label(s) to remove
    #[clap(subcommand)]
    pub reshape: ReshapeType,
}

#[derive(Args, Debug)]
pub struct ValidateCmdArgs {
    /// Rules
    #[clap(value_hint = ValueHint::FilePath)]
    pub rules: PathBuf,
    /// Input directory
    #[clap(value_hint = ValueHint::DirPath)]
    pub input: PathBuf,
    /// Check only json files containing given flag(s). Multiple flags are concatenated by OR.
    #[clap(short, long)]
    pub flag: Vec<String>,
    /// Ignore json files containing given flag(s). Multiple flags are concatenated by OR.
    #[clap(short, long, value_hint = ValueHint::Other)]
    pub ignore: Vec<String>,
    /// Additional rules
    #[clap(short, long, value_hint = ValueHint::FilePath)]
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
    #[clap(value_hint = ValueHint::FilePath)]
    pub output: PathBuf,
    /// Config filename. Used for `label_colors`
    #[clap(short, long, value_hint = ValueHint::FilePath)]
    pub config: Option<PathBuf>,
    /// Flags filename. Used to sort flags
    #[clap(short, long, value_hint = ValueHint::FilePath)]
    pub flags: Option<PathBuf>,
    /// Circle radius
    #[clap(long, default_value = "2")]
    pub radius: usize,
    /// Line width
    #[clap(long, default_value = "2")]
    pub line_width: usize,
    /// Resize image. Specify in imagemagick's `-resize`-like format
    #[clap(long, value_hint = ValueHint::Other)]
    pub resize: Option<String>,
    /// HTML title
    #[clap(long, default_value = "catalog", value_hint = ValueHint::Other)]
    pub title: String,
    /// CSS filename
    #[clap(long, value_hint = ValueHint::FilePath)]
    pub css: Option<PathBuf>,
    /// Override imagePath's directory
    #[clap(long, value_hint = ValueHint::DirPath)]
    pub image_dir: Option<PathBuf>,
    /// The number of jobs. Use all available cores by default.
    #[clap(short, long)]
    pub jobs: Option<usize>,
}

#[derive(Debug, Args)]
pub struct SvgCmdArgs {
    /// Input json filename
    #[clap(value_hint = ValueHint::FilePath)]
    pub input: PathBuf,
    /// Output svg filename
    #[clap(value_hint = ValueHint::FilePath)]
    pub output: PathBuf,
    /// Config filename. Used for `label_colors`
    #[clap(short, long, value_hint = ValueHint::FilePath)]
    pub config: Option<PathBuf>,
    /// Circle radius
    #[clap(long, default_value = "2")]
    pub radius: usize,
    /// Line width
    #[clap(long, default_value = "2")]
    pub line_width: usize,
    /// Resize image. Specify in imagemagick's `-resize`-like format
    #[clap(long, value_hint = ValueHint::Other)]
    pub resize: Option<String>,
}

#[derive(Args, Debug)]
pub struct SwapCmdArgs {
    /// Input json or jsonl/ndjson filename or json containing directory. Specify `-` for ndjson input with stdin (for piping).
    pub input: PathBuf,
    /// New imagePath prefix (or suffix if `--suffix` is specified)
    #[clap(value_hint = ValueHint::Other)]
    pub prefix: String,
    /// Output json filename or output directory. Defaults: <INPUT> for directory or single file input, stdout for jsonl/ndjson input.
    #[clap(value_hint = ValueHint::FilePath)]
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
    #[clap(value_hint = ValueHint::Other)]
    pub param: String,
    /// Output directory for resized images
    #[clap(long, value_hint = ValueHint::DirPath)]
    pub image: Option<PathBuf>,
}

#[derive(ValueEnum, Debug, Copy, Clone)]
pub enum ParentHandling {
    /// Keep given parent directory
    Keep,
    /// Change to absolute path
    Absolute,
    /// Remove parent directory
    Remove,
}

#[derive(Debug, Args)]
pub struct NdjsonCmdArgs {
    /// Directories, json files, or ndjson/jsonl files
    #[clap(required=true, num_args=1.., value_hint = ValueHint::AnyPath)]
    pub input: Vec<PathBuf>,
    /// Key for filename. Only for ndjson output
    #[clap(long, default_value = "filename", id = "key", value_hint = ValueHint::Other)]
    pub filename: String,
    /// Change parent directory in the `filename` field of the output. Applicable only for json and directory inputs
    #[clap(short, long, default_value = "keep")]
    pub parent: ParentHandling,
    /// Glob pattern. Default: "*.json". Specify "**/*.json" for recursive search
    #[clap(short, long, default_value = "*.json", value_hint = ValueHint::Other)]
    pub glob: String,
    /// Do not ignore entries starting with `.`
    #[clap(short, long)]
    pub all: bool,
}

#[derive(Debug, Args)]
pub struct InitCmdArgs {
    /// Input image or image containing directory
    #[clap(value_hint = ValueHint::DirPath)]
    pub input: PathBuf,
    /// Image extension
    #[clap(long, default_value = "jpg", value_hint = ValueHint::Other)]
    pub extension: String,
    /// Key for filename. Only for ndjson output
    #[clap(long, default_value = "filename", id = "key", value_hint = ValueHint::Other)]
    pub filename: String,
}

#[derive(Debug, Args)]
pub struct ArchiveCmdArgs {
    /// Input directory
    #[clap(value_hint = ValueHint::DirPath)]
    pub input: PathBuf,
    /// Output archive (.tar) or "-" for stdout
    #[clap(value_hint = ValueHint::FilePath)]
    pub output: PathBuf,
}

#[derive(ValueEnum, Debug, Copy, Clone)]
pub enum SplitParentHandling {
    /// Keep the parent directory
    Keep,
    /// Ignore the parent directory
    Ignore,
}

#[derive(Debug, Args)]
pub struct SplitCmdArgs {
    /// Input ndjson filename. Stdin is used if omitted
    #[clap(value_hint = ValueHint::FilePath)]
    pub input: Option<PathBuf>,
    /// Output directory. Working directory is used by default
    #[clap(short, long, value_hint = ValueHint::DirPath)]
    pub output: Option<PathBuf>,
    /// Key for filename
    #[clap(long, default_value = "filename", value_hint = ValueHint::Other)]
    pub filename: String,
    /// Key for content
    #[clap(long, default_value = "content", value_hint = ValueHint::Other)]
    pub content: String,
    /// Overwrite json files if exist
    #[clap(long, action)]
    pub overwrite: bool,
    /// How to handle the parent directory in the filename field
    #[clap(short, long, default_value = "keep")]
    pub parent: SplitParentHandling,
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
    /// Invert output. i.e. output non-existing files
    #[clap(short = 'v', long)]
    pub invert: bool,
}

#[derive(Debug, Args)]
pub struct CountCmdArgs {
    /// Input json or jsonl/ndjson filename or json containing directory. Specify `-` for ndjson input with stdin (for piping).
    pub input: PathBuf,
}

#[derive(Debug, Args)]
pub struct SortCmdArgs {
    /// Input json or jsonl/ndjson filename.
    pub input: PathBuf,

    /// Sort by x coordinate
    #[clap(short = 'x', long)]
    pub by_x: bool,

    /// Sort in descending order
    #[clap(short, long)]
    pub descending: bool,
}
