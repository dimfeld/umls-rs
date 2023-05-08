use std::path::PathBuf;

use eyre::Result;

use super::{create_read_stream, RrfCsvReader};

pub struct FileIterator {
    pub columns: Vec<String>,
    locations: Vec<PathBuf>,
    location_index: usize,
}

impl FileIterator {
    pub(super) fn new(file: &super::File) -> Result<Self> {
        Ok(Self {
            columns: file.columns.clone(),
            locations: file.locations.clone(),
            location_index: 0,
        })
    }
}

impl Iterator for FileIterator {
    type Item = Result<RrfCsvReader>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.location_index >= self.locations.len() {
            return None;
        }

        let file = &self.locations[self.location_index];

        self.location_index += 1;

        let csv_reader = create_read_stream(file);
        Some(csv_reader)
    }
}
