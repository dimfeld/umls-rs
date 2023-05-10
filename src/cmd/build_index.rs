use std::path::{Path, PathBuf};

use clap::Args;
use eyre::Result;
use umls::{
    files::Files,
    search::build::{build_string_search, IndexBuilderOptions},
};

#[derive(Args, Debug)]
pub struct BuildIndexArgs {
    /// The directory to write the UMLS files. Defaults to the same directory containing the UMLS data files
    #[arg(short, long, env)]
    pub output: Option<PathBuf>,

    /// If true, the index will be generated to match all strings in lowercase.
    #[arg(short = 'i', long, env)]
    pub case_insensitive: bool,

    /// The source abbreviations (SAB field) to include in the index. If empty, all sources are included.
    #[arg(short, long, env)]
    pub sources: Vec<String>,

    /// The languages (LAT field) to include in the index. If empty, all sources are included.
    #[arg(short, long, env)]
    pub languages: Vec<String>,
}

pub fn run(base_dir: &Path, files: Files, args: BuildIndexArgs) -> Result<()> {
    let output = args
        .output
        .unwrap_or_else(|| base_dir.to_path_buf())
        .join("index");

    std::fs::create_dir(&output)?;

    build_string_search(IndexBuilderOptions {
        output_dir: &output,
        files: &files,
        case_insensitive: args.case_insensitive,
        languages: args.languages,
        sources: args.sources,
    })?;

    Ok(())
}
