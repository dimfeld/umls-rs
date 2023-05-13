use std::path::Path;

use clap::Args;
use eyre::Result;
use fst::Streamer;
use itertools::Itertools;
use smol_str::SmolStr;
use umls::{files::Files, index::score::jaccard_trigram_distance};

#[derive(Args, Debug)]
pub struct SearchArgs {
    /// The path to the index directory
    pub word: String,

    /// The maximum Levenshtein distance to search for
    #[clap(short = 'f', long = "fuzzy", default_value_t = 0)]
    pub fuzzy: u32,

    /// Show output in long format
    #[clap(short = 'l', long = "long")]
    pub long: bool,

    /// Show these code sources
    #[clap(short = 'c', long = "code-source")]
    pub code_types: Vec<SmolStr>,

    /// The minimum score, using Jaccard trigram similarity, when performing fuzzy search
    #[clap(short = 't', long = "score-threshold", default_value_t = 0.7)]
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
                if args.long {
                    println!(
                        "Found in {}us\n{} - {}",
                        duration.as_micros(),
                        concept.cui,
                        concept.preferred_name
                    );

                    if !concept.codes.is_empty() {
                        println!("Codes:");
                        for (code_concept_id, code) in
                            index.downstream_codes(w as u32, &args.code_types)
                        {
                            let code_concept = &index.concepts[code_concept_id];
                            println!("  {} {}: {}", code_concept.cui, code.source, code.code);
                        }
                    }

                    if !concept.parents.is_empty() {
                        println!("Parents:");
                        let parents = concept
                            .parents
                            .iter()
                            .map(|&id| &index.concepts[id as usize])
                            .sorted_by_key(|c| &c.cui)
                            .collect::<Vec<_>>();
                        for parent_concept in &parents {
                            println!(
                                "  {} - {}",
                                parent_concept.cui, parent_concept.preferred_name
                            );
                        }
                    }

                    if !concept.children.is_empty() {
                        println!("Children:");
                        let children = concept
                            .children
                            .iter()
                            .map(|&id| &index.concepts[id as usize])
                            .sorted_by_key(|c| &c.cui)
                            .collect::<Vec<_>>();
                        for child_concept in &children {
                            println!("  {} - {}", child_concept.cui, child_concept.preferred_name);
                        }
                    }
                } else {
                    println!(
                        "Found ({}us) {} - {}",
                        duration.as_micros(),
                        concept.cui,
                        concept.preferred_name
                    );
                    if !concept.codes.is_empty() {
                        print!("Codes:");
                        for code in &concept.codes {
                            print!(" ({}, {})", code.source, code.code);
                        }
                    }
                }

                println!();
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
                if !concept.codes.is_empty() {
                    println!("  Codes: {:?}", concept.codes);
                }
            }
        }
    }

    Ok(())
}
