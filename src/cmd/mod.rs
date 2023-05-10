mod build_index;
mod extract;
mod list_files;
mod list_sources;
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
    ListFiles(list_files::ListFilesArgs),
    ListSources(list_sources::ListSourcesArgs),
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
        Command::ListFiles(a) => list_files::run(files, a),
        Command::ListSources(a) => list_sources::run(files, a),
        Command::BuildIndex(a) => build_index::run(&dir, files, a),
        Command::Search(a) => search::run(&dir, files, a),
        Command::Extract(_) => unreachable!(),
    }
}
