use clap::Args;
use eyre::Result;
use umls::files::Files;

#[derive(Debug, Args)]
pub struct ListFilesArgs {
    #[arg(short, long, help = "Print the columns descriptions for each file")]
    schema: bool,
}

pub fn run(files: Files, args: ListFilesArgs) -> Result<()> {
    let schema = files.read_schema_descriptions()?;

    for file in schema {
        println!(
            "{} - {} - {} rows, {} bytes",
            file.filename, file.description, file.num_rows, file.num_bytes
        );

        if args.schema {
            for col in file.columns {
                println!("  {} {}", col.name, col.description);
            }
            println!();
        }
    }

    Ok(())
}
