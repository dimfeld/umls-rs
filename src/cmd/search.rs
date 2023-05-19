use std::path::Path;

use clap::Args;
use eyre::Result;
use fst::Streamer;
use itertools::Itertools;
use smol_str::SmolStr;
use umls::{
    files::Files,
    index::{score::jaccard_trigram_distance, Index},
};

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

fn print_sorted_concept_list(label: &str, ids: &[u32], index: &Index) {
    if ids.is_empty() {
        return;
    }

    println!("{label}:");

    ids.iter()
        .map(|&id| &index.concepts[id as usize])
        .sorted_by_key(|c| &c.cui)
        .for_each(|concept| {
            println!("  {} - {}", concept.cui, concept.preferred_name);
        });
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

                    println!("Semantic Types:");
                    for id in &concept.types {
                        if let Some(type_data) = index.semantic_types.get(id) {
                            println!("  {} - {}", type_data.tree_number, type_data.name);
                        }
                    }

                    if !concept.codes.is_empty() {
                        println!("Codes:");
                        for (code_concept_id, code) in
                            index.downstream_codes(w as u32, &args.code_types)
                        {
                            let code_concept = &index.concepts[code_concept_id];
                            println!("  {} {}: {}", code_concept.cui, code.source, code.code);
                        }
                    }

                    print_sorted_concept_list("Parents", &concept.parents, &index);
                    print_sorted_concept_list("Children", &concept.children, &index);
                    print_sorted_concept_list("Similar", &concept.similar, &index);
                    print_sorted_concept_list("Synonyms", &concept.synonym, &index);
                    print_sorted_concept_list(
                        "Related, possibly synonymous",
                        &concept.related_possibly_synonymous,
                        &index,
                    );
                    print_sorted_concept_list(
                        "Allowed Qualifiers",
                        &concept.allowed_qualifier,
                        &index,
                    );
                    print_sorted_concept_list("Qualified By", &concept.qualified_by, &index);
                    print_sorted_concept_list(
                        "Other Relationship",
                        &concept.other_relationship,
                        &index,
                    );
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
