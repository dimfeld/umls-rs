mod file_iterator;
mod schema;

use ahash::{HashMap, HashMapExt};
use eyre::Result;
use zip::ZipArchive;

pub use schema::*;

use self::file_iterator::FileIterator;

pub type RrfCsvReader<'a> = csv::Reader<flate2::read::GzDecoder<zip::read::ZipFile<'a>>>;

#[derive(Copy, Clone)]
struct ContainerLocation {
    pub container: u16,
    pub index_in_container: u16,
}

pub struct Files {
    pub dir: String,
    files: HashMap<String, Vec<ContainerLocation>>,
    containers: Vec<ZipArchive<std::fs::File>>,
}

impl Files {
    pub fn new(dir: String) -> Result<Self> {
        let mut containers = std::fs::read_dir(&dir)?
            .filter_map(|path| path.ok())
            .filter(|path| path.file_name().to_string_lossy().ends_with("-meta.nlm"))
            .map(|path| {
                let f = std::fs::File::open(path.path())?;
                let zip = ZipArchive::new(f)?;

                Ok(zip)
            })
            .collect::<Result<Vec<_>>>()?;

        if containers.is_empty() {
            return Err(eyre::eyre!("No UMLS .nlm files found in {}", dir));
        }

        let mut files = HashMap::new();

        for (cidx, container) in containers.iter_mut().enumerate() {
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
                        .or_insert_with(Vec::new)
                        .push(ContainerLocation {
                            container: cidx as u16,
                            index_in_container: i as u16,
                        });
                }
            }
        }

        Ok(Self {
            dir,
            files,
            containers,
        })
    }

    fn get_file_stream<'a>(&'a mut self, filename: &str) -> Result<FileIterator<'a>> {
        let locations = self
            .files
            .get(filename)
            .ok_or_else(|| eyre::eyre!("No file named {}", filename,))?;

        Ok(FileIterator {
            locations: locations.clone(),
            location_index: 0,
            files: self,
        })
    }

    /// Separated internals so that we can use this during construction without having
    /// the full object yet.
    fn get_file_stream_for_location(
        &mut self,
        location: ContainerLocation,
    ) -> Result<RrfCsvReader<'_>> {
        let container = &mut self.containers[location.container as usize];
        let file = container.by_index(location.index_in_container as usize)?;

        let decomp = flate2::read::GzDecoder::new(file);
        let csv_reader = csv::ReaderBuilder::new()
            .delimiter(b'|')
            .has_headers(false)
            .from_reader(decomp);

        Ok(csv_reader)
    }
}
