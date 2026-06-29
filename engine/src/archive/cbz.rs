//! CBZ (ZIP-based comic book archive) extraction.

use anyhow::{Context, Result};
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

use super::{ArchiveEntry, is_image_filename};

/// Extract all image entries from a CBZ (ZIP) archive.
///
/// The CBZ format is simply a ZIP file containing image files
/// (typically named sequentially: page-001.jpg, page-002.jpg, etc.).
pub fn extract_cbz<P: AsRef<Path>>(path: P) -> Result<Vec<ArchiveEntry>> {
    let file = std::fs::File::open(path.as_ref())
        .context("Failed to open CBZ file")?;

    let mut archive = ZipArchive::new(file)
        .context("Failed to read CBZ as ZIP archive")?;

    let mut entries = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .context(format!("Failed to access file index {} in CBZ", i))?;

        let file_name = file.name().to_string();

        // Skip directories and non-image files
        if file.is_dir() || !is_image_filename(&file_name) {
            continue;
        }

        let mut data = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut data)
            .context(format!("Failed to read {} from CBZ", file_name))?;

        entries.push(ArchiveEntry {
            path: Path::new(&file_name).to_path_buf(),
            data,
            page_index: 0, // Will be assigned after sorting in the parent module
        });
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_empty_cbz() {
        // Create a minimal valid ZIP in memory
        let zip_data = create_minimal_zip();
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_empty.cbz");
        std::fs::write(&temp_file, &zip_data).unwrap();

        let entries = extract_cbz(&temp_file).unwrap();
        assert!(entries.is_empty());
        std::fs::remove_file(&temp_file).unwrap();
    }

    fn create_minimal_zip() -> Vec<u8> {
        // Build a minimal valid ZIP file with no files
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(&mut buf);
        zip.finish().unwrap();
        buf.into_inner()
    }
}
