use std::path::Path;

use eyre::Result;
use itertools::Itertools;
use umls::{files::Files, index::Index};

pub fn run(dir: &Path, _files: Files) -> Result<()> {
    let dir = dir.join("index");
    let index = Index::new(&dir)?;

    index
        .concepts
        .iter()
        .flat_map(|c| c.codes.iter())
        .counts_by(|c| &c.source)
        .into_iter()
        .sorted_by(|(aname, _), (bname, _)| aname.cmp(bname))
        .for_each(|c| println!("{:?}", c));

    Ok(())
}
