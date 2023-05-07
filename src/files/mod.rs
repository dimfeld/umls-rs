use eyre::Result;
use zip::ZipArchive;

pub struct Container {
    pub container: String,
    pub files: Vec<String>,
    zip: ZipArchive<std::fs::File>,
}

pub struct Files {
    pub dir: String,
    pub containers: Vec<Container>,
}

impl Files {
    pub fn new(dir: String) -> Result<Self> {
        let containers = std::fs::read_dir(&dir)?
            .filter_map(|path| path.ok())
            .filter(|path| path.file_name().to_string_lossy().ends_with("-meta.nlm"))
            .map(|path| {
                let f = std::fs::File::open(path.path())?;
                let mut zip = ZipArchive::new(f)?;

                let mut files = Vec::new();
                for i in 0..zip.len() {
                    let file = zip.by_index_raw(i)?;
                    if file.is_file() {
                        files.push(file.name().to_string());
                    }
                }

                Ok(Container {
                    container: path.file_name().to_string_lossy().to_string(),
                    zip,
                    files,
                })
            })
            .collect::<Result<Vec<Container>>>()?;

        if containers.is_empty() {
            return Err(eyre::eyre!("No UMLS .nlm files found in {}", dir));
        }

        Ok(Self { dir, containers })
    }
}
