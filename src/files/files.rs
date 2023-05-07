use eyre::Result;
use zip::ZipArchive;

pub struct Container {
    container: String,
    zip: ZipArchive<std::fs::File>,
    files: Vec<String>,
}

pub struct Files {
    pub dir: String,
    pub containers: Vec<Container>,
}

impl Files {
    pub fn new(dir: String) -> Result<Self> {
        let containers = std::fs::read_dir(&dir)?
            .filter_map(|path| path.ok())
            .filter(|path| path.file_name().to_string_lossy().ends_with(".nlm"))
            .map(|path| {
                let f = std::fs::File::open(path.path())?;
                let zip = ZipArchive::new(f)?;
                let files = zip
                    .file_names()
                    .map(|f| f.to_string())
                    .collect::<Vec<String>>();

                Ok(Container {
                    container: path.file_name().to_string_lossy().to_string(),
                    zip,
                    files,
                })
            })
            .collect::<Result<Vec<Container>>>()?;

        Ok(Self { dir, containers })
    }
}
