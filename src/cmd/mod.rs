mod build_index;
mod extract;
mod list;
mod search;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use eyre::Result;
use umls::files::Files;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    #[arg(short, long, env, help = "The directory containing the UMLS files")]
    pub dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    List(list::ListArgs),
    Extract(extract::ExtractArgs),
    BuildIndex(build_index::BuildIndexArgs),
    Search(search::SearchArgs),
}

pub fn run(args: Args) -> Result<()> {
    let dir = args.dir.unwrap_or_else(|| std::env::current_dir().unwrap());
    // Extract is special because we don't assume the files have already been extracted.
    if let Command::Extract(a) = args.command {
        return extract::run(&dir, a);
    }

    let files = Files::new(&dir)?;
    match args.command {
        Command::List(a) => list::run(files, a),
        Command::BuildIndex(a) => build_index::run(&dir, files, a),
        Command::Search(a) => search::run(&dir, files, a),
        Command::Extract(_) => unreachable!(),
    }
}
