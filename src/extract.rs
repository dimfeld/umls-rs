use eyre::Result;
use std::path::Path;

pub fn extract_metathesaurus<'a>(mut input_path: &'a Path, output_path: &'a Path) -> Result<()> {
    let base_input = std::fs::File::open(input_path)?;
    let base_meta = base_input.metadata()?;
    if base_meta.is_file() {
        let mut archive = zip::ZipArchive::new(base_input)?;
        archive.extract(output_path)?;
        // Now that we've extracted the files, start working in the output path.
        input_path = output_path;
    }

    let containers = std::fs::read_dir(input_path)?
        .filter_map(|path| path.ok())
        .filter(|path| path.file_name().to_string_lossy().ends_with("-meta.nlm"));

    // Extract the data out to get the GZ files.
    for container in containers {
        let f = std::fs::File::open(&container.path())?;
        let mut zip = zip::ZipArchive::new(f)?;
        zip.extract(output_path)?;
    }

    Ok(())
}
