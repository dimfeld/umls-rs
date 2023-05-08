use std::path::PathBuf;

use eyre::Result;
use zip::ZipArchive;

use super::{ContainerLocation, RrfCsvReader};

pub struct FileIterator {
    pub(super) locations: Vec<ContainerLocation>,
    pub(super) location_index: usize,
    pub(super) zips: Vec<(usize, ZipArchive<std::fs::File>)>,
}

impl FileIterator {
    pub(super) fn new(locations: Vec<ContainerLocation>, zips: &[PathBuf]) -> Result<Self> {
        let mut found_containers = Vec::new();

        for location in &locations {
            let container = location.container as usize;
            if !found_containers.contains(&container) {
                found_containers.push(container);
            }
        }

        let zips = zips
            .iter()
            .enumerate()
            .filter(|(i, _)| found_containers.contains(i))
            .map(|(i, path)| {
                let file = std::fs::File::open(path)?;
                let zip = ZipArchive::new(file)?;
                Ok((i, zip))
            })
            .collect::<Result<Vec<(_, ZipArchive<std::fs::File>)>>>()?;

        Ok(Self {
            locations,
            location_index: 0,
            zips,
        })
    }

    /// Get the next file in the list. This is difficult to do with the normal
    /// Iterator trait because the ZipFile takes a mutable borrow to the ZipArchive.
    pub fn next(&mut self) -> Option<Result<RrfCsvReader<'_>>> {
        if self.location_index >= self.locations.len() {
            return None;
        }

        let file = self.locations[self.location_index];

        self.location_index += 1;

        let csv_reader = self.get_file_stream_for_location(file);
        Some(csv_reader)
    }

    fn get_file_stream_for_location(
        &mut self,
        location: ContainerLocation,
    ) -> Result<RrfCsvReader<'_>> {
        let container = self
            .zips
            .iter_mut()
            .find(|(i, _)| *i == location.container as usize)
            .unwrap();

        let file = container.1.by_index(location.index_in_container as usize)?;

        let decomp = flate2::read::GzDecoder::new(file);
        let csv_reader = csv::ReaderBuilder::new()
            .delimiter(b'|')
            .has_headers(false)
            .from_reader(decomp);

        Ok(csv_reader)
    }
}
