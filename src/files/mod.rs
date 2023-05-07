use eyre::Result;
use zip::ZipArchive;

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
                let file = container.zip.by_index_raw(i)?;
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
}
