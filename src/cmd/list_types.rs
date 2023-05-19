use std::path::Path;

use clap::Args;
use eyre::Result;
use itertools::Itertools;
use umls::{
    files::Files,
    index::{build::read_semantic_types, Index},
};

#[derive(Debug, Args)]
pub struct ListTypesArgs {
    /// Only show types that are indexed
    #[clap(short, long)]
    indexed_only: bool,
}

pub fn run(base_dir: &Path, files: Files, args: ListTypesArgs) -> Result<()> {
    let types = if args.indexed_only {
        let index = Index::new(&base_dir.join("index"))?;
        index.semantic_types
    } else {
        read_semantic_types(&files)?
    };

    types
        .into_values()
        .sorted_by(|a, b| a.tree_number.cmp(&b.tree_number))
        .for_each(|t| println!("{} - {}", t.tree_number, t.name));

    Ok(())
}
