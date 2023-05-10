use std::path::Path;
use std::{collections::BTreeMap, io::Write};

use ahash::{HashMap, HashMapExt};
use eyre::Result;
use fst::MapBuilder;

use crate::files::Files;

use super::{SearchIndexMeta, METADATA_NAME};
use super::{CONCEPTS_LST_NAME, STRINGS_FST_NAME};

pub struct IndexBuilderOptions<'a> {
    pub output_dir: &'a Path,
    pub files: &'a Files,
    pub case_insensitive: bool,
    /// The languages to include in the index. If empty, all languages will be included.
    pub languages: Vec<String>,
    /// The sources to include in the index. If empty, all sources are included.
    pub sources: Vec<String>,
}

pub fn build_string_search(options: IndexBuilderOptions) -> Result<()> {
    let IndexBuilderOptions {
        output_dir,
        files,
        case_insensitive,
        languages,
        sources,
    } = options;

    let mut mrconso = files.get_file_stream("MRCONSO")?;

    let cui_idx = mrconso.columns.iter().position(|c| c == "CUI").unwrap();
    let lang_idx = mrconso.columns.iter().position(|c| c == "LAT").unwrap();
    let str_idx = mrconso.columns.iter().position(|c| c == "STR").unwrap();
    let source_idx = mrconso.columns.iter().position(|c| c == "SAB").unwrap();

    // First build the lookups. We just do this in memory since in there are expected to be a few
    // tens of millions of strings.
    let mut string_to_number = BTreeMap::new();
    let mut concept_to_number = HashMap::new();

    for line in mrconso.reader.records() {
        let line = line?;
        let cui = line.get(cui_idx).unwrap();
        let string = line.get(str_idx).unwrap();

        let string = if case_insensitive {
            string.to_lowercase()
        } else {
            string.to_string()
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

        string_to_number.entry(string).or_insert_with(|| {
            let next_id = (concept_to_number.len()) as u32;
            let concept_number = *concept_to_number.entry(cui.to_string()).or_insert(next_id);
            concept_number
        });
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
    let mut output_names_writer =
        std::io::BufWriter::new(std::fs::File::create(&output_names_path)?);
    let mut sorted_names = concept_to_number.into_iter().collect::<Vec<_>>();
    sorted_names.sort_unstable_by_key(|(_, id)| *id);

    for (name, _) in sorted_names {
        writeln!(output_names_writer, "{}", name)?;
    }

    output_names_writer.into_inner()?.flush()?;

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
