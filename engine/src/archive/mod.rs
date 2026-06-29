//! Archive extraction module.
//!
//! Supports multiple archive formats:
//! - **CBZ** (ZIP-compressed comic book archive)
//! - **CBR** (RAR-compressed comic book archive)
//! - **CB7** (7z-compressed comic book archive)
//! - **Plain folder** (directory of images)

pub mod cbz;
pub mod cbr;
pub mod cb7;

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Represents a single image entry within an archive.
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    /// The path of the file within the archive.
    pub path: PathBuf,
    /// The raw bytes of the image.
    pub data: Vec<u8>,
    /// The 0-based page index (after sorting).
    pub page_index: usize,
}

/// Supported archive types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveType {
    /// CBZ — ZIP archive
    Cbz,
    /// CBR — RAR archive
    Cbr,
    /// CB7 — 7z archive
    Cb7,
    /// Plain directory on disk
    Folder,
}

impl ArchiveType {
    /// Detect archive type from a file path or directory.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Option<Self> {
        let path = path.as_ref();
        if path.is_dir() {
            return Some(ArchiveType::Folder);
        }
        match path.extension()?.to_str()?.to_lowercase().as_str() {
            "cbz" | "zip" => Some(ArchiveType::Cbz),
            "cbr" | "rar" => Some(ArchiveType::Cbr),
            "cb7" | "7z" => Some(ArchiveType::Cb7),
            _ => None,
        }
    }
}

/// Extract all image entries from a given archive path.
///
/// Supported formats: CBZ, CBR, CB7, and plain folders.
/// Image files are filtered by common image extensions and sorted
/// alphabetically by their path within the archive.
pub fn extract<P: AsRef<Path>>(path: P) -> Result<Vec<ArchiveEntry>> {
    let path = path.as_ref();
    let archive_type =
        ArchiveType::from_path(path).context(format!("Unsupported archive type: {}", path.display()))?;

    log::info!(
        "Extracting {:?} archive: {}",
        archive_type,
        path.display()
    );

    let mut entries = match archive_type {
        ArchiveType::Cbz => cbz::extract_cbz(path)?,
        ArchiveType::Cbr => cbr::extract_cbr(path)?,
        ArchiveType::Cb7 => cb7::extract_cb7(path)?,
        ArchiveType::Folder => extract_folder(path)?,
    };

    // Sort entries by path for consistent page ordering
    entries.sort_by(|a, b| a.path.cmp(&b.path));

    // Assign page indices after sorting
    for (i, entry) in entries.iter_mut().enumerate() {
        entry.page_index = i;
    }

    log::info!("Extracted {} image entries from {}", entries.len(), path.display());
    Ok(entries)
}

/// Extract images from a plain directory on disk.
fn extract_folder<P: AsRef<Path>>(path: P) -> Result<Vec<ArchiveEntry>> {
    let path = path.as_ref();
    let mut entries = Vec::new();

    for entry in std::fs::read_dir(path).context("Failed to read directory")? {
        let entry = entry?;
        let entry_path = entry.path();

        if entry_path.is_file() && is_image_extension(&entry_path) {
            let data = std::fs::read(&entry_path)
                .context(format!("Failed to read image: {}", entry_path.display()))?;
            entries.push(ArchiveEntry {
                path: entry_path,
                data,
                page_index: 0, // Will be set after sorting
            });
        }
    }

    Ok(entries)
}

/// Check if a file path has a supported image extension.
pub fn is_image_extension<P: AsRef<Path>>(path: P) -> bool {
    match path.as_ref().extension().and_then(|e| e.to_str()) {
        Some(ext) => matches!(ext.to_lowercase().as_str(), "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "tiff" | "tif"),
        None => false,
    }
}

/// Check if a file name looks like an image (by extension).
pub fn is_image_filename(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".png")
        || lower.ends_with(".webp")
        || lower.ends_with(".gif")
        || lower.ends_with(".bmp")
        || lower.ends_with(".tiff")
        || lower.ends_with(".tif")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_archive_type() {
        assert_eq!(ArchiveType::from_path("test.cbz"), Some(ArchiveType::Cbz));
        assert_eq!(ArchiveType::from_path("test.cbr"), Some(ArchiveType::Cbr));
        assert_eq!(ArchiveType::from_path("test.cb7"), Some(ArchiveType::Cb7));
        assert_eq!(ArchiveType::from_path("test.zip"), Some(ArchiveType::Cbz));
        assert_eq!(ArchiveType::from_path("test.rar"), Some(ArchiveType::Cbr));
        assert_eq!(ArchiveType::from_path("test.7z"), Some(ArchiveType::Cb7));
        assert_eq!(ArchiveType::from_path("/some/dir"), Some(ArchiveType::Folder));
        assert_eq!(ArchiveType::from_path("test.txt"), None);
    }

    #[test]
    fn test_is_image_extension() {
        assert!(is_image_filename("page001.jpg"));
        assert!(is_image_filename("cover.png"));
        assert!(is_image_filename("scan.webp"));
        assert!(!is_image_filename("metadata.xml"));
        assert!(!is_image_filename("thumbnail.db"));
    }
}
