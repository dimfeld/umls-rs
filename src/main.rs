use clap::Parser;
use eyre::Result;
use umls::files::Files;

mod cmd;

fn main() -> Result<()> {
    let args = cmd::Args::parse();
    let files = Files::new(&args.dir)?;

    cmd::run(files, args)
}
