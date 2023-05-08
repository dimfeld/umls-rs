mod file_iterator;
mod schema;

use std::path::PathBuf;

use ahash::{HashMap, HashMapExt};
use eyre::Result;
use zip::{read::ZipFile, ZipArchive};

pub use schema::*;

use self::file_iterator::FileIterator;

pub type RrfCsvReader<'a> = csv::Reader<flate2::read::GzDecoder<zip::read::ZipFile<'a>>>;

#[derive(Copy, Clone)]
struct ContainerLocation {
    pub container: u16,
    pub index_in_container: u16,
}

#[derive(Clone, Default)]
struct File {
    locations: Vec<ContainerLocation>,
    columns: Vec<String>,
}

pub struct Files {
    files: HashMap<String, File>,
    containers: Vec<PathBuf>,
}

impl Files {
    pub fn new(dir: &str) -> Result<Self> {
        let mut containers = std::fs::read_dir(dir)?
            .filter_map(|path| path.ok())
            .filter(|path| path.file_name().to_string_lossy().ends_with("-meta.nlm"))
            .map(|path| {
                let path = path.path();
                let f = std::fs::File::open(&path)?;
                let zip = ZipArchive::new(f)?;

                Ok((path, zip))
            })
            .collect::<Result<Vec<_>>>()?;

        if containers.is_empty() {
            return Err(eyre::eyre!("No UMLS .nlm files found in {}", dir));
        }

        let mut files = HashMap::new();

        for (cidx, (_, container)) in containers.iter_mut().enumerate() {
            for i in 0..container.len() {
                let file = container.by_index(i)?;
                if file.is_file() {
                    let name = std::path::Path::new(file.name())
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();

                    let base_name = name.split('.').next().unwrap_or_default().to_string();

                    files
                        .entry(base_name)
                        .or_insert_with(File::default)
                        .locations
                        .push(ContainerLocation {
                            container: cidx as u16,
                            index_in_container: i as u16,
                        });
                }
            }
        }

        let containers = containers
            .into_iter()
            .map(|(path, _)| path)
            .collect::<Vec<_>>();

        let mut slf = Self { files, containers };
        slf.init_file_columns()?;

        Ok(slf)
    }

    fn get_file_stream(&mut self, filename: &str) -> Result<FileIterator> {
        let locations = self
            .files
            .get(filename)
            .ok_or_else(|| eyre::eyre!("No file named {}", filename,))?;

        FileIterator::new(locations.clone(), &self.containers)
    }

    fn init_file_columns(&mut self) -> Result<()> {
        let mut files_list = self.get_file_stream("MRFILES")?;
        while let Some(f) = files_list.next() {
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

fn create_read_stream(file: ZipFile<'_>) -> RrfCsvReader<'_> {
    let decomp = flate2::read::GzDecoder::new(file);
    let csv_reader = csv::ReaderBuilder::new()
        .delimiter(b'|')
        .has_headers(false)
        .from_reader(decomp);

    csv_reader
}
