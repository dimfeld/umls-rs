mod file_iterator;
mod find_files;
mod schema;

use std::path::{Path, PathBuf};

use ahash::{HashMap, HashMapExt};
use eyre::Result;

pub use schema::*;

use self::{file_iterator::FileIterator, find_files::find_data_files};

pub type RrfReader = flate2::bufread::GzDecoder<std::io::BufReader<std::fs::File>>;
pub type RrfCsvReader = csv::Reader<RrfReader>;

#[derive(Clone, Default)]
struct File {
    locations: Vec<PathBuf>,
    columns: Vec<String>,
}

pub struct Files {
    files: HashMap<String, File>,
}

impl Files {
    pub fn new(dir: &Path) -> Result<Self> {
        let dir = find_data_files(dir)?;

        let mut files = HashMap::new();

        let entries = std::fs::read_dir(&dir)?;
        for file in entries {
            let file = file?;
            if !file.metadata()?.is_file() {
                continue;
            }

            let name = file.file_name();
            let base_name = name
                .to_string_lossy()
                .split('.')
                .next()
                .unwrap_or_default()
                .to_string();

            files
                .entry(base_name)
                .or_insert_with(File::default)
                .locations
                .push(file.path());
        }

        let mut slf = Self { files };
        slf.init_file_columns()?;

        Ok(slf)
    }

    fn get_file_stream(&self, filename: &str) -> Result<FileIterator> {
        let locations = self
            .files
            .get(filename)
            .ok_or_else(|| eyre::eyre!("No file named {}", filename,))?;

        FileIterator::new(locations)
    }

    fn init_file_columns(&mut self) -> Result<()> {
        let files_list = self.get_file_stream("MRFILES")?;
        for f in files_list {
            let mut f = f?;
            for line in f.records() {
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
                }
            }
        }

        Ok(())
    }
}

fn create_read_stream(path: &Path) -> Result<RrfCsvReader> {
    let file = std::fs::File::open(path)?;
    let bufreader = std::io::BufReader::new(file);
    let decomp = flate2::bufread::GzDecoder::new(bufreader);
    let csv_reader = csv::ReaderBuilder::new()
        .delimiter(b'|')
        .has_headers(false)
        .from_reader(decomp);

    Ok(csv_reader)
}
