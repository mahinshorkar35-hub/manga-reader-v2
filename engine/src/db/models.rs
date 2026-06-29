//! Database model definitions for the manga reader engine.
//!
//! Defines the Rust structs that map to database tables,
//! with serialization support for IPC communication.

use rusqlite::Row;
use serde::{Deserialize, Serialize};

// ─── Manga ─────────────────────────────────────────────────────────

/// A manga series in the catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manga {
    pub id: i64,
    pub title: String,
    pub alt_title: Option<String>,
    pub author: Option<String>,
    pub artist: Option<String>,
    pub description: Option<String>,
    pub status: String,
    pub cover_path: Option<String>,
    pub source: Option<String>,
    pub source_url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Manga {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            title: row.get("title")?,
            alt_title: row.get("alt_title")?,
            author: row.get("author")?,
            artist: row.get("artist")?,
            description: row.get("description")?,
            status: row.get("status")?,
            cover_path: row.get("cover_path")?,
            source: row.get("source")?,
            source_url: row.get("source_url")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

/// Payload for creating a new manga entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewManga {
    pub title: String,
    pub alt_title: Option<String>,
    pub author: Option<String>,
    pub artist: Option<String>,
    pub description: Option<String>,
    pub status: String,
    pub cover_path: Option<String>,
    pub source: Option<String>,
    pub source_url: Option<String>,
}

// ─── Volume ─────────────────────────────────────────────────────────

/// A volume within a manga series.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Volume {
    pub id: i64,
    pub manga_id: i64,
    pub volume_number: f64,
    pub title: String,
    pub cover_path: Option<String>,
    pub created_at: String,
}

impl Volume {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            manga_id: row.get("manga_id")?,
            volume_number: row.get("volume_number")?,
            title: row.get("title")?,
            cover_path: row.get("cover_path")?,
            created_at: row.get("created_at")?,
        })
    }
}

/// Payload for creating a new volume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewVolume {
    pub manga_id: i64,
    pub volume_number: f64,
    pub title: String,
    pub cover_path: Option<String>,
}

// ─── Chapter ────────────────────────────────────────────────────────

/// A chapter within a volume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub id: i64,
    pub volume_id: i64,
    pub manga_id: i64,
    pub chapter_number: f64,
    pub title: Option<String>,
    pub page_count: i64,
    pub archive_path: Option<String>,
    pub created_at: String,
}

impl Chapter {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            volume_id: row.get("volume_id")?,
            manga_id: row.get("manga_id")?,
            chapter_number: row.get("chapter_number")?,
            title: row.get("title")?,
            page_count: row.get("page_count")?,
            archive_path: row.get("archive_path")?,
            created_at: row.get("created_at")?,
        })
    }
}

/// Payload for creating a new chapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewChapter {
    pub volume_id: i64,
    pub manga_id: i64,
    pub chapter_number: f64,
    pub title: String,
    pub page_count: i64,
    pub archive_path: String,
}

// ─── Page ───────────────────────────────────────────────────────────

/// A single page image within a chapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub id: i64,
    pub chapter_id: i64,
    pub page_number: i64,
    pub image_path: String,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub file_size: Option<i64>,
    pub created_at: String,
}

impl Page {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            chapter_id: row.get("chapter_id")?,
            page_number: row.get("page_number")?,
            image_path: row.get("image_path")?,
            width: row.get("width")?,
            height: row.get("height")?,
            file_size: row.get("file_size")?,
            created_at: row.get("created_at")?,
        })
    }
}

/// Payload for creating a new page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPage {
    pub chapter_id: i64,
    pub page_number: i64,
    pub image_path: String,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub file_size: Option<i64>,
}

// ─── Bookmark ───────────────────────────────────────────────────────

/// A user bookmark on a specific page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: i64,
    pub manga_id: i64,
    pub chapter_id: Option<i64>,
    pub page_number: i64,
    pub note: Option<String>,
    pub user_id: String,
    pub created_at: String,
}

impl Bookmark {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            manga_id: row.get("manga_id")?,
            chapter_id: row.get("chapter_id")?,
            page_number: row.get("page_number")?,
            note: row.get("note")?,
            user_id: row.get("user_id")?,
            created_at: row.get("created_at")?,
        })
    }
}

/// Payload for creating a new bookmark.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewBookmark {
    pub manga_id: i64,
    pub chapter_id: Option<i64>,
    pub page_number: i64,
    pub note: Option<String>,
    pub user_id: String,
}

// ─── Progress ───────────────────────────────────────────────────────

/// Reading progress for a user on a specific chapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progress {
    pub id: i64,
    pub user_id: String,
    pub manga_id: i64,
    pub chapter_id: i64,
    pub page_number: i64,
    pub percentage: f64,
    pub completed: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl Progress {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            manga_id: row.get("manga_id")?,
            chapter_id: row.get("chapter_id")?,
            page_number: row.get("page_number")?,
            percentage: row.get("percentage")?,
            completed: row.get("completed")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

/// Payload for upserting reading progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewProgress {
    pub user_id: String,
    pub manga_id: i64,
    pub chapter_id: i64,
    pub page_number: i64,
    pub percentage: f64,
    pub completed: bool,
}

// ─── Category ───────────────────────────────────────────────────────

/// A category/tag for organizing manga.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub created_at: String,
}

impl Category {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            color: row.get("color")?,
            created_at: row.get("created_at")?,
        })
    }
}

/// Payload for creating a new category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCategory {
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
}
