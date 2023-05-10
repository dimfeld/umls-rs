use std::path::Path;

use clap::Args;
use eyre::Result;
use fst::Streamer;
use umls::{files::Files, search::score::jaccard_trigram_distance};

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
        match index.search(&args.word)? {
            Some(w) => println!("Found {}", index.concept_id(w)),
            None => println!("Not found"),
        }
    } else {
        let mut output = index.fuzzy_search(&args.word, args.fuzzy)?;
        let mut results = Vec::new();
        while let Some((s, id, _)) = output.next() {
            let found = std::str::from_utf8(s)?.to_string();
            let score = jaccard_trigram_distance(&args.word, &found);
            results.push((score, id, found));
        }

        results.sort_by(|(ascore, _, _), (bscore, _, _)| bscore.total_cmp(ascore));

        if results.is_empty() {
            println!("No results found");
        } else {
            for (score, id, s) in results {
                println!("{s} ({score:.2}) - {}", index.concept_id(id));
            }
        }
    }

    Ok(())
}
