//! CBR (RAR-based comic book archive) extraction.
//!
//! Uses the `unrar` crate to extract images from RAR archives.
//! CBR is simply a RAR archive containing image files.

use anyhow::{Context, Result};
use std::path::Path;
use unrar::Archive as UnrarArchive;

use super::{ArchiveEntry, is_image_filename};

/// Extract all image entries from a CBR (RAR) archive.
///
/// Iterates through all files in the RAR archive, filters for image
/// files by extension, and reads their contents into memory.
pub fn extract_cbr<P: AsRef<Path>>(path: P) -> Result<Vec<ArchiveEntry>> {
    let path = path.as_ref();

    // Open the RAR archive
    let archive = UnrarArchive::new(path)
        .open_for_listing()
        .context("Failed to open CBR archive")?;

    let mut entries = Vec::new();

    // List all entries in the archive
    for entry in archive {
        let entry = entry.context("Failed to read CBR entry")?;
        let file_name = entry.filename.to_string_lossy().to_string();

        // Skip directories and non-image files
        if entry.is_directory() || !is_image_filename(&file_name) {
            continue;
        }

        // Read the entry data by reopening with extraction
        let data = read_entry_data(path, &file_name)
            .context(format!("Failed to read {} from CBR", file_name))?;

        entries.push(ArchiveEntry {
            path: std::path::PathBuf::from(&file_name),
            data,
            page_index: 0,
        });
    }

    Ok(entries)
}

/// Read a specific entry's data from a RAR archive.
///
/// Opens the archive for processing and scans entries until the
/// requested one is found, reading its data into memory.
fn read_entry_data<P: AsRef<Path>>(archive_path: P, entry_name: &str) -> Result<Vec<u8>> {
    let archive_path = archive_path.as_ref();

    let mut archive = UnrarArchive::new(archive_path)
        .open_for_processing()
        .context("Failed to open CBR for extraction")?;

    loop {
        let header = archive
            .read_header()
            .context("Failed to read CBR header")?
            .ok_or_else(|| anyhow::anyhow!("Entry '{}' not found in CBR archive", entry_name))?;

        if header.entry().filename.to_string_lossy() == entry_name {
            let (data, rest) = header
                .read()
                .context("Failed to read entry data from CBR")?;
            drop(rest);
            return Ok(data);
        } else {
            archive = header
                .skip()
                .context("Failed to skip CBR entry")?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_nonexistent_cbr() {
        let result = extract_cbr("nonexistent.cbr");
        assert!(result.is_err());
    }
}
