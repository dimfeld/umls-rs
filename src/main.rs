use clap::Parser;
use eyre::Result;
use umls::files::Files;

mod cmd;

fn main() -> Result<()> {
    let args = cmd::Args::parse();
    println!("{}", args.dir);

    let mut files = Files::new(args.dir)?;

    let schema = files.read_schema_descriptions()?;

    for file in schema {
        println!(
            "{} {} {} {}",
            file.filename, file.description, file.num_rows, file.num_bytes
        );

        for col in file.columns {
            println!("  {} {}", col.name, col.description);
        }
    }

    Ok(())
}
