use std::path::Path;
use std::{collections::BTreeMap, io::Write};

use ahash::{HashMap, HashMapExt};
use eyre::Result;
use flate2::write::GzEncoder;
use fst::MapBuilder;
use smallvec::SmallVec;
use smol_str::SmolStr;

use crate::files::Files;

use super::{Concept, ConceptCode, SearchIndexMeta, METADATA_NAME};
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
    let code_idx = mrconso.columns.iter().position(|c| c == "CODE").unwrap();

    // First build the lookups. We just do this in memory since in there are expected to be a few
    // tens of millions of strings.
    let mut string_to_number = BTreeMap::new();
    let mut concepts: HashMap<SmolStr, (u32, u32, Concept)> = HashMap::new();

    let convert_for_search = if case_insensitive {
        |s: &str| s.to_lowercase()
    } else {
        |s: &str| s.to_string()
    };

    for line in mrconso.reader.records() {
        let line = line?;
        let cui = line.get(cui_idx).unwrap();
        let code = line.get(code_idx).unwrap();
        let source = line.get(source_idx).unwrap();
        let orig_string = line.get(str_idx).unwrap();

        let string = convert_for_search(orig_string);
        if !languages.is_empty() {
            let lang = line.get(lang_idx).unwrap();
            if !languages.iter().any(|l| l == lang) {
                continue;
            }
        }

        if !sources.is_empty() && !sources.iter().any(|s| s == source) {
            continue;
        }

        let next_id = (concepts.len()) as u32;
        let (concept_number, _, _) = *concepts
            .entry(cui.into())
            .and_modify(|(_, existing_priority, concept)| {
                let new_priority = *ranks
                    .get(&RankSource {
                        sab: source.into(),
                        tty: line.get(tty_idx).unwrap().into(),
                    })
                    .unwrap_or(&0);

                if !code.is_empty() {
                    let concept_code = ConceptCode {
                        source: source.into(),
                        code: code.into(),
                    };

                    if !concept.codes.contains(&concept_code) {
                        concept.codes.push(concept_code);
                    }
                }

                if new_priority > *existing_priority {
                    *existing_priority = new_priority;
                    concept.preferred_name = SmolStr::from(orig_string);
                }
            })
            .or_insert_with(|| {
                let source = SmolStr::from(source);
                let rank_source_arg = RankSource {
                    sab: source,
                    tty: line.get(tty_idx).unwrap().into(),
                };
                let string_priority = *ranks.get(&rank_source_arg).unwrap_or(&0);

                let mut codes = SmallVec::new();
                if !code.is_empty() {
                    codes.push(ConceptCode {
                        source: rank_source_arg.sab,
                        code: code.into(),
                    });
                }

                // Add the CUI to the search index too.
                string_to_number.insert(convert_for_search(cui), next_id);

                (
                    next_id,
                    string_priority,
                    Concept {
                        cui: cui.into(),
                        preferred_name: SmolStr::from(orig_string),
                        codes,
                        types: SmallVec::new(), // TODO....
                        parents: SmallVec::new(),
                        children: SmallVec::new(),
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

    build_relationships(files, sorted_names.as_mut())?;

    for (_, mut concept) in sorted_names {
        concept.codes.sort_unstable();
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

/// Take the sorted list of concepts and add relationship data to it.
/// This modifies `concepts` in place.
fn build_relationships(files: &Files, concepts: &mut [(u32, Concept)]) -> Result<()> {
    let by_cui = concepts
        .iter()
        .enumerate()
        .map(|(i, c)| (c.1.cui.clone(), i))
        .collect::<HashMap<_, _>>();

    let mut mrrel = files.get_file_stream("MRREL")?;
    let cui_idx = mrrel.columns.iter().position(|c| c == "CUI1").unwrap();
    let rel_idx = mrrel.columns.iter().position(|c| c == "REL").unwrap();
    let cui2_idx = mrrel.columns.iter().position(|c| c == "CUI2").unwrap();

    for line in mrrel.reader.records() {
        let line = line?;
        let cui1 = line.get(cui_idx).unwrap();
        let rel = line.get(rel_idx).unwrap();
        let cui2 = line.get(cui2_idx).unwrap();

        if rel != "PAR" && rel != "CHD" || (cui1 == cui2) {
            continue;
        }

        let (i1, i2) = match by_cui.get(cui1).zip(by_cui.get(cui2)) {
            Some((i1, i2)) => (*i1 as u32, *i2 as u32),
            None => continue,
        };

        let is_parent = rel == "PAR";

        {
            let concept1 = &mut concepts[i1 as usize].1;
            if is_parent && !concept1.parents.contains(&i2) {
                concept1.parents.push(i2);
            } else if !is_parent && !concept1.children.contains(&i2) {
                concept1.children.push(i2);
            }
        }

        {
            let concept2 = &mut concepts[i2 as usize].1;
            if is_parent && !concept2.children.contains(&i1) {
                concept2.children.push(i1);
            } else if !is_parent && !concept2.parents.contains(&i1) {
                concept2.parents.push(i1);
            }
        }
    }

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
