pub mod list;

use clap::{Parser, Subcommand};
use eyre::Result;
use umls::files::Files;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    #[arg(
        short,
        long,
        env,
        default_value_t = String::from("."),
        help = "The directory containing the UMLS files"
    )]
    pub dir: String,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    List(list::ListArgs),
}

pub fn run(files: Files, args: Args) -> Result<()> {
    match args.command {
        Command::List(list) => list::run(files, list),
    }
}
