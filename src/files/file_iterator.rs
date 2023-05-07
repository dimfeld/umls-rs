use eyre::Result;

use super::{ContainerLocation, Files, RrfCsvReader};

pub struct FileIterator<'a> {
    pub(super) locations: Vec<ContainerLocation>,
    pub(super) location_index: usize,
    pub(super) files: &'a mut Files,
}

impl<'a> FileIterator<'a> {
    /// Get the next file in the list. This is difficult to do with the normal
    /// Iterator trait because the ZipFile takes a mutable borrow to the ZipArchive.
    pub fn next(&'a mut self) -> Option<Result<RrfCsvReader<'a>>> {
        if self.location_index >= self.locations.len() {
            return None;
        }

        let file = &self.locations[self.location_index];

        self.location_index += 1;

        let csv_reader = self.files.get_file_stream_for_location(*file);
        Some(csv_reader)
    }
}
