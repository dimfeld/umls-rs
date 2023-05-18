mod file_iterator;
mod find_files;
mod schema;

use std::path::{Path, PathBuf};

use ahash::{HashMap, HashMapExt};
use eyre::Result;
use itertools::Itertools;
use smallvec::{smallvec, SmallVec};

pub(crate) use file_iterator::create_csv_reader;
pub use schema::*;

// This should be one more than the maximum number of columns in get_carry_over_columns,
// plus one to account for the PTR column.
const MAX_CARRYOVER_VALUES: usize = 6;

use self::{file_iterator::File, find_files::find_data_files};

#[derive(Clone, Default)]
struct FileMetadata {
    locations: Vec<PathBuf>,
    columns: Vec<String>,
    carry_over_columns: CarryOverColumns,
}

pub struct Files {
    files: HashMap<String, FileMetadata>,
    pub(crate) base_dir: PathBuf,
}

impl Files {
    pub fn new(dir: &Path) -> Result<Self> {
        let dir = find_data_files(dir)?;

        let mut files = HashMap::new();

        let entries = glob::glob(&format!("{}/*/*.gz", dir.display()))?;
        for file in entries {
            let file = file?;
            if !file.metadata()?.is_file() {
                continue;
            }

            let name = file.file_name().unwrap_or_default().to_string_lossy();
            let base_name = name.split('.').next().unwrap_or_default().to_string();

            files
                .entry(base_name)
                .or_insert_with(FileMetadata::default)
                .locations
                .push(file);
        }

        // read_dir may not return the files in order, so sort them.
        for (_, file) in files.iter_mut() {
            file.locations.sort_unstable();
        }

        let mut slf = Self {
            files,
            base_dir: dir,
        };
        slf.init_file_columns()?;

        Ok(slf)
    }

    pub fn get_file_stream(&self, filename: &str) -> Result<File> {
        let locations = self
            .files
            .get(filename)
            .ok_or_else(|| eyre::eyre!("No file named {}", filename,))?;

        File::new(locations)
    }

    fn init_file_columns(&mut self) -> Result<()> {
        let mut mrfiles = self.get_file_stream("MRFILES")?;
        for line in mrfiles.records() {
            let line = line?;
            let filename = line.get(0).unwrap_or_default();
            let basename = filename.split('.').next().unwrap_or_default();
            let columns = line.get(2).unwrap_or_default();

            let columns = columns
                .split(',')
                .map(|s| s.to_string())
                .collect::<Vec<_>>();

            if let Some(f) = self.files.get_mut(basename) {
                f.columns = columns;
                f.carry_over_columns = get_carry_over_columns(basename, &f.columns);
            }
        }

        Ok(())
    }
}

#[derive(Clone, Default)]
struct CarryOverColumns {
    ptr_column: Option<u8>,
    columns: SmallVec<[u8; MAX_CARRYOVER_VALUES]>,
}

/// The raw data is compressed in a sort-of-RLE fashion, where if the first column is absent it
/// means to carry over some values from the previous rows. This is not documented anywhere but is
/// performed in the source code for Metamorphosys at
/// plugins/gov.nih.nlm.umls.meta/src/gov/nih/nlm/umls/meta/io/RRFConceptInputStream.java
fn get_carry_over_columns(basename: &str, columns: &[String]) -> CarryOverColumns {
    let (ptr, column_names): (bool, SmallVec<[&str; MAX_CARRYOVER_VALUES]>) = match basename {
        "MRSAT" => (false, smallvec!["CUI", "METAUI", "STYPE", "SAB"]),
        "MRHIER" => (true, smallvec!["CUI", "AUI", "SAB", "RELA"]),
        "MRREL" => (false, smallvec!["CUI1", "AUI1", "STYPE1", "STYPE2", "SAB"]),
        _ => (false, smallvec![]),
    };

    let column_idxs = column_names
        .into_iter()
        .filter_map(|name| columns.iter().position(|c| c == name).map(|p| p as u8))
        .sorted()
        .collect();

    let ptr_column = if ptr {
        columns.iter().position(|c| *c == "PTR").map(|p| p as u8)
    } else {
        None
    };

    CarryOverColumns {
        ptr_column,
        columns: column_idxs,
    }
}

/* Special PTR handling in MRHIER
 *  tokens[6] = prevPtr1 + "." + prevPtr2 + "." + tokens[6].substring(2);
 * And then to calculate
                  // Set prevPtr1 as the first AUI of the chain
                    if (tokens[6].indexOf(".") != -1) {
                        prevPtr1 = tokens[6].substring(0, tokens[6].indexOf("."));
                        // Set prevPtr2 as the second AUI of the chain. Must determine if
                        // there are 2 or more than 2.
                        final int firstDot = tokens[6].indexOf(".");
                        if (tokens[6].indexOf(".", firstDot+1) != -1)
                            prevPtr2 =
                                    tokens[6].substring(firstDot + 1, tokens[6].indexOf(".",
                                            firstDot + 1));
                        else
                            prevPtr2 = tokens[6].substring(firstDot + 1);
                    } else {
                        prevPtr1="";
                        prevPtr2="";
                    }
*/
