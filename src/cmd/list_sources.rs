use clap::Args;
use eyre::Result;
use umls::files::Files;

#[derive(Debug, Args)]
pub struct ListSourcesArgs {}

pub fn run(files: Files, _args: ListSourcesArgs) -> Result<()> {
    let sources = files.read_sources()?;

    for source in sources {
        println!(
            "{} - {} - {} - {}",
            source.abbreviation, source.language, source.family, source.name
        )
    }

    Ok(())
}
