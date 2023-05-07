use clap::Parser;
use eyre::Result;
use umls::files::Files;

mod cmd;

fn main() -> Result<()> {
    let args = cmd::Args::parse();
    println!("{}", args.dir);

    let files = Files::new(args.dir)?;

    for file in files.files {
        println!(
            "{} {} {}",
            file.container, file.index_in_container, file.name
        );
    }

    Ok(())
}
