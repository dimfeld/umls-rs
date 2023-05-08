use clap::Parser;
use eyre::Result;

mod cmd;

fn main() -> Result<()> {
    let args = cmd::Args::parse();

    cmd::run(args)
}
