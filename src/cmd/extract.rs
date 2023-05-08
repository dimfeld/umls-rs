use std::path::{Path, PathBuf};

use clap::Args;
use eyre::Result;
use umls::extract::extract_metathesaurus;

#[derive(Debug, Args)]
pub struct ExtractArgs {
    #[clap(
        long,
        short,
        help = "The input directory or UMLS ZIP file to extract from"
    )]
    pub input: Option<PathBuf>,
    #[clap(long, short, help = "The output directory to extract to")]
    pub output: PathBuf,
}

pub fn run(input_path: &Path, args: ExtractArgs) -> Result<()> {
    let input_path = args.input.unwrap_or_else(|| input_path.to_path_buf());
    extract_metathesaurus(&input_path, &args.output)?;

    Ok(())
}
