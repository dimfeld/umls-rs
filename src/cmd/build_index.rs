use std::path::{Path, PathBuf};

use clap::Args;
use eyre::Result;
use umls::{files::Files, search::build::build_string_search};

#[derive(Args, Debug)]
pub struct BuildIndexArgs {
    #[arg(
        short,
        long,
        env,
        help = "The directory to write the UMLS files. Defaults to the same directory containing the UMLS data files"
    )]
    pub output: Option<PathBuf>,
}

pub fn run(base_dir: &Path, files: Files, args: BuildIndexArgs) -> Result<()> {
    let output = args
        .output
        .unwrap_or_else(|| base_dir.to_path_buf())
        .join("index");

    std::fs::create_dir(&output)?;

    build_string_search(&output, &files)?;

    Ok(())
}
