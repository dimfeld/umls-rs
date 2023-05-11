use std::path::Path;

use clap::Args;
use eyre::Result;
use fst::Streamer;
use umls::{files::Files, index::score::jaccard_trigram_distance};

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

    #[clap(
        short = 's',
        long = "score-threshold",
        default_value_t = 0.7,
        help = "The minimum score, using Jaccard trigram similarity, when performing fuzzy search"
    )]
    pub score_threshold: f32,
}

pub fn run(base_dir: &Path, _files: Files, args: SearchArgs) -> Result<()> {
    let dir = base_dir.join("index");
    let index = umls::index::Index::new(&dir)?;

    let start_time = std::time::Instant::now();
    if args.fuzzy == 0 {
        match index.search(&args.word)? {
            Some(w) => {
                let duration = start_time.elapsed();
                let concept = index.concept_id(w);
                println!(
                    "Found ({}us) {} - {}",
                    duration.as_micros(),
                    concept.cui,
                    concept.preferred_name
                )
            }
            None => println!("Not found"),
        }
    } else {
        let mut output = index.fuzzy_search(&args.word, args.fuzzy)?;
        let mut results = Vec::new();
        while let Some((s, id, _)) = output.next() {
            let found = std::str::from_utf8(s)?.to_string();
            let score = jaccard_trigram_distance(&args.word, &found);
            if score >= args.score_threshold {
                results.push((score, id, found));
            }
        }

        results.sort_by(|(ascore, _, _), (bscore, _, _)| bscore.total_cmp(ascore));
        let duration = start_time.elapsed();
        println!("Search completed in {}us", duration.as_micros());

        if results.is_empty() {
            println!("No results found");
        } else {
            for (score, id, s) in results {
                let concept = index.concept_id(id);
                println!(
                    "{s} ({score:.2}) - {} - {}",
                    concept.cui, concept.preferred_name
                );
            }
        }
    }

    Ok(())
}
