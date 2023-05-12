use super::Files;
use ahash::{HashMap, HashMapExt};
use eyre::Result;

#[derive(Debug)]
pub struct FileDescription {
    pub filename: String,
    pub description: String,
    pub columns: Vec<Column>,
    pub num_rows: usize,
    pub num_bytes: usize,
}

#[derive(Debug)]
pub struct Column {
    pub name: String,
    pub description: String,
}

#[derive(Debug)]
pub struct UmlsSource {
    pub name: String,
    pub family: String,
    pub language: String,
    pub abbreviation: String,
}

impl Files {
    pub fn read_sources(&self) -> Result<Vec<UmlsSource>> {
        let mut mrsab = self.get_file_stream("MRSAB")?;
        let mut sources = Vec::new();

        let son_idx = mrsab.columns.iter().position(|c| c == "SON").unwrap();
        let lat_idx = mrsab.columns.iter().position(|c| c == "LAT").unwrap();
        let fam_idx = mrsab.columns.iter().position(|c| c == "SF").unwrap();
        let rsab_idx = mrsab.columns.iter().position(|c| c == "RSAB").unwrap();

        for line in mrsab.records() {
            let line = line?;
            let son = line.get(son_idx).unwrap();
            let lat = line.get(lat_idx).unwrap();
            let fam = line.get(fam_idx).unwrap();
            let rsab = line.get(rsab_idx).unwrap();

            sources.push(UmlsSource {
                name: son.to_string(),
                family: fam.to_string(),
                language: lat.to_string(),
                abbreviation: rsab.to_string(),
            })
        }

        Ok(sources)
    }

    pub fn read_schema_descriptions(&self) -> Result<Vec<FileDescription>> {
        let mut column_descs = HashMap::new();

        {
            let mut mrcols = self.get_file_stream("MRCOLS")?;

            for line in mrcols.records() {
                let line = line?;
                let col_name = line.get(0).unwrap_or_default();
                let desc = line.get(1).unwrap_or_default();
                let file_name = line.get(6).unwrap_or_default();

                column_descs.insert(
                    (file_name.to_string(), col_name.to_string()),
                    desc.to_string(),
                );
            }
        }

        let mut mrfiles = self.get_file_stream("MRFILES")?;
        let mut files = Vec::new();
        for line in mrfiles.records() {
            let line = line?;
            let filename = line.get(0).unwrap_or_default().to_string();
            let description = line.get(1).unwrap_or_default();
            let columns = line.get(2).unwrap_or_default();
            let num_rows = line.get(4).unwrap_or_default();
            let num_bytes = line.get(5).unwrap_or_default();

            let columns = columns
                .split(',')
                .map(|col| {
                    let col = col.to_string();
                    let desc = column_descs
                        .remove(&(filename.clone(), col.clone()))
                        .unwrap_or_default();
                    Column {
                        name: col,
                        description: desc,
                    }
                })
                .collect();

            files.push(FileDescription {
                filename: filename.to_string(),
                description: description.to_string(),
                columns,
                num_rows: num_rows.parse()?,
                num_bytes: num_bytes.parse()?,
            })
        }

        Ok(files)
    }
}
