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
        if entry.is_directory || !is_image_filename(&file_name) {
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
/// The `unrar` crate requires us to open the archive for extraction
/// and use a `Read` implementation to get the data.
fn read_entry_data<P: AsRef<Path>>(archive_path: P, entry_name: &str) -> Result<Vec<u8>> {
    use std::io::Read;

    let archive_path = archive_path.as_ref();

    // Open archive for extraction
    let mut archive = UnrarArchive::new(archive_path)
        .open_for_processing()
        .context("Failed to open CBR for extraction")?;

    // Process entries until we find the one we want
    loop {
        match archive {
            unrar::Processing::List(..) => {
                // We shouldn't get List here since we open_for_processing
                anyhow::bail!("Unexpected list state while extracting CBR");
            }
            unrar::Processing::Extract(extract) => {
                let entry_name_from_archive = extract
                    .entry()
                    .filename
                    .to_string_lossy()
                    .to_string();

                if entry_name_from_archive == entry_name {
                    // This is our entry — read it
                    let mut data = Vec::new();
                    extract
                        .read_to_end(&mut data)
                        .context("Failed to read entry data from CBR")?;
                    archive = extract.done().context("Failed to finalize CBR entry")?;
                    return Ok(data);
                } else {
                    // Skip this entry
                    archive = extract
                        .skip()
                        .context("Failed to skip CBR entry")?;
                }
            }
            unrar::Processing::Done => {
                anyhow::bail!("Entry '{}' not found in CBR archive", entry_name);
            }
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
