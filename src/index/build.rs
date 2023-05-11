use std::path::Path;
use std::{collections::BTreeMap, io::Write};

use ahash::{HashMap, HashMapExt};
use eyre::Result;
use flate2::write::GzEncoder;
use fst::MapBuilder;
use smallvec::SmallVec;
use smol_str::SmolStr;

use crate::files::Files;

use super::{Concept, SearchIndexMeta, METADATA_NAME};
use super::{CONCEPTS_LST_NAME, STRINGS_FST_NAME};

pub struct IndexBuilderOptions<'a> {
    pub output_dir: &'a Path,
    pub files: &'a Files,
    pub case_insensitive: bool,
    /// The languages to include in the index. If empty, all languages will be included.
    pub languages: Vec<SmolStr>,
    /// The sources to include in the index. If empty, all sources are included.
    pub sources: Vec<SmolStr>,
}

pub fn build_index(options: IndexBuilderOptions) -> Result<()> {
    let IndexBuilderOptions {
        output_dir,
        files,
        case_insensitive,
        languages,
        sources,
    } = options;

    let ranks = read_ranks(files)?;

    let mut mrconso = files.get_file_stream("MRCONSO")?;

    let cui_idx = mrconso.columns.iter().position(|c| c == "CUI").unwrap();
    let lang_idx = mrconso.columns.iter().position(|c| c == "LAT").unwrap();
    let str_idx = mrconso.columns.iter().position(|c| c == "STR").unwrap();
    let tty_idx = mrconso.columns.iter().position(|c| c == "TTY").unwrap();
    let source_idx = mrconso.columns.iter().position(|c| c == "SAB").unwrap();

    // First build the lookups. We just do this in memory since in there are expected to be a few
    // tens of millions of strings.
    let mut string_to_number = BTreeMap::new();
    let mut concepts: HashMap<SmolStr, (u32, u32, Concept)> = HashMap::new();

    for line in mrconso.reader.records() {
        let line = line?;
        let cui = line.get(cui_idx).unwrap();
        let orig_string = SmolStr::from(line.get(str_idx).unwrap());

        let string = if case_insensitive {
            orig_string.to_lowercase()
        } else {
            orig_string.to_string()
        };

        if !languages.is_empty() {
            let lang = line.get(lang_idx).unwrap();
            if !languages.iter().any(|l| l == lang) {
                continue;
            }
        }

        if !sources.is_empty() {
            let source = line.get(source_idx).unwrap();
            if !sources.iter().any(|s| s == source) {
                continue;
            }
        }

        let next_id = (concepts.len()) as u32;
        let (concept_number, _, _) = *concepts
            .entry(cui.into())
            .and_modify(|(_, existing_priority, concept)| {
                let new_priority = *ranks
                    .get(&RankSource {
                        sab: line.get(source_idx).unwrap().into(),
                        tty: line.get(tty_idx).unwrap().into(),
                    })
                    .unwrap_or(&0);

                if new_priority > *existing_priority {
                    *existing_priority = new_priority;
                    concept.preferred_name = orig_string.clone();
                }
            })
            .or_insert_with(|| {
                let string_priority = *ranks
                    .get(&RankSource {
                        sab: line.get(source_idx).unwrap().into(),
                        tty: line.get(tty_idx).unwrap().into(),
                    })
                    .unwrap_or(&0);

                (
                    next_id,
                    string_priority,
                    Concept {
                        cui: cui.into(),
                        preferred_name: orig_string,
                        types: SmallVec::new(), // TODO....
                    },
                )
            });

        string_to_number.entry(string).or_insert(concept_number);
    }

    // Now that we have the strings sorted (since we're using a BTree) we can build the FST.
    let output_fst_path = output_dir.join(STRINGS_FST_NAME);
    let output_fst_writer = std::io::BufWriter::new(std::fs::File::create(&output_fst_path)?);
    let mut fst_builder = MapBuilder::new(output_fst_writer)?;

    for (string, concept_number) in string_to_number {
        fst_builder.insert(string, concept_number as u64)?;
    }

    fst_builder.finish()?;

    let output_names_path = output_dir.join(CONCEPTS_LST_NAME);
    let mut output_names_writer = GzEncoder::new(
        std::io::BufWriter::new(std::fs::File::create(&output_names_path)?),
        flate2::Compression::default(),
    );
    let mut sorted_names = concepts
        .into_iter()
        .map(|(_, (id, _, concept))| (id, concept))
        .collect::<Vec<_>>();
    sorted_names.sort_unstable_by_key(|(id, _)| *id);

    for (_, concept) in &sorted_names {
        serde_json::to_writer(&mut output_names_writer, &concept)?;
        writeln!(output_names_writer)?;
    }

    let buf_writer = output_names_writer.finish()?;
    buf_writer.into_inner()?.flush()?;

    let meta = SearchIndexMeta {
        case_insensitive,
        languages,
        sources,
    };

    let mut meta_file = std::fs::File::create(output_dir.join(METADATA_NAME))?;
    serde_json::to_writer(&meta_file, &meta)?;
    meta_file.flush()?;

    Ok(())
}

#[derive(Hash, PartialEq, Eq)]
struct RankSource {
    sab: SmolStr,
    tty: SmolStr,
}

/// Read the ranks files and return the list of sources sorted by priority.
fn read_ranks(files: &Files) -> Result<HashMap<RankSource, u32>> {
    let mut mrrank = files.get_file_stream("MRRANK").unwrap();

    let rank_idx = mrrank.columns.iter().position(|c| c == "RANK").unwrap();
    let sab_idx = mrrank.columns.iter().position(|c| c == "SAB").unwrap();
    let tty_idx = mrrank.columns.iter().position(|c| c == "TTY").unwrap();

    let ranks = mrrank
        .reader
        .records()
        .map(|line| {
            let line = line?;
            let rank = line.get(rank_idx).unwrap().parse::<u32>()?;
            let sab = line.get(sab_idx).unwrap();
            let tty = line.get(tty_idx).unwrap();

            Ok((
                RankSource {
                    sab: sab.into(),
                    tty: tty.into(),
                },
                rank,
            ))
        })
        .collect::<Result<HashMap<_, _>>>()?;

    Ok(ranks)
}
