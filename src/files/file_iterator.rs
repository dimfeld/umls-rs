use std::path::PathBuf;

use concat_reader::read::ConcatReader;
use eyre::Result;
use smallvec::SmallVec;
use smol_str::SmolStr;

use super::{CarryOverColumns, MAX_CARRYOVER_VALUES};

pub type RrfReader =
    concat_reader::ConcatReader<Vec<flate2::bufread::GzDecoder<std::io::BufReader<std::fs::File>>>>;
pub type RrfCsvReader = csv::Reader<RrfReader>;

pub struct File {
    pub columns: Vec<String>,
    carry_over_columns: CarryOverColumns,
    reader: RrfCsvReader,
}

impl File {
    pub(super) fn new(file: &super::FileMetadata) -> Result<Self> {
        let reader = create_read_stream(&file.locations)?;
        Ok(Self {
            columns: file.columns.clone(),
            carry_over_columns: file.carry_over_columns.clone(),
            reader,
        })
    }

    pub fn records(&mut self) -> RrfRecordCarryover {
        RrfRecordCarryover::new(self.reader.records(), self.carry_over_columns.clone())
    }
}

type CarryOverValues = SmallVec<[(u8, SmolStr); MAX_CARRYOVER_VALUES]>;

pub struct RrfRecordCarryover<'a> {
    records: csv::StringRecordsIter<'a, RrfReader>,
    carry_over_columns: CarryOverColumns,
    last_values: CarryOverValues,
    last_ptr: SmolStr,
}

impl<'a> RrfRecordCarryover<'a> {
    fn new(
        records: csv::StringRecordsIter<'a, RrfReader>,
        carry_over_columns: CarryOverColumns,
    ) -> Self {
        Self {
            carry_over_columns,
            records,
            last_ptr: SmolStr::default(),
            last_values: SmallVec::new(),
        }
    }

    /// The PTR field uses a carryover compression method where it inherits the first two values of
    /// this hierarchy from the previous rows, but replaces the later segments. This function saves
    /// the first two segments using the following decisions:
    ///
    /// Zero dots: Not a valid value (usually empty)
    /// One dot: This is two segments, so save the whole thing.
    /// Two or more dots: Save up to the second period
    ///
    /// Adapted from line 1907 of
    /// plugins/gov.nih.nlm.umls.meta/src/gov/nih/nlm/umls/meta/io/RRFConceptInputStream.java
    fn save_ptr_values(&mut self, record: &csv::StringRecord) {
        let Some(ptr_idx) = self.carry_over_columns.ptr_column else {
            return;
        };

        let ptr = record.get(ptr_idx as usize).unwrap_or_default();
        let mut dots = ptr
            .char_indices()
            .filter(|&(_, c)| c == '.')
            .map(|(i, _)| i);

        let first_dot = dots.next();
        let second_dot = dots.next();

        let last_ptr = if let Some(second_dot) = second_dot {
            // Two dots, so save up to the second dot.
            &ptr[0..second_dot]
        } else if first_dot.is_some() {
            // Just one dot so save the whole string
            ptr
        } else {
            // No dots. The PTR field is empty.
            ""
        };

        self.last_ptr = SmolStr::from(last_ptr);
    }

    fn calculate_row_ptr_values(&self, record: &csv::StringRecord) -> Option<(u8, SmolStr)> {
        let Some(ptr_idx) = self.carry_over_columns.ptr_column else {
            return None;
        };

        let token = record.get(ptr_idx as usize).unwrap_or_default();
        let ptr_value = SmolStr::from(format!("{}.{}", self.last_ptr, &token[2..]));

        Some((ptr_idx, ptr_value))
    }
}

impl<'a> Iterator for RrfRecordCarryover<'a> {
    type Item = Result<RrfRecord, csv::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let record = match self.records.next() {
            Some(Ok(r)) => r,
            Some(Err(e)) => return Some(Err(e)),
            None => return None,
        };

        if self.carry_over_columns.columns.is_empty() {
            return Some(Ok(RrfRecord {
                carryover: SmallVec::new(),
                record,
            }));
        }

        // If we get here we need to process the carryover.
        if record
            .get(self.carry_over_columns.columns[0] as usize)
            .unwrap_or_default()
            .is_empty()
        {
            // This row should use the carried over values.
            let mut last_values = self.last_values.clone();

            if let Some(ptr) = self.calculate_row_ptr_values(&record) {
                last_values.push(ptr);
            };

            Some(Ok(RrfRecord {
                carryover: last_values,
                record,
            }))
        } else {
            // This row is a full row so we should save the new values.
            let carryover = self
                .carry_over_columns
                .columns
                .iter()
                .map(|&idx| {
                    let value = SmolStr::from(record.get(idx as usize).unwrap_or_default());
                    (idx, value)
                })
                .collect();

            self.last_values = carryover;
            self.save_ptr_values(&record);

            Some(Ok(RrfRecord {
                carryover: SmallVec::new(),
                record,
            }))
        }
    }
}

pub struct RrfRecord {
    carryover: SmallVec<[(u8, SmolStr); MAX_CARRYOVER_VALUES]>,
    record: csv::StringRecord,
}

impl RrfRecord {
    pub fn get(&self, i: usize) -> Option<&str> {
        if let Some(value) = self.carryover.iter().find(|(idx, _)| *idx == i as u8) {
            return Some(value.1.as_str());
        }

        self.record.get(i)
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

    Ok(create_csv_reader(ConcatReader::new(readers)))
}

pub(crate) fn create_csv_reader<R: std::io::Read>(r: R) -> csv::Reader<R> {
    csv::ReaderBuilder::new()
        .delimiter(b'|')
        .has_headers(false)
        .from_reader(r)
}
