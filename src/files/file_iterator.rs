use std::path::PathBuf;

use concat_reader::read::ConcatReader;
use eyre::Result;

pub type RrfReader = flate2::bufread::GzDecoder<std::io::BufReader<std::fs::File>>;
pub type RrfCsvReader = csv::Reader<concat_reader::ConcatReader<Vec<RrfReader>>>;

pub struct File {
    pub columns: Vec<String>,
    pub reader: RrfCsvReader,
}

impl File {
    pub(super) fn new(file: &super::FileMetadata) -> Result<Self> {
        let reader = create_read_stream(&file.locations)?;
        Ok(Self {
            columns: file.columns.clone(),
            reader,
        })
    }
}

/// Create a CSV decoder stream that concatenates the decompressed output from the
/// list of .gz files.
fn create_read_stream(path: &[PathBuf]) -> Result<RrfCsvReader> {
    let readers = path
        .iter()
        .map(|path| {
            let file = std::fs::File::open(path)?;
            let bufreader = std::io::BufReader::new(file);
            let decomp = flate2::bufread::GzDecoder::new(bufreader);
            Ok(decomp)
        })
        .collect::<Result<Vec<_>>>()?;

    let concatted = ConcatReader::new(readers);

    let csv_reader = csv::ReaderBuilder::new()
        .delimiter(b'|')
        .has_headers(false)
        .from_reader(concatted);

    Ok(csv_reader)
}
