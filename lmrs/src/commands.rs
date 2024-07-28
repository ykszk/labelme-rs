use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
use clap::Parser;
#[macro_use]
extern crate log;
use anyhow::Result;

mod archive;
mod drop_dups;
mod exist;
mod filter;
mod init;
mod join;
mod lm2svg;
mod lms2html;
mod ndjson;
mod remove;
mod resize;
mod shapeshift;
mod split_ndjson;
mod swap_prefix;
mod validate;
mod count;

use lmrs::cli::Cli;
use lmrs::cli::Command;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    match cli.command {
        Command::Catalog(args) => lms2html::cmd(args),
        Command::Svg(args) => lm2svg::cmd(args),
        Command::Validate(args) => validate::cmd(args),
        Command::Swap(args) => swap_prefix::cmd(args),
        Command::Ndjson(args) => ndjson::cmd(args),
        Command::Split(args) => split_ndjson::cmd(args),
        Command::Filter(args) => filter::cmd(args),
        Command::Drop(args) => drop_dups::cmd(args),
        Command::Join(args) => join::cmd(args),
        Command::Resize(args) => resize::cmd(args),
        Command::Init(args) => init::cmd(args),
        Command::Exist(args) => exist::cmd(args),
        Command::Remove(args) => remove::cmd(args),
        Command::Shapeshift(args) => shapeshift::cmd(args),
        Command::Archive(args) => archive::cmd(args),
        Command::Count(args) => count::cmd(args),
    }
}
