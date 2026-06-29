//! CB7 (7z-based comic book archive) extraction.
//!
//! Uses the `sevenz-rust` crate to extract images from 7z archives.
//! CB7 is simply a 7z archive containing image files.

use anyhow::{Context, Result};
use sevenz_rust::{Password, SevenZReader};
use std::path::Path;

use super::{ArchiveEntry, is_image_filename};

/// Extract all image entries from a CB7 (7z) archive.
///
/// Iterates through all entries in the 7z archive, filters for
/// image files by extension, and reads their decompressed contents.
pub fn extract_cb7<P: AsRef<Path>>(path: P) -> Result<Vec<ArchiveEntry>> {
    let path = path.as_ref();

    let file = std::fs::File::open(path)
        .context("Failed to open CB7 file")?;
    let file_len = file
        .metadata()
        .context("Failed to get CB7 file metadata")?
        .len();

    let mut reader = SevenZReader::new(file, file_len, Password::empty())
        .map_err(|e| anyhow::anyhow!("Failed to read CB7 archive: {}", e))?;

    // Collect the names of image entries to match during iteration
    let image_names: Vec<String> = reader
        .archive()
        .files
        .iter()
        .filter(|e| !e.is_directory() && is_image_filename(e.name()))
        .map(|e| e.name().to_string())
        .collect();

    let mut entries = Vec::new();

    // Process the archive — for_each_entries calls the closure for every entry
    // with a streaming reader for its decompressed content.
    reader
        .for_each_entries(|entry, entry_reader| {
            let name = entry.name();
            if image_names.iter().any(|n| n == name) {
                let mut data = Vec::new();
                entry_reader
                    .read_to_end(&mut data)
                    .map_err(|e| sevenz_rust::Error::from(e))?;
                entries.push(ArchiveEntry {
                    path: Path::new(name).to_path_buf(),
                    data,
                    page_index: 0,
                });
            }
            Ok(true)
        })
        .map_err(|e| anyhow::anyhow!("Failed to read entries from CB7 archive: {}", e))?;

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_nonexistent_cb7() {
        let result = extract_cb7("nonexistent.cb7");
        assert!(result.is_err());
    }
}
