use eyre::Result;
use fst::IntoStreamer;
use std::{
    io::{BufRead, Read},
    path::Path,
};

pub mod build;

pub struct Searcher {
    pub concepts: Vec<String>,
    pub index: fst::Map<Vec<u8>>,
}

const STRINGS_FST_NAME: &str = "umls_search.strings.fst";
const CONCEPTS_LST_NAME: &str = "umls_search.concepts.lst";

impl Searcher {
    pub fn new(base_dir: &Path) -> Result<Searcher> {
        let concepts_lst_path = base_dir.join(CONCEPTS_LST_NAME);

        let concepts_file = std::fs::File::open(concepts_lst_path)?;
        let concepts_reader = std::io::BufReader::new(concepts_file);
        let concepts = concepts_reader.lines().collect::<Result<Vec<_>, _>>()?;

        let strings_fst_path = base_dir.join(STRINGS_FST_NAME);
        let mut strings = std::fs::File::open(strings_fst_path)?;
        let mut fst_contents = Vec::new();
        strings.read_to_end(&mut fst_contents)?;

        let index = fst::Map::new(fst_contents)?;

        Ok(Self { concepts, index })
    }

    pub fn search(&self, word: &str) -> Option<u64> {
        self.index.get(word.as_bytes())
    }

    pub fn fuzzy_search(
        &self,
        word: &str,
        levenshtein: u32,
    ) -> Result<fst::map::StreamWithState<'_, fst::automaton::Levenshtein>> {
        let auto = fst::automaton::Levenshtein::new_with_limit(word, levenshtein, 1_000_000)?;
        Ok(self.index.search_with_state(auto).into_stream())
    }
}
