use clap::Args;
use eyre::Result;
use umls::files::Files;

#[derive(Debug, Args)]
pub struct ListSourcesArgs {}

pub fn run(files: Files, _args: ListSourcesArgs) -> Result<()> {
    let mut sources = files.read_sources()?;

    sources.sort_by(|a, b| a.abbreviation.cmp(&b.abbreviation));

    for source in sources {
        println!(
            "{} - {} - {} - {}",
            source.abbreviation, source.language, source.family, source.name
        )
    }

    Ok(())
}
