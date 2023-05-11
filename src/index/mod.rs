use eyre::Result;
use flate2::read::GzDecoder;
use fst::{IntoStreamer, Streamer};
use regex_automata::dense;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use smol_str::SmolStr;
use std::{
    borrow::Cow,
    io::{BufRead, Read},
    path::Path,
};

pub mod build;
pub mod score;

#[derive(Serialize, Deserialize, Debug)]
pub struct SearchIndexMeta {
    pub case_insensitive: bool,
    pub languages: Vec<SmolStr>,
    pub sources: Vec<SmolStr>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ConceptCode {
    pub source: SmolStr,
    pub code: SmolStr,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Concept {
    pub cui: SmolStr,
    pub preferred_name: SmolStr,
    pub types: SmallVec<[u16; 4]>,
    pub codes: SmallVec<[ConceptCode; 4]>,
}

pub struct SemanticType {
    pub tui: SmolStr,
    pub name: SmolStr,
    pub tree_number: SmolStr,
    pub description: String,
}

pub struct Index {
    pub meta: SearchIndexMeta,
    concepts: Vec<Concept>,
    index: fst::Map<Vec<u8>>,
}

const METADATA_NAME: &str = "umls_search.metadata.json";
const STRINGS_FST_NAME: &str = "umls_search.strings.fst";
const CONCEPTS_LST_NAME: &str = "umls_search.concepts.ndjson.gz";

impl Index {
    pub fn new(base_dir: &Path) -> Result<Index> {
        let meta_path = base_dir.join(METADATA_NAME);
        let meta_file = std::fs::File::open(meta_path)?;
        let meta = serde_json::from_reader(meta_file)?;

        let concepts_lst_path = base_dir.join(CONCEPTS_LST_NAME);

        let concepts_file = std::fs::File::open(concepts_lst_path)?;
        let concepts_reader = std::io::BufReader::new(GzDecoder::new(concepts_file));
        let concepts = concepts_reader
            .lines()
            .map(|line| Ok::<Concept, eyre::Report>(serde_json::from_str(&line?)?))
            .collect::<Result<Vec<_>, _>>()?;

        let strings_fst_path = base_dir.join(STRINGS_FST_NAME);
        let mut strings = std::fs::File::open(strings_fst_path)?;
        let mut fst_contents = Vec::new();
        strings.read_to_end(&mut fst_contents)?;

        let index = fst::Map::new(fst_contents)?;

        Ok(Self {
            meta,
            concepts,
            index,
        })
    }

    /// Get the string associated with an ID returned from the search function.
    pub fn concept_id(&self, id: u64) -> &Concept {
        &self.concepts[id as usize]
    }

    /// Find a word in a case-insensitive fashion. For indexes built in case-insensitive mode,
    /// this does a simple get. Otherwise it builds an automata that searches the index in a
    /// case-insensitive fashion.
    pub fn search(&self, word: &str) -> Result<Option<u64>> {
        if self.meta.case_insensitive {
            let word = word.to_lowercase();
            Ok(self.search_exact(&word))
        } else {
            let pattern = format!("(?i){}", word);
            self.search_regex(&pattern)
        }
    }

    /// Find an exact match for the given word.
    pub fn search_exact(&self, word: &str) -> Option<u64> {
        self.index.get(word.as_bytes())
    }

    /// Search for a word using a regex pattern.
    pub fn search_regex(&self, word: &str) -> Result<Option<u64>> {
        let dfa = dense::Builder::new().anchored(true).build(word)?;
        let result = self.index.search(&dfa).into_stream().next().map(|i| i.1);
        Ok(result)
    }

    pub fn fuzzy_search(
        &self,
        word: &str,
        levenshtein: u32,
    ) -> Result<fst::map::StreamWithState<'_, fst::automaton::Levenshtein>> {
        let word = if self.meta.case_insensitive {
            Cow::Owned(word.to_lowercase())
        } else {
            Cow::Borrowed(word)
        };

        let auto = fst::automaton::Levenshtein::new_with_limit(&word, levenshtein, 1_000_000)?;
        Ok(self.index.search_with_state(auto).into_stream())
    }
}
