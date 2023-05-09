use std::path::Path;

use clap::Args;
use eyre::Result;
use fst::Streamer;
use umls::files::Files;

#[derive(Args, Debug)]
pub struct SearchArgs {
    #[clap(help = "The path to the index directory")]
    pub word: String,

    #[clap(
        short = 'f',
        long = "fuzzy",
        default_value_t = 0,
        help = "The maximum Levenshtein distance to search for"
    )]
    pub fuzzy: u32,
}

pub fn run(base_dir: &Path, _files: Files, args: SearchArgs) -> Result<()> {
    let dir = base_dir.join("index");
    let index = umls::search::Searcher::new(&dir)?;

    if args.fuzzy == 0 {
        match index.search(&args.word) {
            Some(w) => println!("Found {w}"),
            None => println!("Not found"),
        }
    } else {
        let mut output = index.fuzzy_search(&args.word, args.fuzzy)?;
        let mut found = false;
        while let Some((s, id, _)) = output.next() {
            found = true;
            println!("{} - {}", std::str::from_utf8(s).unwrap(), id);
        }

        if !found {
            println!("No results found");
        }
    }

    Ok(())
}
