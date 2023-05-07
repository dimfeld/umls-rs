mod schema;

use eyre::Result;
use zip::{read::ZipFile, ZipArchive};

pub use schema::*;

pub struct Container {
    pub filename: String,
    zip: ZipArchive<std::fs::File>,
}

pub struct FileLocation {
    pub name: String,
    pub container: usize,
    pub index_in_container: usize,
}

pub struct Files {
    pub dir: String,
    pub files: Vec<FileLocation>,
    pub containers: Vec<Container>,
}

impl Files {
    pub fn new(dir: String) -> Result<Self> {
        let mut containers = std::fs::read_dir(&dir)?
            .filter_map(|path| path.ok())
            .filter(|path| path.file_name().to_string_lossy().ends_with("-meta.nlm"))
            .map(|path| {
                let f = std::fs::File::open(path.path())?;
                let zip = ZipArchive::new(f)?;

                Ok(Container {
                    filename: path.file_name().to_string_lossy().to_string(),
                    zip,
                })
            })
            .collect::<Result<Vec<Container>>>()?;

        if containers.is_empty() {
            return Err(eyre::eyre!("No UMLS .nlm files found in {}", dir));
        }

        let mut files = Vec::new();

        for (cidx, container) in containers.iter_mut().enumerate() {
            for i in 0..container.zip.len() {
                let file = container.zip.by_index(i)?;
                if file.is_file() {
                    let name = std::path::Path::new(file.name())
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();

                    files.push(FileLocation {
                        name,
                        container: cidx,
                        index_in_container: i,
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

    fn get_file_stream<'a>(
        &'a mut self,
        filename: &str,
    ) -> Result<csv::Reader<impl std::io::Read + 'a>> {
        Self::internal_get_file_stream(&self.files, &mut self.containers, filename)
    }

    /// Separated internals so that we can use this during construction without having
    /// the full object yet.
    fn internal_get_file_stream<'a>(
        files: &'_ [FileLocation],
        containers: &'a mut [Container],
        filename: &str,
    ) -> Result<csv::Reader<impl std::io::Read + 'a>> {
        let location = files
            .iter()
            .find(|f| f.name == filename)
            .ok_or_else(|| eyre::eyre!("Could not find file {} in dataset.", filename))?;
        let container = &mut containers[location.container];
        let file = container.zip.by_index(location.index_in_container)?;

        let decomp = flate2::read::GzDecoder::new(file);
        let csv_reader = csv::ReaderBuilder::new()
            .delimiter(b'|')
            .has_headers(false)
            .from_reader(decomp);

        Ok(csv_reader)
    }
}
