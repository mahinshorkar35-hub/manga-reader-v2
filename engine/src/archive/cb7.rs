//! CB7 (7z-based comic book archive) extraction.
//!
//! Uses the `sevenz-rust` crate to extract images from 7z archives.
//! CB7 is simply a 7z archive containing image files.

use anyhow::{Context, Result};
use sevenz_rust::SevenZReader;
use std::io::Read;
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

    let mut archive = SevenZReader::new(file, "".into())
        .context("Failed to read CB7 archive")?;

    let mut entries = Vec::new();

    // Get the list of entries in the archive
    let archive_entries = archive
        .entries()
        .context("Failed to list entries in CB7 archive")?;

    // Filter for image files
    let image_entries: Vec<_> = archive_entries
        .iter()
        .filter(|e| {
            !e.is_directory() && is_image_filename(
                e.name()
                    .unwrap_or("<unknown>")
            )
        })
        .collect();

    // Read each image entry
    for entry in &image_entries {
        let name = entry.name().context("Entry has no name")?.to_string();

        let mut data = Vec::new();
        archive
            .read_entry(name.as_str(), &mut data)
            .context(format!("Failed to read '{}' from CB7 archive", name))?;

        entries.push(ArchiveEntry {
            path: Path::new(&name).to_path_buf(),
            data,
            page_index: 0,
        });
    }

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
