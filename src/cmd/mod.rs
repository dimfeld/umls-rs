mod extract;
mod list;

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
        Command::Extract(_) => unreachable!(),
    }
}
